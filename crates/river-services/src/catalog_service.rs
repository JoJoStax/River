use async_trait::async_trait;
use river_core::{
    Catalog, CatalogRequest, MangaChapter, MangaPage, MediaCategory, MediaItem,
    MediaStream, PluginId, Result, Subtitle,
};
use river_ports::CatalogPort;
use crate::plugin_service::PluginService;
use std::sync::Arc;

pub struct CatalogService {
    plugin_service: Arc<PluginService>,
}

impl CatalogService {
    pub fn new(plugin_service: Arc<PluginService>) -> Self {
        Self { plugin_service }
    }
}

#[async_trait]
impl CatalogPort for CatalogService {
    async fn get_catalogs(&self, category: MediaCategory) -> Result<Vec<Catalog>> {
        let providers = self.plugin_service.get_enabled_providers(Some(category)).await;
        let mut all_catalogs = Vec::new();
        let req = CatalogRequest {
            category,
            catalog_id: None,
            search_query: None,
            genre_filter: None,
            page: 1,
        };

        for provider in providers {
            match provider.fetch_catalogs(&req).await {
                Ok(mut cats) => all_catalogs.append(&mut cats),
                Err(e) => {
                    tracing::warn!("Failed to fetch catalog from plugin {}: {}", provider.metadata().name, e);
                }
            }
        }

        Ok(all_catalogs)
    }

    async fn search(&self, query: &str) -> Result<Vec<MediaItem>> {
        let providers = self.plugin_service.get_enabled_providers(None).await;
        let mut all_items = Vec::new();

        for provider in providers {
            match provider.search(query).await {
                Ok(mut items) => all_items.append(&mut items),
                Err(e) => {
                    tracing::warn!("Failed to search plugin {}: {}", provider.metadata().name, e);
                }
            }
        }

        // Simple deduplication based on title & category
        let mut seen = std::collections::HashSet::new();
        all_items.retain(|item| {
            let key = format!("{}:{}", item.category, item.title.to_lowercase());
            seen.insert(key)
        });

        Ok(all_items)
    }

    async fn get_item_details(&self, plugin_id: &PluginId, item_id: &str) -> Result<MediaItem> {
        let provider = self.plugin_service.get_provider(plugin_id).await?;
        provider.get_item_details(item_id).await
    }

    async fn get_streams(&self, plugin_id: &PluginId, item_id: &str) -> Result<Vec<MediaStream>> {
        let provider = self.plugin_service.get_provider(plugin_id).await?;
        provider.get_streams(item_id).await
    }

    async fn get_manga_chapters(&self, plugin_id: &PluginId, manga_id: &str) -> Result<Vec<MangaChapter>> {
        let provider = self.plugin_service.get_provider(plugin_id).await?;
        provider.get_manga_chapters(manga_id).await
    }

    async fn get_manga_pages(&self, plugin_id: &PluginId, chapter_id: &str) -> Result<Vec<MangaPage>> {
        let provider = self.plugin_service.get_provider(plugin_id).await?;
        provider.get_manga_pages(chapter_id).await
    }

    async fn get_subtitles(&self, plugin_id: &PluginId, item_id: &str) -> Result<Vec<Subtitle>> {
        let provider = self.plugin_service.get_provider(plugin_id).await?;
        provider.get_subtitles(item_id).await
    }
}
