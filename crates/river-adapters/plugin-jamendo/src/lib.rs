use async_trait::async_trait;
use river_core::{
    Catalog, CatalogRequest, MangaChapter, MangaPage, MediaCategory, MediaItem,
    MediaStream, PluginId, PluginMeta, Result, Subtitle,
};
use river_ports::{NetworkClient, PluginProvider};
use std::sync::Arc;

pub struct JamendoPlugin {
    _client: Arc<dyn NetworkClient>,
}

impl JamendoPlugin {
    pub fn new(client: Arc<dyn NetworkClient>) -> Self {
        Self { _client: client }
    }

    fn music_catalog() -> Vec<MediaItem> {
        vec![
            MediaItem {
                id: "track-01".to_string(),
                plugin_id: PluginId("jamendo-music".to_string()),
                category: MediaCategory::Music,
                title: "Cyberpunk Lo-Fi Beats".to_string(),
                description: Some("Futuristic ambient synthwave and lo-fi hip hop beats for coding and focus.".to_string()),
                poster_url: Some("https://images.unsplash.com/photo-1508700115892-45ecd05ae2ad?w=500&auto=format".to_string()),
                backdrop_url: None,
                year: Some(2025),
                genres: vec!["Lo-Fi".to_string(), "Synthwave".to_string(), "Ambient".to_string()],
                author_or_creator: Some("Neon River".to_string()),
                rating: Some(4.8),
            },
            MediaItem {
                id: "track-02".to_string(),
                plugin_id: PluginId("jamendo-music".to_string()),
                category: MediaCategory::Music,
                title: "Acoustic Forest Serenade".to_string(),
                description: Some("Relaxing acoustic guitar melodies mixed with gentle nature sounds.".to_string()),
                poster_url: Some("https://images.unsplash.com/photo-1511671782779-c97d3d27a1d4?w=500&auto=format".to_string()),
                backdrop_url: None,
                year: Some(2024),
                genres: vec!["Acoustic".to_string(), "Folk".to_string(), "Relaxing".to_string()],
                author_or_creator: Some("Echoes of Nature".to_string()),
                rating: Some(4.6),
            },
        ]
    }
}

#[async_trait]
impl PluginProvider for JamendoPlugin {
    fn metadata(&self) -> PluginMeta {
        PluginMeta {
            id: PluginId("jamendo-music".to_string()),
            name: "Jamendo & Open Music Streaming Adapter".to_string(),
            version: "1.0.0".to_string(),
            author: "River Community".to_string(),
            description: "Fetches Creative Commons music tracks, albums, and radio streams.".to_string(),
            supported_categories: vec![MediaCategory::Music],
            icon_url: Some("https://www.jamendo.com/favicon.ico".to_string()),
        }
    }

    async fn fetch_catalogs(&self, req: &CatalogRequest) -> Result<Vec<Catalog>> {
        if req.category != MediaCategory::Music {
            return Ok(vec![]);
        }

        Ok(vec![Catalog {
            id: "jamendo-trending".to_string(),
            name: "Trending Music Tracks".to_string(),
            category: MediaCategory::Music,
            items: Self::music_catalog(),
        }])
    }

    async fn search(&self, query: &str) -> Result<Vec<MediaItem>> {
        let q_lower = query.to_lowercase();
        Ok(Self::music_catalog().into_iter().filter(|i| i.title.to_lowercase().contains(&q_lower)).collect())
    }

    async fn get_item_details(&self, item_id: &str) -> Result<MediaItem> {
        Self::music_catalog().into_iter().find(|i| i.id == item_id)
            .ok_or_else(|| river_core::RiverError::NotFound(format!("Music track {} not found", item_id)))
    }

    async fn get_streams(&self, _item_id: &str) -> Result<Vec<MediaStream>> {
        Ok(vec![
            MediaStream {
                id: "mp3-stream".to_string(),
                title: "320kbps MP3 Audio Stream".to_string(),
                url: "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3".to_string(),
                quality: Some("320kbps".to_string()),
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
