// Pure-Rust video player with pluggable frame decoder.
// # Crate stack (zero FFI)
// - **Symphonia** — audio track decoding from MP4 / MKV direct streams
// - **Rodio / CPAL** — audio output (all platforms including Android)
// - **m3u8-rs** — HLS `.m3u8` playlist parsing
// - **reqwest** — segment / stream fetching (with `rustls-tls`, no OpenSSL)
//
// # Video frame decoding
// Video frame decode is intentionally separated behind the [`VideoDecoder`]
// trait so you can plug in the best available pure-Rust decoder for each
// platform (or swap in a hardware surface later):
//
// - [`NullVideoDecoder`] — audio-only mode; produces no frames
// - Future: a `SoftwareH264Decoder` wrapping `rust_h264` / `h264-reader`
//
// # Communication channels
// UI: PlayerHandle<VideoCmd, VideoState>  ← mpsc commands / watch state
// UI: frame_rx: watch::Receiver<Option<Arc<VideoFrame>>>  ← current RGBA frame

#![forbid(unsafe_code)]

use std::io::{BufReader, Cursor};
use std::sync::Arc;
use std::time::Duration;

use rodio::{Decoder, OutputStream, Sink};
use tokio::sync::{mpsc, watch};
use tracing::{debug, error, warn};

use river_core::{MediaItem, MediaStream, Subtitle};

use crate::player_common::{PlayerHandle, PlayerStatus};

// ─── Video frame ──────────────────────────────────────────────────────────────

/// A single decoded video frame in 32-bit RGBA format.
#[derive(Debug, Clone)]
pub struct VideoFrame {
    /// Raw RGBA pixel bytes — `width * height * 4` bytes.
    pub rgba:     Vec<u8>,
    pub width:    u32,
    pub height:   u32,
    /// Presentation timestamp in seconds.
    pub pts_secs: f64,
}

// ─── Pluggable decoder trait ──────────────────────────────────────────────────

/// A pure-Rust video frame decoder.  
///
/// The player calls [`push_data`] with raw encoded bytes (NAL units, MPEG-TS
/// payloads, etc.) and polls [`next_frame`] to retrieve decoded RGBA frames.
/// Implementors may buffer multiple frames internally.
pub trait VideoDecoder: Send + 'static {
    /// Feed encoded video data into the decoder.
    fn push_data(&mut self, data: &[u8]);

    /// Return the next decoded frame, if one is ready.
    fn next_frame(&mut self) -> Option<VideoFrame>;
}

/// No-op decoder — enables audio-only playback while the frame pipeline is
/// not wired up yet.
pub struct NullVideoDecoder;

impl VideoDecoder for NullVideoDecoder {
    fn push_data(&mut self, _data: &[u8]) {}
    fn next_frame(&mut self) -> Option<VideoFrame> { None }
}

// ─── Public state ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct VideoState {
    pub status:           PlayerStatus,
    pub title:            String,
    /// Current playback position in seconds.
    pub position_secs:    f64,
    /// Total duration in seconds (0 if unknown, e.g. live streams).
    pub duration_secs:    f64,
    /// Output volume 0.0 – 1.0.
    pub volume:           f32,
    pub is_muted:         bool,
    /// Active subtitle language code, e.g. `"en"`.
    pub selected_sub:     Option<String>,
    /// Available subtitle tracks.
    pub subtitles:        Vec<String>,
    /// HLS / network buffer fill 0.0 – 100.0.
    pub buffering_pct:    f32,
    /// Width of the video stream (0 until first frame is decoded).
    pub frame_width:      u32,
    pub frame_height:     u32,
    /// Index of the active quality rendition (0 = highest).
    pub active_rendition: usize,
}

impl Default for VideoState {
    fn default() -> Self {
        Self {
            status:           PlayerStatus::Idle,
            title:            String::new(),
            position_secs:    0.0,
            duration_secs:    0.0,
            volume:           1.0,
            is_muted:         false,
            selected_sub:     None,
            subtitles:        Vec::new(),
            buffering_pct:    0.0,
            frame_width:      0,
            frame_height:     0,
            active_rendition: 0,
        }
    }
}

// ─── Commands ─────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum VideoCmd {
    /// Load a new item with a set of candidate streams and optional subtitles.
    Load {
        item:      MediaItem,
        streams:   Vec<MediaStream>,
        subtitles: Vec<Subtitle>,
    },
    Play,
    Pause,
    Stop,
    /// Absolute seek in seconds.
    Seek(f64),
    SetVolume(f32),
    ToggleMute,
    /// Select subtitle by language code; `None` disables subtitles.
    SelectSubtitle(Option<String>),
    /// Switch to a different quality rendition by index.
    SelectStream(usize),
}

// ─── Player facade ────────────────────────────────────────────────────────────

pub struct VideoPlayer;

impl VideoPlayer {
    /// Spawn the background video task.
    ///
    /// Returns:
    /// - `PlayerHandle` — send commands and read state from the UI
    /// - `frame_rx` — current decoded [`VideoFrame`], ready to upload as an
    ///   egui texture; `None` until the first frame arrives
    pub fn spawn(
        decoder: Box<dyn VideoDecoder>,
    ) -> (
        PlayerHandle<VideoCmd, VideoState>,
        watch::Receiver<Option<Arc<VideoFrame>>>,
    ) {
        let (cmd_tx, cmd_rx) = mpsc::channel(64);
        let (state_tx, state_rx) = watch::channel(VideoState::default());
        let (frame_tx, frame_rx) = watch::channel(None);

        // OutputStream is !Send — run on a dedicated thread.
        let rt = tokio::runtime::Handle::current();
        std::thread::spawn(move || {
            rt.block_on(video_task(cmd_rx, state_tx, frame_tx, decoder));
        });

        (PlayerHandle { cmd_tx, state_rx }, frame_rx)
    }
}

// ─── Internal types ───────────────────────────────────────────────────────────

struct PlayerContext {
    queue:        Vec<MediaStream>,
    index:        usize,
    subtitles:    Vec<Subtitle>,
    selected_sub: Option<String>,
    volume:       f32,
    is_muted:     bool,
}

// ─── Background task ──────────────────────────────────────────────────────────

async fn video_task(
    mut cmd_rx: mpsc::Receiver<VideoCmd>,
    state_tx:   watch::Sender<VideoState>,
    frame_tx:   watch::Sender<Option<Arc<VideoFrame>>>,
    mut decoder: Box<dyn VideoDecoder>,
) {
    // Audio output — rodio for the audio track.
    let audio_ctx = OutputStream::try_default().ok();
    let sink: Option<Sink> = audio_ctx
        .as_ref()
        .and_then(|(_, h)| Sink::try_new(h).ok());

    let mut ctx = PlayerContext {
        queue:        Vec::new(),
        index:        0,
        subtitles:    Vec::new(),
        selected_sub: None,
        volume:       1.0,
        is_muted:     false,
    };

    loop {
        match tokio::time::timeout(Duration::from_millis(200), cmd_rx.recv()).await {
            Ok(None) => break,
            Ok(Some(cmd)) => {
                process_video_cmd(
                    cmd, &mut ctx, &sink, &state_tx, &frame_tx, &mut *decoder,
                )
                .await;
            }
            Err(_tick) => {
                // Update position.
                let pos = sink.as_ref().map(|s| s.get_pos().as_secs_f64()).unwrap_or(0.0);
                let is_playing = state_tx.borrow().status == PlayerStatus::Playing;
                if is_playing {
                    let _ = state_tx.send_modify(|s| s.position_secs = pos);
                }
            }
        }
    }
}

async fn process_video_cmd(
    cmd:      VideoCmd,
    ctx:      &mut PlayerContext,
    sink:     &Option<Sink>,
    state_tx: &watch::Sender<VideoState>,
    frame_tx: &watch::Sender<Option<Arc<VideoFrame>>>,
    _decoder:  &mut dyn VideoDecoder,
) {
    match cmd {
        VideoCmd::Load { item, streams, subtitles } => {
            ctx.queue     = streams;
            ctx.index     = 0;
            ctx.subtitles = subtitles.clone();

            let sub_langs: Vec<String> = subtitles.iter().map(|s| s.language.clone()).collect();
            let _ = state_tx.send(VideoState {
                status: PlayerStatus::Loading,
                title:  item.title.clone(),
                subtitles: sub_langs,
                volume:    ctx.volume,
                is_muted:  ctx.is_muted,
                ..Default::default()
            });

            // Try to load the best stream.
            if let Some(stream) = ctx.queue.get(ctx.index).cloned() {
                start_audio(&stream, sink, ctx.volume, ctx.is_muted).await;
            }

            let _ = state_tx.send_modify(|s| s.status = PlayerStatus::Playing);
        }

        VideoCmd::Play => {
            if let Some(s) = sink { s.play(); }
            let _ = state_tx.send_modify(|s| s.status = PlayerStatus::Playing);
        }

        VideoCmd::Pause => {
            if let Some(s) = sink { s.pause(); }
            let _ = state_tx.send_modify(|s| s.status = PlayerStatus::Paused);
        }

        VideoCmd::Stop => {
            if let Some(s) = sink { s.stop(); }
            let _ = frame_tx.send(None);
            let _ = state_tx.send(VideoState { volume: ctx.volume, ..Default::default() });
        }

        VideoCmd::Seek(secs) => {
            if let Some(s) = sink {
                let _ = s.try_seek(Duration::from_secs_f64(secs.max(0.0)));
            }
            let _ = state_tx.send_modify(|s| s.position_secs = secs.max(0.0));
        }

        VideoCmd::SetVolume(v) => {
            ctx.volume = v.clamp(0.0, 1.0);
            apply_volume(sink.as_ref(), ctx.volume, ctx.is_muted);
            let vol = ctx.volume;
            let _ = state_tx.send_modify(|s| s.volume = vol);
        }

        VideoCmd::ToggleMute => {
            ctx.is_muted = !ctx.is_muted;
            apply_volume(sink.as_ref(), ctx.volume, ctx.is_muted);
            let muted = ctx.is_muted;
            let _ = state_tx.send_modify(|s| s.is_muted = muted);
        }

        VideoCmd::SelectSubtitle(lang) => {
            ctx.selected_sub = lang.clone();
            let _ = state_tx.send_modify(|s| s.selected_sub = lang);
        }

        VideoCmd::SelectStream(idx) => {
            if idx < ctx.queue.len() {
                ctx.index = idx;
                if let Some(stream) = ctx.queue.get(idx).cloned() {
                    start_audio(&stream, sink, ctx.volume, ctx.is_muted).await;
                }
                let _ = state_tx.send_modify(|s| s.active_rendition = idx);
            }
        }
    }
}

/// Start audio playback for the given stream.
///
/// For direct streams (MP4, MKV, OGG, FLAC …) we buffer and decode with
/// Symphonia via Rodio. For HLS playlists we parse the manifest and fetch
/// the first segment.
async fn start_audio(stream: &MediaStream, sink: &Option<Sink>, volume: f32, muted: bool) {
    let url = stream.url.clone();
    let is_hls = url.contains(".m3u8") || stream.is_hls_or_dash;

    let audio_url = if is_hls {
        match resolve_hls_audio_url(&url).await {
            Some(u) => u,
            None => {
                warn!("video_player: could not resolve HLS audio stream for {url}");
                return;
            }
        }
    } else {
        url
    };

    debug!("video_player: loading audio from {audio_url}");
    let bytes = match fetch_bytes(&audio_url).await {
        Ok(b) => b,
        Err(e) => { error!("video_player: fetch error: {e}"); return; }
    };

    let source = match tokio::task::spawn_blocking(move || {
        Decoder::new(BufReader::new(Cursor::new(bytes)))
    })
    .await
    {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => { error!("video_player: decode error: {e}"); return; }
        Err(e)     => { error!("video_player: blocking task error: {e}"); return; }
    };

    if let Some(s) = sink {
        s.stop();
        apply_volume(Some(s), volume, muted);
        s.append(source);
        s.play();
    }
}

/// Parse an HLS master or media playlist and return the URL of the first
/// audio segment — used as a lightweight audio-only fallback while the video
/// pipeline is not yet active.
async fn resolve_hls_audio_url(playlist_url: &str) -> Option<String> {
    let bytes = fetch_bytes(playlist_url).await.ok()?;

    match m3u8_rs::parse_playlist_res(&bytes) {
        Ok(m3u8_rs::Playlist::MasterPlaylist(master)) => {
            // Pick the lowest-bandwidth audio-only rendition if available,
            // otherwise fall back to the first variant stream.
            let uri = master
                .alternatives
                .iter()
                .find(|a| a.media_type == m3u8_rs::AlternativeMediaType::Audio)
                .and_then(|a| a.uri.clone())
                .or_else(|| master.variants.first().map(|v| v.uri.clone()))?;

            // Resolve relative URIs.
            Some(resolve_relative(playlist_url, &uri))
        }
        Ok(m3u8_rs::Playlist::MediaPlaylist(media)) => {
            // Already a media playlist — return the first segment URI.
            let seg = media.segments.first()?;
            Some(resolve_relative(playlist_url, &seg.uri))
        }
        Err(e) => {
            error!("video_player: HLS parse error: {e:?}");
            None
        }
    }
}

fn resolve_relative(base: &str, path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") {
        return path.to_string();
    }
    // Strip the filename from base and append path.
    if let Some(slash) = base.rfind('/') {
        format!("{}/{}", &base[..slash], path)
    } else {
        path.to_string()
    }
}

fn apply_volume(sink: Option<&Sink>, volume: f32, muted: bool) {
    if let Some(s) = sink {
        s.set_volume(if muted { 0.0 } else { volume });
    }
}

async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    reqwest::get(url)
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| e.to_string())
}
