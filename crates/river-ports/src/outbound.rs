use async_trait::async_trait;
use river_core::{
    Catalog, CatalogRequest, MangaChapter, MangaPage, MediaCategory, MediaItem,
    MediaStream, ReadProgress, Result, Subtitle, WatchProgress, PluginMeta,
};

#[async_trait]
pub trait PluginProvider: Send + Sync {
    fn metadata(&self) -> PluginMeta;
    async fn fetch_catalogs(&self, req: &CatalogRequest) -> Result<Vec<Catalog>>;
    async fn search(&self, query: &str) -> Result<Vec<MediaItem>>;
    async fn get_item_details(&self, item_id: &str) -> Result<MediaItem>;
    async fn get_streams(&self, item_id: &str) -> Result<Vec<MediaStream>>;
    async fn get_manga_chapters(&self, manga_id: &str) -> Result<Vec<MangaChapter>>;
    async fn get_manga_pages(&self, chapter_id: &str) -> Result<Vec<MangaPage>>;
    async fn get_subtitles(&self, item_id: &str) -> Result<Vec<Subtitle>>;
}

#[async_trait]
pub trait StorageRepository: Send + Sync {
    async fn init_schema(&self) -> Result<()>;
    async fn save_library_item(&self, item: &MediaItem) -> Result<()>;
    async fn remove_library_item(&self, plugin_id: &str, item_id: &str) -> Result<()>;
    async fn get_library_items(&self, category: Option<MediaCategory>) -> Result<Vec<MediaItem>>;
    async fn is_in_library(&self, plugin_id: &str, item_id: &str) -> Result<bool>;
    async fn save_watch_progress(&self, progress: &WatchProgress) -> Result<()>;
    async fn get_watch_progress(&self, item_id: &str) -> Result<Option<WatchProgress>>;
    async fn save_read_progress(&self, progress: &ReadProgress) -> Result<()>;
    async fn get_read_progress(&self, item_id: &str) -> Result<Option<ReadProgress>>;
    async fn set_plugin_enabled(&self, plugin_id: &str, enabled: bool) -> Result<()>;
    async fn is_plugin_enabled(&self, plugin_id: &str) -> Result<bool>;
}

#[async_trait]
pub trait NetworkClient: Send + Sync {
    async fn get_json_untyped(&self, url: &str) -> Result<serde_json::Value>;
    async fn get_text(&self, url: &str) -> Result<String>;
    async fn post_json_untyped(&self, url: &str, payload: &serde_json::Value) -> Result<serde_json::Value>;
}

impl<T: ?Sized + NetworkClient> NetworkClientExt for T {}

#[async_trait]
pub trait NetworkClientExt: NetworkClient {
    async fn get_json<Resp: serde::de::DeserializeOwned + Send>(&self, url: &str) -> Result<Resp> {
        let val = self.get_json_untyped(url).await?;
        serde_json::from_value(val).map_err(|e| river_core::RiverError::Serialization(e.to_string()))
    }

    async fn post_json<Req: serde::Serialize + Send + Sync, Resp: serde::de::DeserializeOwned + Send>(
        &self,
        url: &str,
        payload: &Req,
    ) -> Result<Resp> {
        let req_val = serde_json::to_value(payload).map_err(|e| river_core::RiverError::Serialization(e.to_string()))?;
        let resp_val = self.post_json_untyped(url, &req_val).await?;
        serde_json::from_value(resp_val).map_err(|e| river_core::RiverError::Serialization(e.to_string()))
    }
}
