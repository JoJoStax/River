use async_trait::async_trait;
use river_core::{
    MediaCategory, PluginId, PluginMeta, Result, RiverError,
};
use river_ports::{PluginPort, PluginProvider, StorageRepository};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct PluginService {
    providers: RwLock<HashMap<PluginId, Arc<dyn PluginProvider>>>,
    storage: Arc<dyn StorageRepository>,
}

impl PluginService {
    pub fn new(storage: Arc<dyn StorageRepository>) -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
            storage,
        }
    }

    pub async fn register_provider(&self, provider: Arc<dyn PluginProvider>) {
        let meta = provider.metadata();
        let id = meta.id.clone();
        let mut guard = self.providers.write().await;
        guard.insert(id, provider);
    }

    pub async fn get_provider(&self, id: &PluginId) -> Result<Arc<dyn PluginProvider>> {
        let guard = self.providers.read().await;
        guard
            .get(id)
            .cloned()
            .ok_or_else(|| RiverError::NotFound(format!("Plugin not found: {}", id)))
    }

    pub async fn get_enabled_providers(&self, category: Option<MediaCategory>) -> Vec<Arc<dyn PluginProvider>> {
        let guard = self.providers.read().await;
        let mut result = Vec::new();
        for (id, provider) in guard.iter() {
            let enabled = match self.storage.is_plugin_enabled(&id.0).await {
                Ok(val) => val,
                Err(_) => true, // Default to enabled if error or not yet set
            };
            if enabled {
                if let Some(cat) = category {
                    if provider.metadata().supported_categories.contains(&cat) {
                        result.push(provider.clone());
                    }
                } else {
                    result.push(provider.clone());
                }
            }
        }
        result
    }
}

#[async_trait]
impl PluginPort for PluginService {
    async fn list_plugins(&self) -> Vec<PluginMeta> {
        let guard = self.providers.read().await;
        guard.values().map(|p| p.metadata()).collect()
    }

    async fn get_plugin_meta(&self, id: &PluginId) -> Option<PluginMeta> {
        let guard = self.providers.read().await;
        guard.get(id).map(|p| p.metadata())
    }

    async fn enable_plugin(&self, id: &PluginId) -> Result<()> {
        self.storage.set_plugin_enabled(&id.0, true).await
    }

    async fn disable_plugin(&self, id: &PluginId) -> Result<()> {
        self.storage.set_plugin_enabled(&id.0, false).await
    }

    async fn is_plugin_enabled(&self, id: &PluginId) -> bool {
        self.storage.is_plugin_enabled(&id.0).await.unwrap_or(true)
    }
}
