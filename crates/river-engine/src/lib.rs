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

        Ok(Self { store })
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

        Ok(Self { store })
    }
}
