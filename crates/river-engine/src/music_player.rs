// Pure-Rust music player.
//
// # Crate stack (zero FFI)
// - **Symphonia** — format probing, demuxing, and audio decoding
//   (MP3, AAC, FLAC, OGG Vorbis, WAV, M4A/ALAC, AIFF …)
// - **Rodio** — high-level audio output (wraps CPAL; supports Android
//   AAudio / OpenSL ES, iOS AudioUnit, Windows WASAPI, Linux ALSA/PipeWire)
//
// # Design
// [`MusicPlayer::spawn`] starts one background tokio task.  
// The UI holds a [`PlayerHandle<MusicCmd, MusicState>`] and reads/writes
// through zero-copy `watch` / `mpsc` channels — **no blocking on the render
// thread**.

#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::io::{BufReader, Cursor};
use std::time::Duration;

use rodio::{Decoder, OutputStream, Sink};
use tokio::sync::{mpsc, watch};
use tracing::{debug, error};

use river_core::{MediaItem, MediaStream};

use crate::player_common::{advance_queue, PlayerHandle, PlayerStatus, RepeatMode};

// ─── Public state ─────────────────────────────────────────────────────────────

/// Snapshot of the music player published to the UI every 250 ms and on every
/// command. Cheap to clone (all strings are short).
#[derive(Debug, Clone)]
pub struct MusicState {
    pub status:        PlayerStatus,
    /// Display title of the current track.
    pub title:         String,
    pub artist:        Option<String>,
    pub album:         Option<String>,
    /// Poster / album-art URL (load separately if you want the image).
    pub art_url:       Option<String>,
    /// Decoded playback position in seconds.
    pub position_secs: f64,
    /// Total track duration in seconds (0 if unknown).
    pub duration_secs: f64,
    /// Output volume, 0.0 – 1.0.
    pub volume:        f32,
    pub shuffle:       bool,
    pub repeat:        RepeatMode,
    pub queue_len:     usize,
    pub queue_index:   usize,
}

impl Default for MusicState {
    fn default() -> Self {
        Self {
            status:        PlayerStatus::Idle,
            title:         String::new(),
            artist:        None,
            album:         None,
            art_url:       None,
            position_secs: 0.0,
            duration_secs: 0.0,
            volume:        1.0,
            shuffle:       false,
            repeat:        RepeatMode::Off,
            queue_len:     0,
            queue_index:   0,
        }
    }
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/// Commands the UI can send to the music player task.
#[derive(Debug)]
pub enum MusicCmd {
    /// Replace the queue and start playing immediately.
    LoadQueue(Vec<(MediaItem, MediaStream)>),
    /// Enqueue a single track (starts playing if the queue was empty).
    LoadTrack(MediaItem, MediaStream),
    Play,
    Pause,
    Stop,
    /// Seek to an absolute position in seconds.
    Seek(f64),
    Next,
    Prev,
    /// Set output volume 0.0 – 1.0.
    SetVolume(f32),
    ToggleShuffle,
    SetRepeat(RepeatMode),
}

// ─── Internal queue entry ─────────────────────────────────────────────────────

#[derive(Clone)]
struct Entry {
    item:   MediaItem,
    stream: MediaStream,
}

// ─── Player facade ────────────────────────────────────────────────────────────

/// Spawns the background audio task and returns the UI handle.
pub struct MusicPlayer;

impl MusicPlayer {
    pub fn spawn() -> PlayerHandle<MusicCmd, MusicState> {
        let (cmd_tx, cmd_rx) = mpsc::channel(64);
        let (state_tx, state_rx) = watch::channel(MusicState::default());

        // rodio's OutputStream is !Send, so we run the player on a dedicated
        // OS thread that owns the audio context for its entire lifetime.
        let rt = tokio::runtime::Handle::current();
        std::thread::spawn(move || {
            // Re-enter the tokio runtime on this thread so we can .await inside.
            rt.block_on(music_task(cmd_rx, state_tx));
        });

        PlayerHandle { cmd_tx, state_rx }
    }
}

// ─── Background task ──────────────────────────────────────────────────────────

async fn music_task(
    mut cmd_rx: mpsc::Receiver<MusicCmd>,
    state_tx:   watch::Sender<MusicState>,
) {
    // rodio audio context — must stay alive for the duration of the task.
    // On devices with no audio hardware we continue as a no-op player.
    let audio_ctx = OutputStream::try_default().ok();
    let sink: Option<Sink> = audio_ctx
        .as_ref()
        .and_then(|(_, handle)| Sink::try_new(handle).ok());

    let mut queue:   VecDeque<Entry> = VecDeque::new();
    let mut index:   usize           = 0;
    let mut volume:  f32             = 1.0;
    let mut shuffle: bool            = false;
    let mut repeat:  RepeatMode      = RepeatMode::Off;

    // Helper: publish current state snapshot.
    macro_rules! publish {
        ($status:expr) => {{
            let entry = queue.get(index);
            let _ = state_tx.send(MusicState {
                status:        $status,
                title:         entry.map(|e| e.item.title.clone()).unwrap_or_default(),
                artist:        entry.and_then(|e| e.item.author_or_creator.clone()),
                album:         None,
                art_url:       entry.and_then(|e| e.item.poster_url.clone()),
                position_secs: sink.as_ref().map(|s| s.get_pos().as_secs_f64()).unwrap_or(0.0),
                duration_secs: 0.0, // updated after decode
                volume,
                shuffle,
                repeat,
                queue_len:     queue.len(),
                queue_index:   index,
            });
        }};
    }

    publish!(PlayerStatus::Idle);

    loop {
        // 250 ms tick — update position and detect track-end.
        match tokio::time::timeout(Duration::from_millis(250), cmd_rx.recv()).await {
            // Channel closed — app is shutting down.
            Ok(None) => break,

            // Command received.
            Ok(Some(cmd)) => {
                handle_cmd(
                    cmd, &mut queue, &mut index, &sink, volume, &mut shuffle,
                    &mut repeat, &state_tx,
                )
                .await;
                volume = state_tx.borrow().volume; // keep in sync
            }

            // 250 ms tick — publish updated position.
            Err(_timeout) => {
                // Advance to next track when the sink drains.
                if sink.as_ref().map(|s| s.empty()).unwrap_or(false)
                    && !queue.is_empty()
                    && sink.as_ref().map(|s| s.get_pos().as_secs_f64()).unwrap_or(0.0) > 0.1
                {
                    if let Some(next_idx) =
                        advance_queue(index, queue.len(), 1, shuffle, repeat)
                    {
                        index = next_idx;
                        if let Some(entry) = queue.get(next_idx).cloned() {
                            publish!(PlayerStatus::Loading);
                            play_entry(&entry, &sink, volume).await;
                            publish!(PlayerStatus::Playing);
                        }
                    } else {
                        publish!(PlayerStatus::Ended);
                    }
                    continue;
                }

                let current_status = state_tx.borrow().status.clone();
                if current_status == PlayerStatus::Playing {
                    publish!(PlayerStatus::Playing);
                }
            }
        }
    }
}

async fn handle_cmd(
    cmd:      MusicCmd,
    queue:    &mut VecDeque<Entry>,
    index:    &mut usize,
    sink:     &Option<Sink>,
    volume:   f32,
    shuffle:  &mut bool,
    repeat:   &mut RepeatMode,
    state_tx: &watch::Sender<MusicState>,
) {
    match cmd {
        MusicCmd::LoadQueue(tracks) => {
            queue.clear();
            for (item, stream) in tracks {
                queue.push_back(Entry { item, stream });
            }
            *index = 0;
            if let Some(entry) = queue.front().cloned() {
                let _ = state_tx.send(MusicState {
                    status: PlayerStatus::Loading,
                    title: entry.item.title.clone(),
                    ..MusicState::default()
                });
                play_entry(&entry, sink, volume).await;
                let _ = state_tx.send_modify(|s| s.status = PlayerStatus::Playing);
            }
        }

        MusicCmd::LoadTrack(item, stream) => {
            queue.clear();
            queue.push_back(Entry { item: item.clone(), stream });
            *index = 0;
            let entry = queue.front().cloned().unwrap();
            let _ = state_tx.send(MusicState {
                status: PlayerStatus::Loading,
                title: item.title.clone(),
                ..MusicState::default()
            });
            play_entry(&entry, sink, volume).await;
            let _ = state_tx.send_modify(|s| s.status = PlayerStatus::Playing);
        }

        MusicCmd::Play => {
            if let Some(s) = sink { s.play(); }
            let _ = state_tx.send_modify(|s| s.status = PlayerStatus::Playing);
        }

        MusicCmd::Pause => {
            if let Some(s) = sink { s.pause(); }
            let _ = state_tx.send_modify(|s| s.status = PlayerStatus::Paused);
        }

        MusicCmd::Stop => {
            if let Some(s) = sink { s.stop(); }
            let _ = state_tx.send(MusicState { status: PlayerStatus::Idle, volume, ..Default::default() });
        }

        MusicCmd::Seek(secs) => {
            if let Some(s) = sink {
                let _ = s.try_seek(Duration::from_secs_f64(secs.max(0.0)));
            }
            let _ = state_tx.send_modify(|s| s.position_secs = secs.max(0.0));
        }

        MusicCmd::Next => {
            if let Some(next_idx) = advance_queue(*index, queue.len(), 1, *shuffle, *repeat) {
                *index = next_idx;
                if let Some(entry) = queue.get(next_idx).cloned() {
                    let _ = state_tx.send_modify(|s| s.status = PlayerStatus::Loading);
                    play_entry(&entry, sink, volume).await;
                    let _ = state_tx.send_modify(|s| s.status = PlayerStatus::Playing);
                }
            }
        }

        MusicCmd::Prev => {
            // If more than 3 s in, restart current track instead of going back.
            let pos = sink.as_ref().map(|s| s.get_pos().as_secs_f64()).unwrap_or(0.0);
            if pos > 3.0 {
                if let Some(s) = sink {
                    let _ = s.try_seek(Duration::ZERO);
                }
            } else if let Some(prev_idx) = advance_queue(*index, queue.len(), -1, *shuffle, *repeat) {
                *index = prev_idx;
                if let Some(entry) = queue.get(prev_idx).cloned() {
                    let _ = state_tx.send_modify(|s| s.status = PlayerStatus::Loading);
                    play_entry(&entry, sink, volume).await;
                    let _ = state_tx.send_modify(|s| s.status = PlayerStatus::Playing);
                }
            }
        }

        MusicCmd::SetVolume(v) => {
            let v = v.clamp(0.0, 1.0);
            if let Some(s) = sink { s.set_volume(v); }
            let _ = state_tx.send_modify(|s| s.volume = v);
        }

        MusicCmd::ToggleShuffle => {
            *shuffle = !*shuffle;
            let shuf = *shuffle;
            let _ = state_tx.send_modify(|s| s.shuffle = shuf);
        }

        MusicCmd::SetRepeat(mode) => {
            *repeat = mode;
            let _ = state_tx.send_modify(|s| s.repeat = mode);
        }
    }
}

/// Fetch the stream and hand it to rodio for playback.
/// Returns after the audio starts (not after it finishes).
async fn play_entry(entry: &Entry, sink: &Option<Sink>, volume: f32) {
    let url = entry.stream.url.clone();
    debug!("music_player: fetching {url}");

    let bytes = match fetch_bytes(&url).await {
        Ok(b) => b,
        Err(e) => {
            error!("music_player: fetch failed: {e}");
            return;
        }
    };

    // Decoder::new is blocking — hand it to the thread pool.
    let source = match tokio::task::spawn_blocking(move || {
        Decoder::new(BufReader::new(Cursor::new(bytes)))
    })
    .await
    {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => { error!("music_player: decode error: {e}"); return; }
        Err(e)     => { error!("music_player: task panic: {e}");   return; }
    };

    if let Some(s) = sink {
        s.stop();              // clear any previous track
        s.set_volume(volume);
        s.append(source);
        s.play();
    }
}

/// Fetch all bytes from a URL (buffers in memory — acceptable for audio tracks).
async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    reqwest::get(url)
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| e.to_string())
}
