use async_trait::async_trait;
use river_core::{
    Catalog, CatalogRequest, MangaChapter, MangaPage, MediaCategory, MediaItem,
    MediaStream, PluginId, PluginMeta, Result, Subtitle,
};
use river_ports::{NetworkClient, PluginProvider};
use std::sync::Arc;

pub struct RssPodcastPlugin {
    _client: Arc<dyn NetworkClient>,
}

impl RssPodcastPlugin {
    pub fn new(client: Arc<dyn NetworkClient>) -> Self {
        Self { _client: client }
    }

    fn podcast_catalog() -> Vec<MediaItem> {
        vec![
            MediaItem {
                id: "pod-01".to_string(),
                plugin_id: PluginId("rss-podcasts".to_string()),
                category: MediaCategory::Podcast,
                title: "Rustacean Station Podcast".to_string(),
                description: Some("A community project for covering all things Rust: interviews, news, and deep dives into the Rust programming language.".to_string()),
                poster_url: Some("https://rustacean-station.org/img/rustacean-station-logo.png".to_string()),
                backdrop_url: None,
                year: Some(2026),
                genres: vec!["Technology".to_string(), "Programming".to_string(), "Rust".to_string()],
                author_or_creator: Some("Rustacean Station Team".to_string()),
                rating: Some(5.0),
            },
            MediaItem {
                id: "pod-02".to_string(),
                plugin_id: PluginId("rss-podcasts".to_string()),
                category: MediaCategory::Podcast,
                title: "Lex Fridman Podcast".to_string(),
                description: Some("Conversations about science, technology, history, philosophy, and the nature of intelligence.".to_string()),
                poster_url: Some("https://lexfridman.com/wordpress/wp-content/uploads/2021/04/lex_fridman_podcast_cover.jpg".to_string()),
                backdrop_url: None,
                year: Some(2026),
                genres: vec!["Science".to_string(), "AI".to_string(), "Philosophy".to_string()],
                author_or_creator: Some("Lex Fridman".to_string()),
                rating: Some(4.9),
            },
        ]
    }
}

#[async_trait]
impl PluginProvider for RssPodcastPlugin {
    fn metadata(&self) -> PluginMeta {
        PluginMeta {
            id: PluginId("rss-podcasts".to_string()),
            name: "Open Podcast RSS Feed Adapter".to_string(),
            version: "1.0.0".to_string(),
            author: "River Community".to_string(),
            description: "Aggregates open RSS podcast directories and audio streams.".to_string(),
            supported_categories: vec![MediaCategory::Podcast],
            icon_url: Some("https://www.rssboard.org/images/rss-icon-128x128.png".to_string()),
        }
    }

    async fn fetch_catalogs(&self, req: &CatalogRequest) -> Result<Vec<Catalog>> {
        if req.category != MediaCategory::Podcast {
            return Ok(vec![]);
        }

        Ok(vec![Catalog {
            id: "rss-top-podcasts".to_string(),
            name: "Top Tech & Science Podcasts".to_string(),
            category: MediaCategory::Podcast,
            items: Self::podcast_catalog(),
        }])
    }

    async fn search(&self, query: &str) -> Result<Vec<MediaItem>> {
        let q_lower = query.to_lowercase();
        Ok(Self::podcast_catalog().into_iter().filter(|i| i.title.to_lowercase().contains(&q_lower)).collect())
    }

    async fn get_item_details(&self, item_id: &str) -> Result<MediaItem> {
        Self::podcast_catalog().into_iter().find(|i| i.id == item_id)
            .ok_or_else(|| river_core::RiverError::NotFound(format!("Podcast {} not found", item_id)))
    }

    async fn get_streams(&self, _item_id: &str) -> Result<Vec<MediaStream>> {
        Ok(vec![
            MediaStream {
                id: "podcast-ep1".to_string(),
                title: "Latest Episode Direct MP3".to_string(),
                url: "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-2.mp3".to_string(),
                quality: Some("192kbps".to_string()),
                is_direct: true,
                is_hls_or_dash: false,
                is_magnet_or_torrent: false,
                headers: None,
            },
        ])
    }

    async fn get_manga_chapters(&self, _manga_id: &str) -> Result<Vec<MangaChapter>> {
        Ok(vec![])
    }

    async fn get_manga_pages(&self, _chapter_id: &str) -> Result<Vec<MangaPage>> {
        Ok(vec![])
    }

    async fn get_subtitles(&self, _item_id: &str) -> Result<Vec<Subtitle>> {
        Ok(vec![])
    }
}
