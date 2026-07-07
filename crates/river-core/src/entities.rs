use serde::{Deserialize, Serialize};
use crate::value_objects::{MediaCategory, PluginId};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaItem {
    pub id: String,
    pub plugin_id: PluginId,
    pub category: MediaCategory,
    pub title: String,
    pub description: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub year: Option<i32>,
    pub genres: Vec<String>,
    pub author_or_creator: Option<String>,
    pub rating: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaStream {
    pub id: String,
    pub title: String,
    pub url: String,
    pub quality: Option<String>,
    pub is_direct: bool,
    pub is_hls_or_dash: bool,
    pub is_magnet_or_torrent: bool,
    pub headers: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MangaChapter {
    pub id: String,
    pub manga_id: String,
    pub chapter_number: f32,
    pub title: Option<String>,
    pub volume: Option<i32>,
    pub scanlator: Option<String>,
    pub release_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MangaPage {
    pub page_number: u32,
    pub image_url: String,
    pub headers: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Subtitle {
    pub id: String,
    pub language: String,
    pub url: String,
    pub format: String, // e.g., "vtt", "srt", "ass"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginMeta {
    pub id: PluginId,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub supported_categories: Vec<MediaCategory>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CatalogRequest {
    pub category: MediaCategory,
    pub catalog_id: Option<String>,
    pub search_query: Option<String>,
    pub genre_filter: Option<String>,
    pub page: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Catalog {
    pub id: String,
    pub name: String,
    pub category: MediaCategory,
    pub items: Vec<MediaItem>,
}
