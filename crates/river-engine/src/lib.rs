use river_core::Result;
use river_network_reqwest::ReqwestClient;
use river_ports::StorageRepository;
use river_presentation::AppStore;
use river_services::{CatalogService, LibraryService, PluginService};
use river_storage_sqlite::SqliteStorage;
use std::sync::Arc;

pub struct RiverEngine {
    pub store: Arc<AppStore>,
}

impl RiverEngine {
    pub async fn new_in_memory() -> Result<Self> {
        let storage = Arc::new(SqliteStorage::new_in_memory()?);
        storage.init_schema().await?;

        let _network_client = Arc::new(ReqwestClient::new());
        let plugin_service = Arc::new(PluginService::new(storage.clone()));

        // Media scrapers cleaned for UI framework focus!
        // (See docs/DEVELOPER_GUIDE.md for adding custom media providers)

        let catalog_service = Arc::new(CatalogService::new(plugin_service.clone()));
        let library_service = Arc::new(LibraryService::new(storage.clone()));

        let store = Arc::new(AppStore::new(
            catalog_service,
            library_service,
            plugin_service,
        ));

        Ok(Self { store })
    }

    pub async fn new_with_db_path(path: &str) -> Result<Self> {
        let storage = Arc::new(SqliteStorage::new_from_path(path)?);
        storage.init_schema().await?;

        let _network_client = Arc::new(ReqwestClient::new());
        let plugin_service = Arc::new(PluginService::new(storage.clone()));

        // Media scrapers cleaned for UI framework focus!

        let catalog_service = Arc::new(CatalogService::new(plugin_service.clone()));
        let library_service = Arc::new(LibraryService::new(storage.clone()));

        let store = Arc::new(AppStore::new(
            catalog_service,
            library_service,
            plugin_service,
        ));

        Ok(Self { store })
    }
}
