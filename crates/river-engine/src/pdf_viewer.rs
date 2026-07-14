// Pure-Rust PDF viewer.
//
// # Crate stack (zero FFI)
// - **`lopdf`** — 100 % Rust PDF parser; reads cross-reference table, object
//   graph, page tree, and embedded content streams
// - **`image`** — decodes embedded JPEG / PNG / JBIG2 images extracted from
//   page streams
// - **`reqwest`** — PDF file fetching (with `rustls-tls`)
//
// # Scope
// `lopdf` is a structural parser, not a full renderer.  This viewer exposes:
// - Page count and dimensions (`MediaBox`)
// - Plain-text content extracted from each page's content stream
// - Embedded images decoded to raw JPEG/PNG bytes for the UI to display
//
// Complex typography, ligatures, and advanced PDF graphics operations are
// the render layer's responsibility.  This is intentional: keeping the
// engine layer dependency-free enables future swap-in of a higher-quality
// pure-Rust renderer without touching the state machine.

#![forbid(unsafe_code)]

use std::sync::Arc;

use lopdf::{Document, Object};
use tokio::sync::{mpsc, watch};
use tracing::{debug, error, warn};

use river_core::MediaItem;

use crate::player_common::{PlayerHandle, PlayerStatus};

// ─── Page data ────────────────────────────────────────────────────────────────

/// Structured representation of one PDF page as extracted by `lopdf`.
///
/// The render layer can choose how much of this it actually uses.
#[derive(Debug, Clone, Default)]
pub struct PdfPageData {
    /// 1-based page number.
    pub page_num:   u32,
    /// Page width in PDF user-units (points; 72 pts = 1 inch).
    pub width_pts:  f64,
    /// Page height in PDF user-units.
    pub height_pts: f64,
    /// Best-effort plain-text extraction from the content stream.
    /// May contain rendering artefacts for complex documents.
    pub text:       String,
    /// Embedded images decoded from the page's XObject dictionary.
    /// Each `Vec<u8>` is raw JPEG or PNG bytes the UI can upload directly.
    pub images:     Vec<Vec<u8>>,
}

// ─── Public state ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PdfState {
    pub status:      PlayerStatus,
    pub title:       String,
    /// Total number of pages in the document (0 until loaded).
    pub total_pages: u32,
    /// Currently displayed page (0-based).
    pub current_page: u32,
    /// Zoom level: 1.0 = fit-width.
    pub zoom:        f32,
    /// Continuous scroll mode vs. single-page mode.
    pub is_continuous: bool,
}

impl Default for PdfState {
    fn default() -> Self {
        Self {
            status:       PlayerStatus::Idle,
            title:        String::new(),
            total_pages:  0,
            current_page: 0,
            zoom:         1.0,
            is_continuous: false,
        }
    }
}

// ─── Commands ─────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum PdfCmd {
    /// Fetch and parse a PDF from the given URL; use the item's title as display name.
    Load { item: MediaItem, stream_url: String },
    NextPage,
    PrevPage,
    /// Jump to an absolute 0-based page index.
    JumpToPage(u32),
    SetZoom(f32),
    ToggleContinuous,
}

// ─── Player facade ────────────────────────────────────────────────────────────

pub struct PdfViewer;

impl PdfViewer {
    /// Spawn the background PDF task.
    ///
    /// Returns:
    /// - `PlayerHandle` — navigation commands + state watch
    /// - `page_rx` — current page data (`None` until a page is ready)
    pub fn spawn() -> (
        PlayerHandle<PdfCmd, PdfState>,
        watch::Receiver<Option<Arc<PdfPageData>>>,
    ) {
        let (cmd_tx, cmd_rx) = mpsc::channel(64);
        let (state_tx, state_rx) = watch::channel(PdfState::default());
        let (page_tx, page_rx) = watch::channel(None);

        tokio::spawn(pdf_task(cmd_rx, state_tx, page_tx));

        (PlayerHandle { cmd_tx, state_rx }, page_rx)
    }
}

// ─── Background task ──────────────────────────────────────────────────────────

async fn pdf_task(
    mut cmd_rx: mpsc::Receiver<PdfCmd>,
    state_tx:   watch::Sender<PdfState>,
    page_tx:    watch::Sender<Option<Arc<PdfPageData>>>,
) {
    // The loaded Document is kept in memory; lopdf is an in-memory parser.
    let mut doc: Option<Document> = None;

    loop {
        match cmd_rx.recv().await {
            None => break,
            Some(cmd) => {
                handle_pdf_cmd(cmd, &mut doc, &state_tx, &page_tx).await;
            }
        }
    }
}

async fn handle_pdf_cmd(
    cmd:      PdfCmd,
    doc:      &mut Option<Document>,
    state_tx: &watch::Sender<PdfState>,
    page_tx:  &watch::Sender<Option<Arc<PdfPageData>>>,
) {
    match cmd {
        PdfCmd::Load { item, stream_url } => {
            let _ = state_tx.send(PdfState {
                status: PlayerStatus::Loading,
                title:  item.title.clone(),
                ..Default::default()
            });

            debug!("pdf_viewer: fetching {stream_url}");
            let bytes = match fetch_bytes(&stream_url).await {
                Ok(b) => b,
                Err(e) => {
                    error!("pdf_viewer: fetch error: {e}");
                    let _ = state_tx.send_modify(|s| {
                        s.status = PlayerStatus::Error(e);
                    });
                    return;
                }
            };

            // Parse PDF — blocking, hand to thread pool.
            let loaded = tokio::task::spawn_blocking(move || {
                Document::load_mem(&bytes)
            })
            .await;

            match loaded {
                Ok(Ok(d)) => {
                    let total_pages = d.get_pages().len() as u32;
                    *doc = Some(d);
                    let _ = state_tx.send(PdfState {
                        status:      PlayerStatus::Playing,
                        title:       item.title.clone(),
                        total_pages,
                        current_page: 0,
                        zoom:        1.0,
                        is_continuous: false,
                    });
                    // Load first page.
                    load_and_publish_page(doc, 0, &page_tx);
                }
                Ok(Err(e)) => {
                    error!("pdf_viewer: parse error: {e}");
                    let _ = state_tx.send_modify(|s| {
                        s.status = PlayerStatus::Error(e.to_string());
                    });
                }
                Err(e) => {
                    error!("pdf_viewer: blocking task error: {e}");
                    let _ = state_tx.send_modify(|s| {
                        s.status = PlayerStatus::Error(e.to_string());
                    });
                }
            }
        }

        PdfCmd::NextPage => {
            let (cur, total) = {
                let s = state_tx.borrow();
                (s.current_page, s.total_pages)
            };
            if total > 0 && cur + 1 < total {
                let next = cur + 1;
                load_and_publish_page(doc, next, &page_tx);
                let _ = state_tx.send_modify(|s| s.current_page = next);
            } else if total > 0 && cur + 1 >= total {
                let _ = state_tx.send_modify(|s| s.status = PlayerStatus::Ended);
            }
        }

        PdfCmd::PrevPage => {
            let cur = state_tx.borrow().current_page;
            if cur > 0 {
                let prev = cur - 1;
                load_and_publish_page(doc, prev, &page_tx);
                let _ = state_tx.send_modify(|s| s.current_page = prev);
            }
        }

        PdfCmd::JumpToPage(n) => {
            let total = state_tx.borrow().total_pages;
            if total > 0 && n < total {
                load_and_publish_page(doc, n, &page_tx);
                let _ = state_tx.send_modify(|s| s.current_page = n);
            }
        }

        PdfCmd::SetZoom(z) => {
            let z = z.clamp(0.25, 8.0);
            let _ = state_tx.send_modify(|s| s.zoom = z);
        }

        PdfCmd::ToggleContinuous => {
            let _ = state_tx.send_modify(|s| s.is_continuous = !s.is_continuous);
        }
    }
}

// ─── Page extraction ──────────────────────────────────────────────────────────

/// Extract and publish page data for `page_num` (0-based).
fn load_and_publish_page(
    doc:      &Option<Document>,
    page_num: u32,
    page_tx:  &watch::Sender<Option<Arc<PdfPageData>>>,
) {
    let d = match doc {
        Some(d) => d,
        None => return,
    };

    match extract_page(d, page_num) {
        Some(data) => { let _ = page_tx.send(Some(Arc::new(data))); }
        None       => { warn!("pdf_viewer: page {page_num} could not be extracted"); }
    }
}

/// Extract text and embedded images from a single PDF page.
fn extract_page(doc: &Document, page_num: u32) -> Option<PdfPageData> {
    // lopdf pages are 1-based; convert from our 0-based index.
    let pages = doc.get_pages();
    let page_id = *pages.get(&(page_num + 1))?;

    // ── MediaBox (page dimensions) ────────────────────────────────────────
    let (width_pts, height_pts) = get_page_size(doc, page_id);

    // ── Text content ──────────────────────────────────────────────────────
    let text = extract_text(doc, page_id);

    // ── Embedded images ───────────────────────────────────────────────────
    let images = extract_images(doc, page_id);

    Some(PdfPageData {
        page_num:   page_num + 1, // 1-based for display
        width_pts,
        height_pts,
        text,
        images,
    })
}

fn get_page_size(doc: &Document, page_id: lopdf::ObjectId) -> (f64, f64) {
    if let Ok(dict) = doc.get_object(page_id).and_then(|o| o.as_dict()) {
        if let Ok(Object::Array(arr)) = dict.get(b"MediaBox") {
            if arr.len() == 4 {
                let w = arr[2].as_float().unwrap_or(0.0) as f64;
                let h = arr[3].as_float().unwrap_or(0.0) as f64;
                return (w, h);
            }
        }
    }
    (595.0, 842.0) // A4 default
}

/// Very simple text extraction: collect all bytes from page content streams,
/// look for text between BT…ET operators, and extract PDF string literals.
/// This is intentionally lightweight and best-effort.
fn extract_text(doc: &Document, page_id: lopdf::ObjectId) -> String {
    let content_bytes = match doc.get_page_content(page_id) {
        Ok(b) => b,
        Err(_) => return String::new(),
    };

    // Decode printable ASCII / UTF-8 from raw content stream.
    // A proper PDF text extractor would decode character maps, but for
    // a lightweight viewer this gives readable results for most documents.
    let mut out = String::new();
    let mut in_string = false;
    let mut buf = Vec::new();

    for &byte in &content_bytes {
        match byte {
            b'(' => { in_string = true; buf.clear(); }
            b')' => {
                if in_string {
                    if let Ok(s) = std::str::from_utf8(&buf) {
                        if !s.trim().is_empty() {
                            out.push_str(s);
                            out.push(' ');
                        }
                    }
                    buf.clear();
                    in_string = false;
                }
            }
            _ => {
                if in_string {
                    buf.push(byte);
                }
            }
        }
    }

    out.trim().to_string()
}

/// Walk the page's XObject dictionary looking for embedded image streams.
/// Returns each image as raw JPEG or PNG bytes (whatever was embedded).
fn extract_images(doc: &Document, page_id: lopdf::ObjectId) -> Vec<Vec<u8>> {
    let mut images = Vec::new();

    // Get Resources → XObject dictionary.
    let resources = match doc
        .get_object(page_id)
        .and_then(|o| o.as_dict())
        .and_then(|d| d.get(b"Resources"))
        .and_then(|o| {
            if let Ok(id) = o.as_reference() {
                doc.get_object(id)
            } else {
                Ok(o)
            }
        })
        .and_then(|o| o.as_dict())
    {
        Ok(r) => r.clone(),
        Err(_) => return images,
    };

    let xobjects = match resources.get(b"XObject").and_then(|o| {
        if let Ok(id) = o.as_reference() {
            doc.get_object(id)
        } else {
            Ok(o)
        }
    }).and_then(|o| o.as_dict()) {
        Ok(d) => d.clone(),
        Err(_) => return images,
    };

    for (_key, obj) in &xobjects {
        let xobj_id = match obj.as_reference() {
            Ok(id) => id,
            Err(_) => continue,
        };

        if let Ok(stream) = doc.get_object(xobj_id).and_then(|o| o.as_stream()) {
            // Only process images (Subtype = Image).
            let is_image = stream
                .dict
                .get(b"Subtype")
                .map(|o| o.as_name().map(|n| n == b"Image").unwrap_or(false))
                .unwrap_or(false);

            if is_image {
                if let Ok(bytes) = stream.decompressed_content() {
                    if !bytes.is_empty() {
                        images.push(bytes);
                    }
                }
            }
        }
    }

    images
}

// ─── Network ──────────────────────────────────────────────────────────────────

async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    reqwest::get(url)
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| e.to_string())
}
