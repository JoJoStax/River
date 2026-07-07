use crate::intent::Intent;
use crate::state::{
    AppState, CatalogState, DetailsState, LibraryState, ReaderState, SearchState,
};
use river_ports::{CatalogPort, LibraryPort, PluginPort};
use std::sync::{Arc, RwLock};

pub struct AppStore {
    state: Arc<RwLock<AppState>>,
    catalog_port: Arc<dyn CatalogPort>,
    library_port: Arc<dyn LibraryPort>,
    plugin_port: Arc<dyn PluginPort>,
}

impl AppStore {
    pub fn new(
        catalog_port: Arc<dyn CatalogPort>,
        library_port: Arc<dyn LibraryPort>,
        plugin_port: Arc<dyn PluginPort>,
    ) -> Self {
        Self {
            state: Arc::new(RwLock::new(AppState::default())),
            catalog_port,
            library_port,
            plugin_port,
        }
    }

    pub async fn get_state(&self) -> AppState {
        self.get_state_sync()
    }

    pub fn get_state_sync(&self) -> AppState {
        self.state.read().unwrap().clone()
    }

    pub async fn dispatch(&self, intent: Intent) {
        match intent {
            Intent::SelectCategory(cat) => {
                let mut guard = self.state.write().unwrap();
                guard.selected_category = cat;
            }
            Intent::LoadCatalogs(cat) => {
                {
                    let mut guard = self.state.write().unwrap();
                    guard.catalog_state = CatalogState::Loading;
                }
                match self.catalog_port.get_catalogs(cat).await {
                    Ok(catalogs) => {
                        let mut guard = self.state.write().unwrap();
                        guard.catalog_state = CatalogState::Loaded(catalogs);
                    }
                    Err(e) => {
                        let mut guard = self.state.write().unwrap();
                        guard.catalog_state = CatalogState::Error(e.to_string());
                    }
                }
            }
            Intent::Search { query } => {
                {
                    let mut guard = self.state.write().unwrap();
                    guard.search_state = SearchState::Searching;
                }
                match self.catalog_port.search(&query).await {
                    Ok(items) => {
                        let mut guard = self.state.write().unwrap();
                        guard.search_state = SearchState::Results(items);
                    }
                    Err(e) => {
                        let mut guard = self.state.write().unwrap();
                        guard.search_state = SearchState::Error(e.to_string());
                    }
                }
            }
            Intent::GetDetails { plugin_id, item_id } => {
                {
                    let mut guard = self.state.write().unwrap();
                    guard.details_state = DetailsState::Loading;
                }
                let item_res = self.catalog_port.get_item_details(&plugin_id, &item_id).await;
                let streams_res = self.catalog_port.get_streams(&plugin_id, &item_id).await;
                let chapters_res = self.catalog_port.get_manga_chapters(&plugin_id, &item_id).await;

                let mut guard = self.state.write().unwrap();
                match item_res {
                    Ok(item) => {
                        let streams = streams_res.unwrap_or_default();
                        let chapters = chapters_res.unwrap_or_default();
                        guard.details_state = DetailsState::Loaded { item, streams, chapters };
                    }
                    Err(e) => {
                        guard.details_state = DetailsState::Error(e.to_string());
                    }
                }
            }
            Intent::ReadChapter { plugin_id, chapter_id } => {
                {
                    let mut guard = self.state.write().unwrap();
                    guard.reader_state = ReaderState::Loading;
                }
                match self.catalog_port.get_manga_pages(&plugin_id, &chapter_id).await {
                    Ok(pages) => {
                        let mut guard = self.state.write().unwrap();
                        guard.reader_state = ReaderState::Loaded { chapter_id, pages };
                    }
                    Err(e) => {
                        let mut guard = self.state.write().unwrap();
                        guard.reader_state = ReaderState::Error(e.to_string());
                    }
                }
            }
            Intent::AddToLibrary(item) => {
                let _ = self.library_port.add_to_library(item).await;
                // Reload library
                let items = self.library_port.get_library_items(None).await.unwrap_or_default();
                let mut guard = self.state.write().unwrap();
                guard.library_state = LibraryState::Loaded(items);
            }
            Intent::RemoveFromLibrary { plugin_id, item_id } => {
                let _ = self.library_port.remove_from_library(&plugin_id, &item_id).await;
                let items = self.library_port.get_library_items(None).await.unwrap_or_default();
                let mut guard = self.state.write().unwrap();
                guard.library_state = LibraryState::Loaded(items);
            }
            Intent::LoadLibrary(cat) => {
                {
                    let mut guard = self.state.write().unwrap();
                    guard.library_state = LibraryState::Loading;
                }
                match self.library_port.get_library_items(cat).await {
                    Ok(items) => {
                        let mut guard = self.state.write().unwrap();
                        guard.library_state = LibraryState::Loaded(items);
                    }
                    Err(e) => {
                        let mut guard = self.state.write().unwrap();
                        guard.library_state = LibraryState::Error(e.to_string());
                    }
                }
            }
            Intent::ListPlugins => {
                let plugins = self.plugin_port.list_plugins().await;
                let mut guard = self.state.write().unwrap();
                guard.plugins = plugins;
            }
            Intent::TogglePlugin { plugin_id, enable } => {
                if enable {
                    let _ = self.plugin_port.enable_plugin(&plugin_id).await;
                } else {
                    let _ = self.plugin_port.disable_plugin(&plugin_id).await;
                }
                let plugins = self.plugin_port.list_plugins().await;
                let mut guard = self.state.write().unwrap();
                guard.plugins = plugins;
            }
        }
    }
}
