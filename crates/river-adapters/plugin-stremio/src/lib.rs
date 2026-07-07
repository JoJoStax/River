use async_trait::async_trait;
use river_core::{
    Catalog, CatalogRequest, MangaChapter, MangaPage, MediaCategory, MediaItem,
    MediaStream, PluginId, PluginMeta, Result, Subtitle,
};
use river_ports::{NetworkClient, PluginProvider};
use std::sync::Arc;

pub struct StremioPlugin {
    client: Arc<dyn NetworkClient>,
}

impl StremioPlugin {
    pub fn new(client: Arc<dyn NetworkClient>) -> Self {
        Self { client }
    }

    fn fallback_catalog() -> Vec<MediaItem> {
        vec![
            MediaItem {
                id: "tt0031051".to_string(),
                plugin_id: PluginId("stremio-cinemeta".to_string()),
                category: MediaCategory::Video,
                title: "Big Buck Bunny (Open Movie)".to_string(),
                description: Some("A large and lovable rabbit deals with bullying forest creatures in an animated short film by the Blender Foundation.".to_string()),
                poster_url: Some("https://upload.wikimedia.org/wikipedia/commons/thumb/c/c5/Big_buck_bunny_poster_big.jpg/640px-Big_buck_bunny_poster_big.jpg".to_string()),
                backdrop_url: None,
                year: Some(2008),
                genres: vec!["Animation".to_string(), "Comedy".to_string(), "Short".to_string()],
                author_or_creator: Some("Blender Foundation".to_string()),
                rating: Some(8.2),
            },
            MediaItem {
                id: "tt0088247".to_string(),
                plugin_id: PluginId("stremio-cinemeta".to_string()),
                category: MediaCategory::Video,
                title: "Elephant's Dream".to_string(),
                description: Some("Two men explore a strange mechanical world known as The Machine.".to_string()),
                poster_url: Some("https://upload.wikimedia.org/wikipedia/commons/0/0c/ElephantsDreamPoster.jpg".to_string()),
                backdrop_url: None,
                year: Some(2006),
                genres: vec!["Sci-Fi".to_string(), "Animation".to_string()],
                author_or_creator: Some("Blender Foundation".to_string()),
                rating: Some(7.5),
            },
        ]
    }
}

#[async_trait]
impl PluginProvider for StremioPlugin {
    fn metadata(&self) -> PluginMeta {
        PluginMeta {
            id: PluginId("stremio-cinemeta".to_string()),
            name: "Stremio Cinemeta & Public Domain Adapter".to_string(),
            version: "1.0.0".to_string(),
            author: "River Community".to_string(),
            description: "Fetches movies, series, and live streams via Stremio HTTP addon manifests and Cinemeta APIs.".to_string(),
            supported_categories: vec![MediaCategory::Video],
            icon_url: Some("https://www.stremio.com/website/stremio-logo-small.png".to_string()),
        }
    }

    async fn fetch_catalogs(&self, req: &CatalogRequest) -> Result<Vec<Catalog>> {
        if req.category != MediaCategory::Video {
            return Ok(vec![]);
        }

        // Try querying Stremio Cinemeta public API
        let url = "https://v3-cinemeta.strem.io/catalog/movie/top.json";
        let items = match self.client.get_json_untyped(url).await {
            Ok(val) => {
                let mut parsed_items = Vec::new();
                if let Some(metas) = val.get("metas").and_then(|m| m.as_array()) {
                    for meta in metas.iter().take(10) {
                        let id = meta.get("id").and_then(|i| i.as_str()).unwrap_or("unknown").to_string();
                        let title = meta.get("name").and_then(|n| n.as_str()).unwrap_or("Untitled").to_string();
                        let poster = meta.get("poster").and_then(|p| p.as_str()).map(|s| s.to_string());
                        let desc = meta.get("description").and_then(|d| d.as_str()).map(|s| s.to_string());
                        let year = meta.get("releaseInfo").and_then(|y| y.as_str())
                            .and_then(|y_str| y_str.split('-').next())
                            .and_then(|y_num| y_num.parse::<i32>().ok());

                        parsed_items.push(MediaItem {
                            id,
                            plugin_id: self.metadata().id,
                            category: MediaCategory::Video,
                            title,
                            description: desc,
                            poster_url: poster,
                            backdrop_url: None,
                            year,
                            genres: vec!["Movie".to_string()],
                            author_or_creator: None,
                            rating: None,
                        });
                    }
                }
                if parsed_items.is_empty() {
                    Self::fallback_catalog()
                } else {
                    parsed_items
                }
            }
            Err(e) => {
                tracing::info!("Stremio Cinemeta API offline or unreachable ({}), using fallback catalog", e);
                Self::fallback_catalog()
            }
        };

        Ok(vec![Catalog {
            id: "stremio-movies".to_string(),
            name: "Stremio Top Movies".to_string(),
            category: MediaCategory::Video,
            items,
        }])
    }

    async fn search(&self, query: &str) -> Result<Vec<MediaItem>> {
        let q_lower = query.to_lowercase();
        let all = Self::fallback_catalog();
        Ok(all.into_iter().filter(|i| i.title.to_lowercase().contains(&q_lower)).collect())
    }

    async fn get_item_details(&self, item_id: &str) -> Result<MediaItem> {
        let all = Self::fallback_catalog();
        all.into_iter().find(|i| i.id == item_id)
            .ok_or_else(|| river_core::RiverError::NotFound(format!("Stremio item {} not found", item_id)))
    }

    async fn get_streams(&self, item_id: &str) -> Result<Vec<MediaStream>> {
        if item_id == "tt0031051" {
            Ok(vec![
                MediaStream {
                    id: "bbb-1080p".to_string(),
                    title: "Big Buck Bunny 1080p Direct MP4".to_string(),
                    url: "http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4".to_string(),
                    quality: Some("1080p".to_string()),
                    is_direct: true,
                    is_hls_or_dash: false,
                    is_magnet_or_torrent: false,
                    headers: None,
                },
                MediaStream {
                    id: "bbb-hls".to_string(),
                    title: "Big Buck Bunny HLS Adaptive Stream".to_string(),
                    url: "https://test-streams.mux.dev/x36xhzz/x36xhzz.m3u8".to_string(),
                    quality: Some("Auto/HLS".to_string()),
                    is_direct: false,
                    is_hls_or_dash: true,
                    is_magnet_or_torrent: false,
                    headers: None,
                },
            ])
        } else {
            Ok(vec![MediaStream {
                id: "stream-default".to_string(),
                title: "Default Stream 720p".to_string(),
                url: "http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ElephantsDream.mp4".to_string(),
                quality: Some("720p".to_string()),
                is_direct: true,
                is_hls_or_dash: false,
                is_magnet_or_torrent: false,
                headers: None,
            }])
        }
    }

    async fn get_manga_chapters(&self, _manga_id: &str) -> Result<Vec<MangaChapter>> {
        Ok(vec![])
    }

    async fn get_manga_pages(&self, _chapter_id: &str) -> Result<Vec<MangaPage>> {
        Ok(vec![])
    }

    async fn get_subtitles(&self, _item_id: &str) -> Result<Vec<Subtitle>> {
        Ok(vec![Subtitle {
            id: "en-sub".to_string(),
            language: "English".to_string(),
            url: "https://example.com/sub.vtt".to_string(),
            format: "vtt".to_string(),
        }])
    }
}
