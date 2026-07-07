use river_core::{
    Catalog, MangaChapter, MangaPage, MediaCategory, MediaItem,
    MediaStream, PluginMeta,
};

#[derive(Debug, Clone, PartialEq)]
pub enum CatalogState {
    Idle,
    Loading,
    Loaded(Vec<Catalog>),
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SearchState {
    Idle,
    Searching,
    Results(Vec<MediaItem>),
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DetailsState {
    Idle,
    Loading,
    Loaded { item: MediaItem, streams: Vec<MediaStream>, chapters: Vec<MangaChapter> },
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReaderState {
    Idle,
    Loading,
    Loaded { chapter_id: String, pages: Vec<MangaPage> },
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LibraryState {
    Idle,
    Loading,
    Loaded(Vec<MediaItem>),
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppState {
    pub selected_category: MediaCategory,
    pub catalog_state: CatalogState,
    pub search_state: SearchState,
    pub details_state: DetailsState,
    pub reader_state: ReaderState,
    pub library_state: LibraryState,
    pub plugins: Vec<PluginMeta>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            selected_category: MediaCategory::Video,
            catalog_state: CatalogState::Idle,
            search_state: SearchState::Idle,
            details_state: DetailsState::Idle,
            reader_state: ReaderState::Idle,
            library_state: LibraryState::Idle,
            plugins: Vec::new(),
        }
    }
}
