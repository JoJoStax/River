use async_trait::async_trait;
use river_core::{
    Catalog, MangaChapter, MangaPage, MediaCategory, MediaItem,
    MediaStream, PluginId, PluginMeta, ReadProgress, Result, Subtitle, WatchProgress,
};

#[async_trait]
pub trait CatalogPort: Send + Sync {
    async fn get_catalogs(&self, category: MediaCategory) -> Result<Vec<Catalog>>;
    async fn search(&self, query: &str) -> Result<Vec<MediaItem>>;
    async fn get_item_details(&self, plugin_id: &PluginId, item_id: &str) -> Result<MediaItem>;
    async fn get_streams(&self, plugin_id: &PluginId, item_id: &str) -> Result<Vec<MediaStream>>;
    async fn get_manga_chapters(&self, plugin_id: &PluginId, manga_id: &str) -> Result<Vec<MangaChapter>>;
    async fn get_manga_pages(&self, plugin_id: &PluginId, chapter_id: &str) -> Result<Vec<MangaPage>>;
    async fn get_subtitles(&self, plugin_id: &PluginId, item_id: &str) -> Result<Vec<Subtitle>>;
}

#[async_trait]
pub trait LibraryPort: Send + Sync {
    async fn get_library_items(&self, category: Option<MediaCategory>) -> Result<Vec<MediaItem>>;
    async fn add_to_library(&self, item: MediaItem) -> Result<()>;
    async fn remove_from_library(&self, plugin_id: &PluginId, item_id: &str) -> Result<()>;
    async fn is_in_library(&self, plugin_id: &PluginId, item_id: &str) -> Result<bool>;
    async fn save_watch_progress(&self, progress: WatchProgress) -> Result<()>;
    async fn get_watch_progress(&self, item_id: &str) -> Result<Option<WatchProgress>>;
    async fn save_read_progress(&self, progress: ReadProgress) -> Result<()>;
    async fn get_read_progress(&self, item_id: &str) -> Result<Option<ReadProgress>>;
}

#[async_trait]
pub trait PluginPort: Send + Sync {
    async fn list_plugins(&self) -> Vec<PluginMeta>;
    async fn get_plugin_meta(&self, id: &PluginId) -> Option<PluginMeta>;
    async fn enable_plugin(&self, id: &PluginId) -> Result<()>;
    async fn disable_plugin(&self, id: &PluginId) -> Result<()>;
    async fn is_plugin_enabled(&self, id: &PluginId) -> bool;
}
