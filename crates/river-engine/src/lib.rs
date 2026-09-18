// ─── Media player modules (100 % pure Rust) ──────────────────────────────────
pub mod player_common;
pub mod music_player;
pub mod video_viewer;
pub mod comic_viwer;
pub mod pdf_viewer;

// Public re-exports so callers don't need deep paths.
pub use player_common::{
    PlayerHandle, PlayerStatus, RepeatMode, ReadingDir, progress_ratio, advance_queue,
};
pub use music_player::{MusicPlayer, MusicState, MusicCmd};
pub use video_viewer::{
    VideoPlayer, VideoState, VideoCmd, VideoFrame,
    VideoDecoder, NullVideoDecoder, FfmpegVideoDecoder,
};
pub use comic_viwer::{ComicViewer, ComicState, ComicCmd, ComicSource};
pub use pdf_viewer::{PdfViewer, PdfState, PdfCmd, PdfPageData};

// ─── Engine composition root ─────────────────────────────────────────────────
use river_core::Result;
use river_network::ReqwestClient;
use river_ports::StorageRepository;
use river_presentation::AppStore;
use river_services::{CatalogService, LibraryService, PluginService};
use river_storage::SqliteStorage;
use std::sync::Arc;

pub struct RiverEngine {
    pub store: Arc<AppStore>,
    pub music_player: Arc<PlayerHandle<MusicCmd, MusicState>>,
    pub comic_viewer: Arc<PlayerHandle<ComicCmd, ComicState>>,
    pub comic_page_rx: tokio::sync::watch::Receiver<Option<Arc<Vec<u8>>>>,
    pub pdf_viewer: Arc<PlayerHandle<PdfCmd, PdfState>>,
    pub pdf_page_rx: tokio::sync::watch::Receiver<Option<Arc<PdfPageData>>>,
    pub video_player: Arc<std::sync::Mutex<(PlayerHandle<VideoCmd, VideoState>, tokio::sync::watch::Receiver<Option<Arc<VideoFrame>>>)>>,
}

impl RiverEngine {
    pub async fn new_in_memory() -> Result<Self> {
        let storage = Arc::new(SqliteStorage::new_in_memory()?);
        storage.init_schema().await?;

        let _network_client = Arc::new(ReqwestClient::new());
        let plugin_service = Arc::new(PluginService::new(storage.clone()));

        let catalog_service = Arc::new(CatalogService::new(plugin_service.clone()));
        let library_service = Arc::new(LibraryService::new(storage.clone()));

        let store = Arc::new(AppStore::new(
            catalog_service,
            library_service,
            plugin_service,
        ));

        let music_player = Arc::new(MusicPlayer::spawn());
        let (comic_viewer, comic_page_rx) = ComicViewer::spawn();
        let (pdf_viewer, pdf_page_rx) = PdfViewer::spawn();
        let (video_handle, video_frame_rx) = VideoPlayer::spawn(Box::new(NullVideoDecoder));

        Ok(Self {
            store,
            music_player,
            comic_viewer: Arc::new(comic_viewer),
            comic_page_rx,
            pdf_viewer: Arc::new(pdf_viewer),
            pdf_page_rx,
            video_player: Arc::new(std::sync::Mutex::new((video_handle, video_frame_rx))),
        })
    }

    pub async fn new_with_db_path(path: &str) -> Result<Self> {
        let storage = Arc::new(SqliteStorage::new_from_path(path)?);
        storage.init_schema().await?;

        let _network_client = Arc::new(ReqwestClient::new());
        let plugin_service = Arc::new(PluginService::new(storage.clone()));

        let catalog_service = Arc::new(CatalogService::new(plugin_service.clone()));
        let library_service = Arc::new(LibraryService::new(storage.clone()));

        let store = Arc::new(AppStore::new(
            catalog_service,
            library_service,
            plugin_service,
        ));

        let music_player = Arc::new(MusicPlayer::spawn());
        let (comic_viewer, comic_page_rx) = ComicViewer::spawn();
        let (pdf_viewer, pdf_page_rx) = PdfViewer::spawn();
        let (video_handle, video_frame_rx) = VideoPlayer::spawn(Box::new(NullVideoDecoder));

        Ok(Self {
            store,
            music_player,
            comic_viewer: Arc::new(comic_viewer),
            comic_page_rx,
            pdf_viewer: Arc::new(pdf_viewer),
            pdf_page_rx,
            video_player: Arc::new(std::sync::Mutex::new((video_handle, video_frame_rx))),
        })
    }
}
