use async_trait::async_trait;
use river_core::{
    MediaCategory, MediaItem, ReadProgress, Result, RiverError, WatchProgress,
};
use river_ports::StorageRepository;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub struct SqliteStorage {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStorage {
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| RiverError::Storage(e.to_string()))?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn new_from_path(path: &str) -> Result<Self> {
        let conn = Connection::open(path)
            .map_err(|e| RiverError::Storage(e.to_string()))?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[async_trait]
impl StorageRepository for SqliteStorage {
    async fn init_schema(&self) -> Result<()> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let guard = conn.lock().unwrap();
            guard.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS library_items (
                    id TEXT NOT NULL,
                    plugin_id TEXT NOT NULL,
                    category TEXT NOT NULL,
                    title TEXT NOT NULL,
                    data_json TEXT NOT NULL,
                    PRIMARY KEY (plugin_id, id)
                );

                CREATE TABLE IF NOT EXISTS watch_progress (
                    item_id TEXT PRIMARY KEY,
                    current_time REAL NOT NULL,
                    total_duration REAL NOT NULL,
                    last_updated INTEGER NOT NULL
                );

                CREATE TABLE IF NOT EXISTS read_progress (
                    item_id TEXT PRIMARY KEY,
                    chapter_id TEXT NOT NULL,
                    page_number INTEGER NOT NULL,
                    total_pages INTEGER NOT NULL,
                    last_updated INTEGER NOT NULL
                );

                CREATE TABLE IF NOT EXISTS plugin_settings (
                    plugin_id TEXT PRIMARY KEY,
                    enabled INTEGER NOT NULL
                );
                "
            ).map_err(|e| RiverError::Storage(e.to_string()))
        })
        .await
        .map_err(|e| RiverError::Internal(e.to_string()))??;
        Ok(())
    }

    async fn save_library_item(&self, item: &MediaItem) -> Result<()> {
        let conn = self.conn.clone();
        let item = item.clone();
        tokio::task::spawn_blocking(move || {
            let guard = conn.lock().unwrap();
            let json = serde_json::to_string(&item)
                .map_err(|e| RiverError::Serialization(e.to_string()))?;
            guard.execute(
                "INSERT OR REPLACE INTO library_items (id, plugin_id, category, title, data_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    item.id,
                    item.plugin_id.0,
                    item.category.to_string(),
                    item.title,
                    json
                ],
            ).map_err(|e| RiverError::Storage(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| RiverError::Internal(e.to_string()))?
    }

    async fn remove_library_item(&self, plugin_id: &str, item_id: &str) -> Result<()> {
        let conn = self.conn.clone();
        let plugin_id = plugin_id.to_string();
        let item_id = item_id.to_string();
        tokio::task::spawn_blocking(move || {
            let guard = conn.lock().unwrap();
            guard.execute(
                "DELETE FROM library_items WHERE plugin_id = ?1 AND id = ?2",
                rusqlite::params![plugin_id, item_id],
            ).map_err(|e| RiverError::Storage(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| RiverError::Internal(e.to_string()))?
    }

    async fn get_library_items(&self, category: Option<MediaCategory>) -> Result<Vec<MediaItem>> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let guard = conn.lock().unwrap();
            let mut items = Vec::new();
            if let Some(cat) = category {
                let mut stmt = guard.prepare("SELECT data_json FROM library_items WHERE category = ?1")
                    .map_err(|e| RiverError::Storage(e.to_string()))?;
                let rows = stmt.query_map([cat.to_string()], |row| row.get::<_, String>(0))
                    .map_err(|e| RiverError::Storage(e.to_string()))?;
                for r in rows {
                    let json = r.map_err(|e| RiverError::Storage(e.to_string()))?;
                    let item: MediaItem = serde_json::from_str(&json)
                        .map_err(|e| RiverError::Serialization(e.to_string()))?;
                    items.push(item);
                }
            } else {
                let mut stmt = guard.prepare("SELECT data_json FROM library_items")
                    .map_err(|e| RiverError::Storage(e.to_string()))?;
                let rows = stmt.query_map([], |row| row.get::<_, String>(0))
                    .map_err(|e| RiverError::Storage(e.to_string()))?;
                for r in rows {
                    let json = r.map_err(|e| RiverError::Storage(e.to_string()))?;
                    let item: MediaItem = serde_json::from_str(&json)
                        .map_err(|e| RiverError::Serialization(e.to_string()))?;
                    items.push(item);
                }
            }
            Ok(items)
        })
        .await
        .map_err(|e| RiverError::Internal(e.to_string()))?
    }

    async fn is_in_library(&self, plugin_id: &str, item_id: &str) -> Result<bool> {
        let conn = self.conn.clone();
        let plugin_id = plugin_id.to_string();
        let item_id = item_id.to_string();
        tokio::task::spawn_blocking(move || {
            let guard = conn.lock().unwrap();
            let mut stmt = guard.prepare("SELECT 1 FROM library_items WHERE plugin_id = ?1 AND id = ?2")
                .map_err(|e| RiverError::Storage(e.to_string()))?;
            let exists = stmt.exists(rusqlite::params![plugin_id, item_id])
                .map_err(|e| RiverError::Storage(e.to_string()))?;
            Ok(exists)
        })
        .await
        .map_err(|e| RiverError::Internal(e.to_string()))?
    }

    async fn save_watch_progress(&self, progress: &WatchProgress) -> Result<()> {
        let conn = self.conn.clone();
        let p = progress.clone();
        tokio::task::spawn_blocking(move || {
            let guard = conn.lock().unwrap();
            guard.execute(
                "INSERT OR REPLACE INTO watch_progress (item_id, current_time, total_duration, last_updated)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![p.item_id, p.current_time_seconds, p.total_duration_seconds, p.last_updated_unix],
            ).map_err(|e| RiverError::Storage(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| RiverError::Internal(e.to_string()))?
    }

    async fn get_watch_progress(&self, item_id: &str) -> Result<Option<WatchProgress>> {
        let conn = self.conn.clone();
        let id = item_id.to_string();
        tokio::task::spawn_blocking(move || {
            let guard = conn.lock().unwrap();
            let mut stmt = guard.prepare("SELECT current_time, total_duration, last_updated FROM watch_progress WHERE item_id = ?1")
                .map_err(|e| RiverError::Storage(e.to_string()))?;
            let mut rows = stmt.query([&id])
                .map_err(|e| RiverError::Storage(e.to_string()))?;
            if let Some(row) = rows.next().map_err(|e| RiverError::Storage(e.to_string()))? {
                Ok(Some(WatchProgress {
                    item_id: id,
                    current_time_seconds: row.get(0).map_err(|e| RiverError::Storage(e.to_string()))?,
                    total_duration_seconds: row.get(1).map_err(|e| RiverError::Storage(e.to_string()))?,
                    last_updated_unix: row.get(2).map_err(|e| RiverError::Storage(e.to_string()))?,
                }))
            } else {
                Ok(None)
            }
        })
        .await
        .map_err(|e| RiverError::Internal(e.to_string()))?
    }

    async fn save_read_progress(&self, progress: &ReadProgress) -> Result<()> {
        let conn = self.conn.clone();
        let p = progress.clone();
        tokio::task::spawn_blocking(move || {
            let guard = conn.lock().unwrap();
            guard.execute(
                "INSERT OR REPLACE INTO read_progress (item_id, chapter_id, page_number, total_pages, last_updated)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![p.item_id, p.chapter_id, p.page_number, p.total_pages, p.last_updated_unix],
            ).map_err(|e| RiverError::Storage(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| RiverError::Internal(e.to_string()))?
    }

    async fn get_read_progress(&self, item_id: &str) -> Result<Option<ReadProgress>> {
        let conn = self.conn.clone();
        let id = item_id.to_string();
        tokio::task::spawn_blocking(move || {
            let guard = conn.lock().unwrap();
            let mut stmt = guard.prepare("SELECT chapter_id, page_number, total_pages, last_updated FROM read_progress WHERE item_id = ?1")
                .map_err(|e| RiverError::Storage(e.to_string()))?;
            let mut rows = stmt.query([&id])
                .map_err(|e| RiverError::Storage(e.to_string()))?;
            if let Some(row) = rows.next().map_err(|e| RiverError::Storage(e.to_string()))? {
                Ok(Some(ReadProgress {
                    item_id: id,
                    chapter_id: row.get(0).map_err(|e| RiverError::Storage(e.to_string()))?,
                    page_number: row.get(1).map_err(|e| RiverError::Storage(e.to_string()))?,
                    total_pages: row.get(2).map_err(|e| RiverError::Storage(e.to_string()))?,
                    last_updated_unix: row.get(3).map_err(|e| RiverError::Storage(e.to_string()))?,
                }))
            } else {
                Ok(None)
            }
        })
        .await
        .map_err(|e| RiverError::Internal(e.to_string()))?
    }

    async fn set_plugin_enabled(&self, plugin_id: &str, enabled: bool) -> Result<()> {
        let conn = self.conn.clone();
        let id = plugin_id.to_string();
        tokio::task::spawn_blocking(move || {
            let guard = conn.lock().unwrap();
            guard.execute(
                "INSERT OR REPLACE INTO plugin_settings (plugin_id, enabled) VALUES (?1, ?2)",
                rusqlite::params![id, if enabled { 1 } else { 0 }],
            ).map_err(|e| RiverError::Storage(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| RiverError::Internal(e.to_string()))?
    }

    async fn is_plugin_enabled(&self, plugin_id: &str) -> Result<bool> {
        let conn = self.conn.clone();
        let id = plugin_id.to_string();
        tokio::task::spawn_blocking(move || {
            let guard = conn.lock().unwrap();
            let mut stmt = guard.prepare("SELECT enabled FROM plugin_settings WHERE plugin_id = ?1")
                .map_err(|e| RiverError::Storage(e.to_string()))?;
            let mut rows = stmt.query([&id])
                .map_err(|e| RiverError::Storage(e.to_string()))?;
            if let Some(row) = rows.next().map_err(|e| RiverError::Storage(e.to_string()))? {
                let val: i32 = row.get(0).map_err(|e| RiverError::Storage(e.to_string()))?;
                Ok(val != 0)
            } else {
                Ok(true) // Default enabled
            }
        })
        .await
        .map_err(|e| RiverError::Internal(e.to_string()))?
    }
}
