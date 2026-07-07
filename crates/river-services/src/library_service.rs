use async_trait::async_trait;
use river_core::{
    MediaCategory, MediaItem, PluginId, ReadProgress, Result, WatchProgress,
};
use river_ports::{LibraryPort, StorageRepository};
use std::sync::Arc;

pub struct LibraryService {
    storage: Arc<dyn StorageRepository>,
}

impl LibraryService {
    pub fn new(storage: Arc<dyn StorageRepository>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl LibraryPort for LibraryService {
    async fn get_library_items(&self, category: Option<MediaCategory>) -> Result<Vec<MediaItem>> {
        self.storage.get_library_items(category).await
    }

    async fn add_to_library(&self, item: MediaItem) -> Result<()> {
        self.storage.save_library_item(&item).await
    }

    async fn remove_from_library(&self, plugin_id: &PluginId, item_id: &str) -> Result<()> {
        self.storage.remove_library_item(&plugin_id.0, item_id).await
    }

    async fn is_in_library(&self, plugin_id: &PluginId, item_id: &str) -> Result<bool> {
        self.storage.is_in_library(&plugin_id.0, item_id).await
    }

    async fn save_watch_progress(&self, progress: WatchProgress) -> Result<()> {
        self.storage.save_watch_progress(&progress).await
    }

    async fn get_watch_progress(&self, item_id: &str) -> Result<Option<WatchProgress>> {
        self.storage.get_watch_progress(item_id).await
    }

    async fn save_read_progress(&self, progress: ReadProgress) -> Result<()> {
        self.storage.save_read_progress(&progress).await
    }

    async fn get_read_progress(&self, item_id: &str) -> Result<Option<ReadProgress>> {
        self.storage.get_read_progress(item_id).await
    }
}
