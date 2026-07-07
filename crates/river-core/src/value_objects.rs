use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaCategory {
    Video,
    Music,
    Manga,
    Podcast,
}

impl fmt::Display for MediaCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MediaCategory::Video => write!(f, "video"),
            MediaCategory::Music => write!(f, "music"),
            MediaCategory::Manga => write!(f, "manga"),
            MediaCategory::Podcast => write!(f, "podcast"),
        }
    }
}

impl MediaCategory {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "video" | "movie" | "series" | "anime" => Some(MediaCategory::Video),
            "music" | "audio" | "track" | "album" => Some(MediaCategory::Music),
            "manga" | "comic" | "book" | "manhwa" => Some(MediaCategory::Manga),
            "podcast" | "show" => Some(MediaCategory::Podcast),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(pub String);

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginId(pub String);

impl fmt::Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchProgress {
    pub item_id: String,
    pub current_time_seconds: f64,
    pub total_duration_seconds: f64,
    pub last_updated_unix: i64,
}

impl WatchProgress {
    pub fn percentage(&self) -> f64 {
        if self.total_duration_seconds <= 0.0 {
            0.0
        } else {
            (self.current_time_seconds / self.total_duration_seconds * 100.0).clamp(0.0, 100.0)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadProgress {
    pub item_id: String,
    pub chapter_id: String,
    pub page_number: u32,
    pub total_pages: u32,
    pub last_updated_unix: i64,
}
