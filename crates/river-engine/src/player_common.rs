// Shared primitives for all River media player state machines.
//
// Every player follows the same zero-cost communication pattern:
//
// ```text
// UI thread                        background tokio task
//   PlayerHandle::send(cmd)  ──►  mpsc::Receiver<Cmd>
//   PlayerHandle::state()    ◄──  watch::Sender<State>  (latest snapshot)
//   extra_rx.borrow()        ◄──  watch::Sender<Media>  (current frame/page)
// ```
//
// The UI never awaits anything — all calls are fire-and-forget or zero-cost reads.

#![forbid(unsafe_code)]

use tokio::sync::{mpsc, watch};

// ─── Player lifecycle status ─────────────────────────────────────────────────

/// Lifecycle state common to every media player.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PlayerStatus {
    /// No media loaded; player is ready but inactive.
    #[default]
    Idle,
    /// Fetching or probing the source.
    Loading,
    /// Actively playing.
    Playing,
    /// Paused by the user.
    Paused,
    /// Filling the buffer; playback will resume automatically.
    Buffering,
    /// Reached the natural end of the content.
    Ended,
    /// Unrecoverable error — message describes what went wrong.
    Error(String),
}

impl PlayerStatus {
    /// `true` while media is loaded (playing, paused, or buffering).
    #[inline]
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Playing | Self::Paused | Self::Buffering)
    }

    /// `true` if the player has stopped and no media is held.
    #[inline]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Idle | Self::Ended | Self::Error(_))
    }
}

// ─── Repeat mode ─────────────────────────────────────────────────────────────

/// Queue repeat behaviour for queue-based players.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepeatMode {
    /// Stop when the queue is exhausted.
    #[default]
    Off,
    /// Repeat the current item indefinitely.
    One,
    /// Loop the whole queue.
    All,
}

// ─── Reading direction ───────────────────────────────────────────────────────

/// Page-turn direction for comic / manga readers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReadingDir {
    /// Western left-to-right.
    #[default]
    LeftToRight,
    /// Japanese / Korean right-to-left.
    RightToLeft,
    /// Vertical / webtoon scroll.
    Vertical,
}

// ─── Player handle ───────────────────────────────────────────────────────────

/// Held by the UI to command a background player and observe its state.
///
/// **All methods are non-blocking** — safe to call on the egui render thread
/// without ever awaiting or locking.
pub struct PlayerHandle<Cmd, State> {
    pub cmd_tx:   mpsc::Sender<Cmd>,
    pub state_rx: watch::Receiver<State>,
}

impl<Cmd, State> PlayerHandle<Cmd, State>
where
    Cmd:   Send + 'static,
    State: Clone + Send + Sync + 'static,
{
    /// Instant zero-cost snapshot of the latest published state.
    #[inline]
    pub fn state(&self) -> State {
        self.state_rx.borrow().clone()
    }

    /// Fire-and-forget command. Silently dropped if the player task has exited.
    pub fn send(&self, cmd: Cmd) {
        let tx = self.cmd_tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(cmd).await;
        });
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// `position / duration` clamped to `[0.0, 1.0]`.
#[inline]
pub fn progress_ratio(position_secs: f64, duration_secs: f64) -> f64 {
    if duration_secs <= 0.0 {
        0.0
    } else {
        (position_secs / duration_secs).clamp(0.0, 1.0)
    }
}

/// Advance a queue index by `delta` (+1 = forward, -1 = back) respecting
/// shuffle and repeat settings. Returns `None` when the queue is exhausted
/// and `RepeatMode::Off` is active.
pub fn advance_queue(
    current: usize,
    queue_len: usize,
    delta: i64,
    shuffle: bool,
    repeat: RepeatMode,
) -> Option<usize> {
    if queue_len == 0 {
        return None;
    }
    if delta == 0 {
        return Some(current);
    }

    if shuffle && delta > 0 {
        // Deterministic pseudo-random step — no `rand` crate dependency.
        let next = current
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407)
            % queue_len;
        return Some(next);
    }

    let next = if delta > 0 {
        current.saturating_add(1)
    } else {
        current.saturating_sub(1)
    };

    match repeat {
        RepeatMode::All => Some(next % queue_len),
        RepeatMode::One => Some(current),
        RepeatMode::Off => {
            if next < queue_len {
                Some(next)
            } else {
                None
            }
        }
    }
}
