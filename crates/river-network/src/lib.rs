use async_trait::async_trait;
use reqwest::Client;
use river_core::{Result, RiverError};
use river_ports::NetworkClient;

pub struct ReqwestClient {
    client: Client,
}

impl ReqwestClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("River-Media-Aggregator/0.1 (Rust)")
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_default();
        Self { client }
    }
}

impl Default for ReqwestClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NetworkClient for ReqwestClient {
    async fn get_json_untyped(&self, url: &str) -> Result<serde_json::Value> {
        let resp = self.client.get(url).send().await
            .map_err(|e| RiverError::Network(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(RiverError::Network(format!("HTTP {}", resp.status())));
        }
        let val = resp.json::<serde_json::Value>().await
            .map_err(|e| RiverError::Serialization(e.to_string()))?;
        Ok(val)
    }

    async fn get_text(&self, url: &str) -> Result<String> {
        let resp = self.client.get(url).send().await
            .map_err(|e| RiverError::Network(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(RiverError::Network(format!("HTTP {}", resp.status())));
        }
        let text = resp.text().await
            .map_err(|e| RiverError::Network(e.to_string()))?;
        Ok(text)
    }

    async fn post_json_untyped(&self, url: &str, payload: &serde_json::Value) -> Result<serde_json::Value> {
        let resp = self.client.post(url).json(payload).send().await
            .map_err(|e| RiverError::Network(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(RiverError::Network(format!("HTTP {}", resp.status())));
        }
        let val = resp.json::<serde_json::Value>().await
            .map_err(|e| RiverError::Serialization(e.to_string()))?;
        Ok(val)
    }
}
