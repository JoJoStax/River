//! FFmpeg video player.
//! Uses ffmpeg-next for decoding and rodio for audio.

#![allow(unsafe_code)]

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use std::any::Any;

use ffmpeg_next as ffmpeg;
use ffmpeg::format::Pixel;
use ffmpeg::media::Type as MediaType;
use ffmpeg::software::scaling::{context::Context as Scaler, flag::Flags as ScaleFlags};
use ffmpeg::software::resampling::context::Context as Resampler;
use ffmpeg::format::sample::{Sample, Type as SampleType};
use ffmpeg::util::channel_layout::ChannelLayout;

use rodio::{buffer::SamplesBuffer, OutputStream, Sink};
use tokio::sync::{mpsc, watch};
use tracing::{error, warn};

use river_core::{MediaItem, MediaStream, Subtitle};

use crate::player_common::{PlayerHandle, PlayerStatus};

// ─── Public output types ──────────────────────────────────────────────────────

/// A single decoded video frame in 32-bit RGBA format.
/// Identical layout to the previous pure-Rust version — the UI is unchanged.
#[derive(Debug, Clone)]
pub struct VideoFrame {
    /// Raw RGBA bytes: `width * height * 4`.
    pub rgba:     Vec<u8>,
    pub width:    u32,
    pub height:   u32,
    /// Presentation timestamp in seconds (stream time-base corrected).
    pub pts_secs: f64,
}

// ─── VideoDecoder trait (public, unchanged) ───────────────────────────────────

/// Pluggable video frame decoder — same trait as before so existing code
/// that calls `VideoPlayer::spawn(Box::new(NullVideoDecoder))` still compiles.
///
/// For real playback pass `Box::new(FfmpegVideoDecoder::open(url, headers)?)`.
pub trait VideoDecoder: Send + 'static {
    fn push_data(&mut self, data: &[u8]);
    fn next_frame(&mut self) -> Option<VideoFrame>;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub struct NullVideoDecoder;
impl VideoDecoder for NullVideoDecoder {
    fn push_data(&mut self, _: &[u8]) {}
    fn next_frame(&mut self) -> Option<VideoFrame> { None }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// ─── FfmpegVideoDecoder ───────────────────────────────────────────────────────

/// Full FFmpeg-backed decoder.
///
/// Open once with [`FfmpegVideoDecoder::open`], then call
/// [`decode_next`] in a tight loop to pump frames and PCM.
pub struct FfmpegVideoDecoder {
    ictx:         ffmpeg::format::context::Input,
    video_stream: usize,
    audio_stream: usize,
    video_dec:    ffmpeg::codec::decoder::Video,
    audio_dec:    ffmpeg::codec::decoder::Audio,
    scaler:       Scaler,
    resampler:    Resampler,
    frame_queue:  VecDeque<VideoFrame>,
    audio_queue:  VecDeque<Vec<f32>>,
    duration_sec: f64,
    width:        u32,
    height:       u32,
    current_pts:  f64,
}

unsafe impl Send for FfmpegVideoDecoder {}

const SAMPLE_RATE: u32 = 44_100;
const CHANNELS:   u16  = 2;

impl FfmpegVideoDecoder {
    /// Open any URL FFmpeg supports.
    ///
    /// `headers` is passed as AVOption `headers` (e.g. `Referer: …\r\n`).
    pub fn open(url: &str, headers: Option<&HashMap<String, String>>) -> Result<Self, String> {
        ffmpeg::init().map_err(|e| format!("ffmpeg init: {e}"))?;

        // Build AVDictionary options for the format context.
        let mut opts = ffmpeg::Dictionary::new();
        if let Some(h) = headers {
            let header_str: String = h
                .iter()
                .map(|(k, v)| format!("{k}: {v}\r\n"))
                .collect();
            opts.set("headers", &header_str);
        }
        // Allow HLS live streams.
        opts.set("live_start_index", "-1");
        // Generous probe to handle slow servers.
        opts.set("analyzeduration", "5000000");

        let ictx = ffmpeg::format::input_with_dictionary(&url, opts)
            .map_err(|e| format!("ffmpeg open '{url}': {e}"))?;

        // Locate best video and audio streams.
        let video_stream = ictx
            .streams()
            .best(MediaType::Video)
            .ok_or("no video stream")?
            .index();
        let audio_stream = ictx
            .streams()
            .best(MediaType::Audio)
            .ok_or("no audio stream")?
            .index();

        // Build video decoder.
        let v_stream = ictx.stream(video_stream).unwrap();
        let v_codec_ctx = ffmpeg::codec::context::Context::from_parameters(
            v_stream.parameters(),
        )
        .map_err(|e| format!("video codec ctx: {e}"))?;
        let video_dec = v_codec_ctx
            .decoder()
            .video()
            .map_err(|e| format!("video decoder: {e}"))?;

        let width  = video_dec.width();
        let height = video_dec.height();

        // Pixel-format converter: whatever FFmpeg produces → RGBA.
        let scaler = Scaler::get(
            video_dec.format(),
            width,
            height,
            Pixel::RGBA,
            width,
            height,
            ScaleFlags::BILINEAR,
        )
        .map_err(|e| format!("scaler: {e}"))?;

        // Build audio decoder.
        let a_stream = ictx.stream(audio_stream).unwrap();
        let a_codec_ctx = ffmpeg::codec::context::Context::from_parameters(
            a_stream.parameters(),
        )
        .map_err(|e| format!("audio codec ctx: {e}"))?;
        let audio_dec = a_codec_ctx
            .decoder()
            .audio()
            .map_err(|e| format!("audio decoder: {e}"))?;

        // PCM resampler: source format → packed f32 stereo at 44100 Hz.
        let resampler = ffmpeg::software::resampling::Context::get(
            audio_dec.format(),
            audio_dec.channel_layout(),
            audio_dec.rate(),
            Sample::F32(SampleType::Packed),
            ChannelLayout::STEREO,
            SAMPLE_RATE,
        )
        .map_err(|e| format!("resampler: {e}"))?;

        // Duration from container (0 for live streams).
        let duration_sec = if ictx.duration() > 0 {
            ictx.duration() as f64 / f64::from(ffmpeg::ffi::AV_TIME_BASE)
        } else {
            0.0
        };

        Ok(Self {
            ictx,
            video_stream,
            audio_stream,
            video_dec,
            audio_dec,
            scaler,
            resampler,
            frame_queue: VecDeque::new(),
            audio_queue: VecDeque::new(),
            duration_sec,
            width,
            height,
            current_pts: 0.0,
        })
    }

    /// Decode one packet from the container.
    ///
    /// Returns `(video_frame_ready, audio_chunk_ready)` so callers know
    /// when to drain. Returns `Err` on EOF or unrecoverable error.
    pub fn decode_next(&mut self) -> Result<(bool, bool), String> {
        let mut video_ready = false;
        let mut audio_ready = false;

        match self.ictx.packets().next() {
            None => return Err("EOF".to_string()),
            Some((stream, packet)) => {
                let si = stream.index();

                if si == self.video_stream {
                    self.video_dec
                        .send_packet(&packet)
                        .map_err(|e| format!("send video packet: {e}"))?;

                    let mut raw = ffmpeg::util::frame::video::Video::empty();
                    while self.video_dec.receive_frame(&mut raw).is_ok() {
                        let mut rgba = ffmpeg::util::frame::video::Video::empty();
                        self.scaler
                            .run(&raw, &mut rgba)
                            .map_err(|e| format!("scaler run: {e}"))?;

                        // pts in stream time-base → seconds.
                        let tb  = stream.time_base();
                        let pts = raw.pts().unwrap_or(0) as f64
                            * f64::from(tb.numerator())
                            / f64::from(tb.denominator());
                        self.current_pts = pts;

                        self.frame_queue.push_back(VideoFrame {
                            rgba:     rgba.data(0).to_vec(),
                            width:    self.width,
                            height:   self.height,
                            pts_secs: pts,
                        });
                        video_ready = true;
                    }
                } else if si == self.audio_stream {
                    self.audio_dec
                        .send_packet(&packet)
                        .map_err(|e| format!("send audio packet: {e}"))?;

                    let mut raw = ffmpeg::util::frame::audio::Audio::empty();
                    while self.audio_dec.receive_frame(&mut raw).is_ok() {
                        let mut out = ffmpeg::util::frame::audio::Audio::empty();
                        let delay = self
                            .resampler
                            .run(&raw, &mut out)
                            .map_err(|e| format!("resampler run: {e}"))?;

                        let samples = out.data(0);
                        // Interpret raw bytes as packed f32 LE.
                        let floats: Vec<f32> = samples
                            .chunks_exact(4)
                            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                            .collect();

                        if !floats.is_empty() {
                            self.audio_queue.push_back(floats);
                            audio_ready = true;
                        }

                        // Flush resampler tail.
                        if delay.is_some() {
                            let mut flush = ffmpeg::util::frame::audio::Audio::empty();
                            while self.resampler.flush(&mut flush).is_ok() {
                                let s = flush.data(0);
                                let floats: Vec<f32> = s
                                    .chunks_exact(4)
                                    .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                                    .collect();
                                if !floats.is_empty() {
                                    self.audio_queue.push_back(floats);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((video_ready, audio_ready))
    }

    /// Drain all buffered PCM chunks since the last call.
    pub fn drain_audio(&mut self) -> Vec<Vec<f32>> {
        self.audio_queue.drain(..).collect()
    }

    /// Frame-accurate seek via libavformat.
    pub fn seek(&mut self, secs: f64) -> Result<(), String> {
        let ts = (secs * f64::from(ffmpeg::ffi::AV_TIME_BASE)) as i64;
        unsafe {
            // SEEK_FLAG_BACKWARD ensures we land on a keyframe at or before ts.
            let ret = ffmpeg::ffi::av_seek_frame(
                self.ictx.as_mut_ptr(),
                -1, // any stream
                ts,
                ffmpeg::ffi::AVSEEK_FLAG_BACKWARD as i32,
            );
            if ret < 0 {
                return Err(format!("seek failed: {ret}"));
            }
        }
        // Flush decoder buffers so stale frames don't appear after seek.
        self.video_dec.flush();
        self.audio_dec.flush();
        self.frame_queue.clear();
        self.audio_queue.clear();
        self.current_pts = secs;
        Ok(())
    }

    /// Total duration from the container header (0.0 for live streams).
    pub fn duration_secs(&self) -> f64     { self.duration_sec }
    /// Decoded frame dimensions.
    pub fn dimensions(&self)     -> (u32, u32) { (self.width, self.height) }
    /// PTS of the most recently decoded video frame, in seconds.
    pub fn current_pts(&self)    -> f64     { self.current_pts }

    /// All subtitle streams: `(stream_index, language_code)`.
    pub fn subtitle_tracks(&self) -> Vec<(usize, String)> {
        self.ictx
            .streams()
            .filter(|s| s.parameters().medium() == MediaType::Subtitle)
            .map(|s| {
                let lang = s
                    .metadata()
                    .get("language")
                    .unwrap_or("und")
                    .to_string();
                (s.index(), lang)
            })
            .collect()
    }
}

impl VideoDecoder for FfmpegVideoDecoder {
    fn push_data(&mut self, _: &[u8]) {}
    fn next_frame(&mut self) -> Option<VideoFrame> { self.frame_queue.pop_front() }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// ─── Public state (unchanged from before) ────────────────────────────────────

#[derive(Debug, Clone)]
pub struct VideoState {
    pub status:           PlayerStatus,
    pub title:            String,
    pub position_secs:    f64,
    pub duration_secs:    f64,
    pub volume:           f32,
    pub is_muted:         bool,
    pub selected_sub:     Option<String>,
    pub subtitles:        Vec<String>,
    pub buffering_pct:    f32,
    pub frame_width:      u32,
    pub frame_height:     u32,
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

// ─── Commands (unchanged) ─────────────────────────────────────────────────────

#[derive(Debug)]
pub enum VideoCmd {
    Load {
        item:      MediaItem,
        streams:   Vec<MediaStream>,
        subtitles: Vec<Subtitle>,
    },
    Play,
    Pause,
    Stop,
    Seek(f64),
    SetVolume(f32),
    ToggleMute,
    SelectSubtitle(Option<String>),
    SelectStream(usize),
}

// ─── Player facade (unchanged public API) ────────────────────────────────────

pub struct VideoPlayer;

impl VideoPlayer {
    /// Spawn the background video task.
    ///
    /// - Pass `Box::new(FfmpegVideoDecoder::open(url, headers)?)` for real playback.
    /// - Pass `Box::new(NullVideoDecoder)` for audio-only / testing.
    pub fn spawn(
        decoder: Box<dyn VideoDecoder>,
    ) -> (
        PlayerHandle<VideoCmd, VideoState>,
        watch::Receiver<Option<Arc<VideoFrame>>>,
    ) {
        let (cmd_tx, cmd_rx) = mpsc::channel(64);
        let (state_tx, state_rx) = watch::channel(VideoState::default());
        let (frame_tx, frame_rx) = watch::channel(None);

        // OutputStream is !Send — run everything on a dedicated OS thread.
        let rt = tokio::runtime::Handle::current();
        std::thread::spawn(move || {
            rt.block_on(video_task(cmd_rx, state_tx, frame_tx, decoder));
        });

        (PlayerHandle { cmd_tx, state_rx }, frame_rx)
    }
}

// ─── Internal context ────────── yeepeeeeee ───────────────────────────────────

struct Ctx {
    streams:      Vec<MediaStream>,
    index:        usize,
    volume:       f32,
    is_muted:     bool,
    selected_sub: Option<String>,
    /// The live FFmpeg decoder — `None` when idle.
    decoder:      Option<Box<dyn VideoDecoder>>,
}

// ─── Background task ──────────────────────────────────────────────────────────

async fn video_task(
    mut cmd_rx:   mpsc::Receiver<VideoCmd>,
    state_tx:     watch::Sender<VideoState>,
    frame_tx:     watch::Sender<Option<Arc<VideoFrame>>>,
    init_decoder: Box<dyn VideoDecoder>,
) {
    // Audio output — stays alive for the entire task lifetime.
    let audio_ctx = OutputStream::try_default().ok();
    let sink: Option<Sink> = audio_ctx
        .as_ref()
        .and_then(|(_, h)| Sink::try_new(h).ok());

    let mut ctx = Ctx {
        streams:      Vec::new(),
        index:        0,
        volume:       1.0,
        is_muted:     false,
        selected_sub: None,
        decoder:      Some(init_decoder),
    };

    loop {
        // 1 ms decode tick to keep AV in sync; commands are checked each lap.
        match tokio::time::timeout(Duration::from_millis(1), cmd_rx.recv()).await {
            Ok(None) => break,

            Ok(Some(cmd)) => {
                handle_cmd(cmd, &mut ctx, &sink, &state_tx, &frame_tx).await;
            }

            // Decode tick — pump the FFmpeg pipeline.
            Err(_) => {
                let is_playing = state_tx.borrow().status == PlayerStatus::Playing;
                if !is_playing { continue; }

                let dec = match ctx.decoder.as_mut() {
                    Some(d) => d,
                    None    => continue,
                };

                // Pump one packet. EOF or error → mark Ended.
                match pump_once(dec.as_mut(), &sink, &frame_tx) {
                    Ok((_, _)) => {
                        // Update position and dimensions from the decoder.
                        let (pos, w, h) = if let Some(ffm) = dec.as_any_mut().downcast_mut::<FfmpegVideoDecoder>() {
                            (ffm.current_pts(), ffm.width, ffm.height)
                        } else {
                            (0.0, 0, 0)
                        };
                        let _ = state_tx.send_modify(|s| {
                            s.position_secs = pos;
                            if w > 0 { s.frame_width  = w; }
                            if h > 0 { s.frame_height = h; }
                        });
                    }
                    Err(e) if e == "EOF" => {
                        let _ = state_tx.send_modify(|s| s.status = PlayerStatus::Ended);
                    }
                    Err(e) => {
                        error!("video_player: decode error: {e}");
                        let _ = state_tx.send_modify(|s| {
                            s.status = PlayerStatus::Error(e);
                        });
                    }
                }
            }
        }
    }
}

/// Decode one packet and push audio/video to their channels.
fn pump_once(
    dec:      &mut dyn VideoDecoder,
    sink:     &Option<Sink>,
    frame_tx: &watch::Sender<Option<Arc<VideoFrame>>>,
) -> Result<(bool, bool), String> {
    let ffm = match dec.as_any_mut().downcast_mut::<FfmpegVideoDecoder>() {
        Some(f) => f,
        None => {
            let _ = dec.next_frame();
            return Ok((false, false));
        }
    };

    let (vr, ar) = ffm.decode_next()?;

    // Drain and play audio.
    if ar {
        if let Some(s) = sink {
            for chunk in ffm.drain_audio() {
                s.append(SamplesBuffer::new(CHANNELS, SAMPLE_RATE, chunk));
            }
        }
    }

    // Publish latest video frame.
    if vr {
        if let Some(frame) = ffm.next_frame() {
            let _ = frame_tx.send(Some(Arc::new(frame)));
        }
    }

    Ok((vr, ar))
}

// ─── Command handler ──────────────────────────────────────────────────────────

async fn handle_cmd(
    cmd:      VideoCmd,
    ctx:      &mut Ctx,
    sink:     &Option<Sink>,
    state_tx: &watch::Sender<VideoState>,
    frame_tx: &watch::Sender<Option<Arc<VideoFrame>>>,
) {
    match cmd {
        VideoCmd::Load { item, streams, subtitles: _ } => {
            ctx.streams = streams;
            ctx.index   = 0;

            let _ = state_tx.send(VideoState {
                status:   PlayerStatus::Loading,
                title:    item.title.clone(),
                volume:   ctx.volume,
                is_muted: ctx.is_muted,
                ..Default::default()
            });

            // Open FFmpeg on a blocking thread (I/O probe).
            let stream = match ctx.streams.get(ctx.index).cloned() {
                Some(s) => s,
                None    => {
                    let _ = state_tx.send_modify(|s| {
                        s.status = PlayerStatus::Error("no streams".to_string());
                    });
                    return;
                }
            };

            let url     = stream.url.clone();
            let headers = stream.headers.clone();

            match tokio::task::spawn_blocking(move || {
                FfmpegVideoDecoder::open(&url, headers.as_ref())
            })
            .await
            {
                Ok(Ok(ffm)) => {
                    let dur  = ffm.duration_secs();
                    let (w, h) = ffm.dimensions();
                    let subs: Vec<String> = ffm
                        .subtitle_tracks()
                        .into_iter()
                        .map(|(_, lang)| lang)
                        .collect();

                    ctx.decoder = Some(Box::new(ffm));
                    if let Some(s) = sink { s.stop(); apply_volume(Some(s), ctx.volume, ctx.is_muted); }

                    let _ = state_tx.send(VideoState {
                        status:        PlayerStatus::Playing,
                        title:         item.title,
                        duration_secs: dur,
                        frame_width:   w,
                        frame_height:  h,
                        subtitles:     subs,
                        volume:        ctx.volume,
                        is_muted:      ctx.is_muted,
                        ..Default::default()
                    });
                }
                Ok(Err(e)) => {
                    error!("video_player: FFmpeg open failed: {e}");
                    let _ = state_tx.send_modify(|s| s.status = PlayerStatus::Error(e));
                }
                Err(e) => {
                    error!("video_player: spawn_blocking panic: {e}");
                    let _ = state_tx.send_modify(|s| {
                        s.status = PlayerStatus::Error(e.to_string());
                    });
                }
            }
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
            ctx.decoder = None;
            let _ = frame_tx.send(None);
            let _ = state_tx.send(VideoState {
                volume: ctx.volume, is_muted: ctx.is_muted, ..Default::default()
            });
        }

        VideoCmd::Seek(secs) => {
            if let Some(ffm) = ctx
                .decoder
                .as_mut()
                .and_then(|d| d.as_any_mut().downcast_mut::<FfmpegVideoDecoder>())
            {
                if let Err(e) = ffm.seek(secs) {
                    warn!("video_player: seek error: {e}");
                }
            }
            if let Some(s) = sink { s.stop(); apply_volume(Some(s), ctx.volume, ctx.is_muted); }
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
            if idx < ctx.streams.len() {
                ctx.index = idx;
                let stream = ctx.streams[idx].clone();
                let url    = stream.url.clone();
                let headers = stream.headers.clone();

                match tokio::task::spawn_blocking(move || {
                    FfmpegVideoDecoder::open(&url, headers.as_ref())
                })
                .await
                {
                    Ok(Ok(ffm)) => {
                        ctx.decoder = Some(Box::new(ffm));
                        if let Some(s) = sink { s.stop(); apply_volume(Some(s), ctx.volume, ctx.is_muted); }
                        let _ = state_tx.send_modify(|s| s.active_rendition = idx);
                    }
                    Ok(Err(e)) => {
                        error!("video_player: stream switch failed: {e}");
                    }
                    Err(e) => {
                        error!("video_player: stream switch spawn_blocking failed: {e}");
                    }
                }
            }
        }
    }
}

// ─── Volume helper ────────────────────────────────────────────────────────────

fn apply_volume(sink: Option<&Sink>, volume: f32, muted: bool) {
    if let Some(s) = sink {
        s.set_volume(if muted { 0.0 } else { volume });
    }
}
