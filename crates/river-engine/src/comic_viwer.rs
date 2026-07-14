// Pure-Rust comic / manga reader.
//
// # Crate stack (zero FFI)
// - **`zip`** — CBZ archive extraction (pure Rust, deflate)
// - **`image`** — JPEG / PNG / WebP / GIF page decoding
// - **`reqwest`** — online manga page fetching (with `rustls-tls`)
//
// # Architecture
//One background task manages chapter/page state.  
// The UI receives the current page as raw JPEG bytes via a dedicated
// `watch::Receiver<Option<Arc<Vec<u8>>>>`, and reads navigation state from
// the main `PlayerHandle`.
//
// Page images are kept as compressed bytes (not decoded RGBA) to minimise
// memory use on low-end devices. The render layer decodes them into a
// texture exactly once and caches the result.
//
// # Supported sources
// - **CBZ / ZIP** archives (local path or pre-fetched bytes)
// - **Online manga pages** via URLs from a [`MangaPage`] list

#![forbid(unsafe_code)]

use std::io::{Cursor, Read};
use std::sync::Arc;


use tokio::sync::{mpsc, watch};
use tracing::{debug, error};

use river_core::{MangaChapter, MangaPage, MediaItem};

use crate::player_common::{PlayerHandle, PlayerStatus, ReadingDir};

// ─── Public state ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ComicState {
    pub status:        PlayerStatus,
    pub item_title:    String,
    /// Total number of chapters available.
    pub chapter_count: usize,
    /// Currently open chapter (0-based).
    pub chapter_index: usize,
    pub chapter_title: Option<String>,
    /// Total pages in the current chapter.
    pub page_count:    u32,
    /// Current page number (0-based).
    pub current_page:  u32,
    pub reading_dir:   ReadingDir,
    /// Zoom level: 1.0 = fit-width.
    pub zoom:          f32,
}

impl Default for ComicState {
    fn default() -> Self {
        Self {
            status:        PlayerStatus::Idle,
            item_title:    String::new(),
            chapter_count: 0,
            chapter_index: 0,
            chapter_title: None,
            page_count:    0,
            current_page:  0,
            reading_dir:   ReadingDir::LeftToRight,
            zoom:          1.0,
        }
    }
}

// ─── Commands ─────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum ComicCmd {
    /// Load a new comic / manga item with its chapter list.
    ///
    /// For online manga the pages are fetched lazily on `OpenChapter`.
    /// For CBZ, supply the archive bytes in [`ComicSource`] instead.
    Load {
        item:     MediaItem,
        chapters: Vec<MangaChapter>,
        source:   ComicSource,
    },
    /// Open a chapter by index and supply its page list
    /// (for online manga) or skip `pages` for a CBZ (pages are read from
    /// the archive automatically).
    OpenChapter {
        index: usize,
        pages: Vec<MangaPage>,
    },
    NextPage,
    PrevPage,
    JumpToPage(u32),
    NextChapter,
    PrevChapter,
    SetZoom(f32),
    SetReadingDir(ReadingDir),
}

/// Where the comic pages come from.
#[derive(Debug, Clone)]
pub enum ComicSource {
    /// CBZ / ZIP archive as raw bytes (e.g., already downloaded by the app).
    Cbz(Arc<Vec<u8>>),
    /// Online manga pages supplied chapter-by-chapter via `OpenChapter`.
    Online,
}

// ─── Player facade ────────────────────────────────────────────────────────────

pub struct ComicViewer;

impl ComicViewer {
    /// Spawn the background reader task.
    ///
    /// Returns:
    /// - `PlayerHandle` — navigation state + commands
    /// - `page_rx` — compressed page bytes (JPEG/PNG) ready to upload as a
    ///   texture; `None` until the first page loads
    pub fn spawn() -> (
        PlayerHandle<ComicCmd, ComicState>,
        watch::Receiver<Option<Arc<Vec<u8>>>>,
    ) {
        let (cmd_tx, cmd_rx) = mpsc::channel(64);
        let (state_tx, state_rx) = watch::channel(ComicState::default());
        let (page_tx, page_rx) = watch::channel(None);

        tokio::spawn(comic_task(cmd_rx, state_tx, page_tx));

        (PlayerHandle { cmd_tx, state_rx }, page_rx)
    }
}

// ─── Internal state ───────────────────────────────────────────────────────────

struct Ctx {
    chapters:    Vec<MangaChapter>,
    pages:       Vec<MangaPage>,         // pages for current chapter
    source:      ComicSource,
    cbz_entries: Vec<String>,            // sorted image filenames inside CBZ
    reading_dir: ReadingDir,
    zoom:        f32,
}

impl Default for Ctx {
    fn default() -> Self {
        Self {
            chapters:    Vec::new(),
            pages:       Vec::new(),
            source:      ComicSource::Online,
            cbz_entries: Vec::new(),
            reading_dir: ReadingDir::LeftToRight,
            zoom:        1.0,
        }
    }
}

// ─── Background task ──────────────────────────────────────────────────────────

async fn comic_task(
    mut cmd_rx: mpsc::Receiver<ComicCmd>,
    state_tx:   watch::Sender<ComicState>,
    page_tx:    watch::Sender<Option<Arc<Vec<u8>>>>,
) {
    let mut ctx = Ctx::default();

    loop {
        // No periodic tick needed — page loads happen on demand.
        match cmd_rx.recv().await {
            None => break,
            Some(cmd) => {
                handle_comic_cmd(cmd, &mut ctx, &state_tx, &page_tx).await;
            }
        }
    }
}

async fn handle_comic_cmd(
    cmd:      ComicCmd,
    ctx:      &mut Ctx,
    state_tx: &watch::Sender<ComicState>,
    page_tx:  &watch::Sender<Option<Arc<Vec<u8>>>>,
) {
    match cmd {
        ComicCmd::Load { item, chapters, source } => {
            ctx.chapters = chapters;
            ctx.source   = source.clone();
            ctx.pages    = Vec::new();
            ctx.cbz_entries = Vec::new();

            // For CBZ, index image entries once.
            if let ComicSource::Cbz(ref bytes) = source {
                ctx.cbz_entries = index_cbz_images(bytes);
                debug!("comic_viewer: CBZ has {} pages", ctx.cbz_entries.len());
            }

            let chapter_count = ctx.chapters.len();
            let _ = state_tx.send(ComicState {
                status:        PlayerStatus::Loading,
                item_title:    item.title.clone(),
                chapter_count,
                ..Default::default()
            });

            // Auto-open chapter 0.
            open_chapter(0, ctx, state_tx, page_tx).await;
        }

        ComicCmd::OpenChapter { index, pages } => {
            ctx.pages = pages;
            open_chapter(index, ctx, state_tx, page_tx).await;
        }

        ComicCmd::NextPage => {
            let page_count = current_page_count(ctx);
            let cur = state_tx.borrow().current_page;
            if cur + 1 < page_count {
                load_page(cur + 1, ctx, state_tx, page_tx).await;
            } else {
                // Reached last page — try next chapter.
                let ch_idx  = state_tx.borrow().chapter_index;
                let ch_count = state_tx.borrow().chapter_count;
                if ch_idx + 1 < ch_count {
                    let _ = state_tx.send_modify(|s| s.status = PlayerStatus::Loading);
                    open_chapter(ch_idx + 1, ctx, state_tx, page_tx).await;
                } else {
                    let _ = state_tx.send_modify(|s| s.status = PlayerStatus::Ended);
                }
            }
        }

        ComicCmd::PrevPage => {
            let cur = state_tx.borrow().current_page;
            if cur > 0 {
                load_page(cur - 1, ctx, state_tx, page_tx).await;
            } else {
                let ch_idx = state_tx.borrow().chapter_index;
                if ch_idx > 0 {
                    open_chapter(ch_idx - 1, ctx, state_tx, page_tx).await;
                }
            }
        }

        ComicCmd::JumpToPage(n) => {
            let page_count = current_page_count(ctx);
            if n < page_count {
                load_page(n, ctx, state_tx, page_tx).await;
            }
        }

        ComicCmd::NextChapter => {
            let ch_idx   = state_tx.borrow().chapter_index;
            let ch_count = state_tx.borrow().chapter_count;
            if ch_idx + 1 < ch_count {
                open_chapter(ch_idx + 1, ctx, state_tx, page_tx).await;
            }
        }

        ComicCmd::PrevChapter => {
            let ch_idx = state_tx.borrow().chapter_index;
            if ch_idx > 0 {
                open_chapter(ch_idx - 1, ctx, state_tx, page_tx).await;
            }
        }

        ComicCmd::SetZoom(z) => {
            ctx.zoom = z.clamp(0.5, 5.0);
            let zoom = ctx.zoom;
            let _ = state_tx.send_modify(|s| s.zoom = zoom);
        }

        ComicCmd::SetReadingDir(dir) => {
            ctx.reading_dir = dir;
            let _ = state_tx.send_modify(|s| s.reading_dir = dir);
        }
    }
}

/// Open (or switch to) a chapter and load its first page.
async fn open_chapter(
    index:    usize,
    ctx:      &mut Ctx,
    state_tx: &watch::Sender<ComicState>,
    page_tx:  &watch::Sender<Option<Arc<Vec<u8>>>>,
) {
    let page_count = current_page_count(ctx);
    let chapter_title = ctx
        .chapters
        .get(index)
        .and_then(|c| c.title.clone());

    let _ = state_tx.send_modify(|s| {
        s.chapter_index = index;
        s.chapter_title = chapter_title;
        s.page_count    = page_count;
        s.current_page  = 0;
        s.status        = PlayerStatus::Loading;
    });

    load_page(0, ctx, state_tx, page_tx).await;
}

/// Load page bytes for `page_num` and publish them.
async fn load_page(
    page_num: u32,
    ctx:      &Ctx,
    state_tx: &watch::Sender<ComicState>,
    page_tx:  &watch::Sender<Option<Arc<Vec<u8>>>>,
) {
    let bytes = match &ctx.source {
        ComicSource::Cbz(archive_bytes) => {
            load_cbz_page(archive_bytes, &ctx.cbz_entries, page_num as usize)
        }
        ComicSource::Online => {
            load_online_page(&ctx.pages, page_num as usize).await
        }
    };

    match bytes {
        Some(b) => {
            let _ = page_tx.send(Some(Arc::new(b)));
            let page_count = current_page_count(ctx);
            let _ = state_tx.send_modify(|s| {
                s.current_page = page_num;
                s.page_count   = page_count;
                s.status       = PlayerStatus::Playing;
            });
        }
        None => {
            error!("comic_viewer: failed to load page {page_num}");
            let _ = state_tx.send_modify(|s| {
                s.status = PlayerStatus::Error(format!("page {page_num} unavailable"));
            });
        }
    }
}

// ─── CBZ helpers ──────────────────────────────────────────────────────────────

/// List all image entries in a ZIP archive, sorted by name.
fn index_cbz_images(archive_bytes: &[u8]) -> Vec<String> {
    let cursor = Cursor::new(archive_bytes);
    let mut archive = match zip::ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(e) => { error!("comic_viewer: CBZ open error: {e}"); return Vec::new(); }
    };

    let mut names: Vec<String> = Vec::new();
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            let n = file.name().to_ascii_lowercase();
            if n.ends_with(".jpg")
                || n.ends_with(".jpeg")
                || n.ends_with(".png")
                || n.ends_with(".webp")
                || n.ends_with(".gif")
            {
                names.push(file.name().to_string());
            }
        }
    }

    names.sort();
    names
}

/// Extract raw image bytes for a specific page from a CBZ.
fn load_cbz_page(archive_bytes: &[u8], entries: &[String], page: usize) -> Option<Vec<u8>> {
    let name = entries.get(page)?;
    let cursor = Cursor::new(archive_bytes);
    let mut archive = zip::ZipArchive::new(cursor).ok()?;
    let mut file = archive.by_name(name).ok()?;
    let mut buf = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut buf).ok()?;
    Some(buf)
}

// ─── Online helpers ───────────────────────────────────────────────────────────

/// Fetch raw image bytes for a page from an online URL.
async fn load_online_page(pages: &[MangaPage], page: usize) -> Option<Vec<u8>> {
    let p = pages.get(page)?;
    debug!("comic_viewer: fetching page {} from {}", page, p.image_url);

    let mut req = reqwest::Client::new().get(&p.image_url);
    if let Some(headers) = &p.headers {
        for (k, v) in headers {
            req = req.header(k.as_str(), v.as_str());
        }
    }
    req.send()
        .await
        .ok()?
        .bytes()
        .await
        .ok()
        .map(|b| b.to_vec())
}

fn current_page_count(ctx: &Ctx) -> u32 {
    match &ctx.source {
        ComicSource::Cbz(_) => ctx.cbz_entries.len() as u32,
        ComicSource::Online => ctx.pages.len() as u32,
    }
}
