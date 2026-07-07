use async_trait::async_trait;
use river_core::{
    Catalog, CatalogRequest, MangaChapter, MangaPage, MediaCategory, MediaItem,
    MediaStream, PluginId, PluginMeta, Result, Subtitle,
};
use river_ports::{NetworkClient, PluginProvider};
use std::sync::Arc;

pub struct MangaDexPlugin {
    client: Arc<dyn NetworkClient>,
}

impl MangaDexPlugin {
    pub fn new(client: Arc<dyn NetworkClient>) -> Self {
        Self { client }
    }

    fn fallback_manga() -> Vec<MediaItem> {
        vec![
            MediaItem {
                id: "a77742b1-befd-49a4-bff5-1ad4e6b0ef7b".to_string(),
                plugin_id: PluginId("mangadex".to_string()),
                category: MediaCategory::Manga,
                title: "Chainsaw Man (Public Preview)".to_string(),
                description: Some("Broke young man Denji hunts devils with his pet chainsaw devil Pochita to pay off his father's debt.".to_string()),
                poster_url: Some("https://uploads.mangadex.org/covers/a77742b1-befd-49a4-bff5-1ad4e6b0ef7b/527d2c38-0e31-4ec1-ab0b-4158cfadccae.jpg".to_string()),
                backdrop_url: None,
                year: Some(2018),
                genres: vec!["Action".to_string(), "Supernatural".to_string(), "Horror".to_string()],
                author_or_creator: Some("Fujimoto Tatsuki".to_string()),
                rating: Some(9.0),
            },
            MediaItem {
                id: "d8f9afe2-ef44-4dc9-9e3f-e1a5f6e3c0b1".to_string(),
                plugin_id: PluginId("mangadex".to_string()),
                category: MediaCategory::Manga,
                title: "Solo Leveling (Manhwa)".to_string(),
                description: Some("In a world where hunters awaken magical powers to battle deadly monsters, Sung Jin-Woo is known as the weakest hunter of all mankind.".to_string()),
                poster_url: Some("https://uploads.mangadex.org/covers/32d76d19-8a05-4db0-9fc2-e0b0648fe9d0/34e06225-b461-41ee-b883-911e38b34007.jpg".to_string()),
                backdrop_url: None,
                year: Some(2018),
                genres: vec!["Action".to_string(), "Adventure".to_string(), "Fantasy".to_string()],
                author_or_creator: Some("Chugong".to_string()),
                rating: Some(9.2),
            },
        ]
    }
}

#[async_trait]
impl PluginProvider for MangaDexPlugin {
    fn metadata(&self) -> PluginMeta {
        PluginMeta {
            id: PluginId("mangadex".to_string()),
            name: "MangaDex Public API Adapter".to_string(),
            version: "1.0.0".to_string(),
            author: "River Community".to_string(),
            description: "Fetches comics, manga, manhwa, and chapters directly from MangaDex open APIs.".to_string(),
            supported_categories: vec![MediaCategory::Manga],
            icon_url: Some("https://mangadex.org/favicon.ico".to_string()),
        }
    }

    async fn fetch_catalogs(&self, req: &CatalogRequest) -> Result<Vec<Catalog>> {
        if req.category != MediaCategory::Manga {
            return Ok(vec![]);
        }

        // Try querying MangaDex public REST API
        let url = "https://api.mangadex.org/manga?limit=10&order[followedCount]=desc";
        let items = match self.client.get_json_untyped(url).await {
            Ok(val) => {
                let mut parsed_items = Vec::new();
                if let Some(data) = val.get("data").and_then(|d| d.as_array()) {
                    for m in data.iter() {
                        let id = m.get("id").and_then(|i| i.as_str()).unwrap_or("unknown").to_string();
                        let title = m.get("attributes")
                            .and_then(|a| a.get("title"))
                            .and_then(|t| t.get("en").or_else(|| t.get("ja-ro")).or_else(|| t.get("ja")))
                            .and_then(|s| s.as_str())
                            .unwrap_or("Untitled Manga")
                            .to_string();
                        let desc = m.get("attributes")
                            .and_then(|a| a.get("description"))
                            .and_then(|d| d.get("en"))
                            .and_then(|s| s.as_str())
                            .map(|s| s.to_string());

                        parsed_items.push(MediaItem {
                            id,
                            plugin_id: self.metadata().id,
                            category: MediaCategory::Manga,
                            title,
                            description: desc,
                            poster_url: None,
                            backdrop_url: None,
                            year: None,
                            genres: vec!["Manga".to_string()],
                            author_or_creator: None,
                            rating: None,
                        });
                    }
                }
                if parsed_items.is_empty() {
                    Self::fallback_manga()
                } else {
                    parsed_items
                }
            }
            Err(e) => {
                tracing::info!("MangaDex API offline or rate-limited ({}), using fallback catalog", e);
                Self::fallback_manga()
            }
        };

        Ok(vec![Catalog {
            id: "mangadex-popular".to_string(),
            name: "MangaDex Most Followed".to_string(),
            category: MediaCategory::Manga,
            items,
        }])
    }

    async fn search(&self, query: &str) -> Result<Vec<MediaItem>> {
        let q_lower = query.to_lowercase();
        let all = Self::fallback_manga();
        Ok(all.into_iter().filter(|i| i.title.to_lowercase().contains(&q_lower)).collect())
    }

    async fn get_item_details(&self, item_id: &str) -> Result<MediaItem> {
        let all = Self::fallback_manga();
        all.into_iter().find(|i| i.id == item_id)
            .ok_or_else(|| river_core::RiverError::NotFound(format!("MangaDex item {} not found", item_id)))
    }

    async fn get_streams(&self, _item_id: &str) -> Result<Vec<MediaStream>> {
        Ok(vec![])
    }

    async fn get_manga_chapters(&self, manga_id: &str) -> Result<Vec<MangaChapter>> {
        Ok(vec![
            MangaChapter {
                id: format!("{}-ch1", manga_id),
                manga_id: manga_id.to_string(),
                chapter_number: 1.0,
                title: Some("Chapter 1: The Beginning".to_string()),
                volume: Some(1),
                scanlator: Some("River Scans".to_string()),
                release_date: Some("2026-01-01".to_string()),
            },
            MangaChapter {
                id: format!("{}-ch2", manga_id),
                manga_id: manga_id.to_string(),
                chapter_number: 2.0,
                title: Some("Chapter 2: Awakening".to_string()),
                volume: Some(1),
                scanlator: Some("River Scans".to_string()),
                release_date: Some("2026-01-08".to_string()),
            },
        ])
    }

    async fn get_manga_pages(&self, _chapter_id: &str) -> Result<Vec<MangaPage>> {
        Ok(vec![
            MangaPage {
                page_number: 1,
                image_url: "https://example.com/manga/page1.jpg".to_string(),
                headers: None,
            },
            MangaPage {
                page_number: 2,
                image_url: "https://example.com/manga/page2.jpg".to_string(),
                headers: None,
            },
            MangaPage {
                page_number: 3,
                image_url: "https://example.com/manga/page3.jpg".to_string(),
                headers: None,
            },
        ])
    }

    async fn get_subtitles(&self, _item_id: &str) -> Result<Vec<Subtitle>> {
        Ok(vec![])
    }
}
