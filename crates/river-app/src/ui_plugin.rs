use crate::plugin_core::load_kdl_plugin;
use crate::plugin_ui_core::{dispatch_ui_job, UiExecutionMode, UiThemeConfig};
use eframe::egui;
use river_presentation::{AppState, AppStore};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

/// The Core Contract for declarative UI renderers.
///
/// Notice how `render_window` receives the full `&egui::Context` and `&mut UiPluginManager`!
/// This allows KDL plugins to build the entire structural feel of the application from scratch—
/// defining sidebars, headers, floating docks, or minimalist single-pane layouts!
pub trait UiRenderer: Send + Sync {
    /// Unique identifier for this UI plugin
    fn id(&self) -> &str;

    /// Display name
    fn name(&self) -> &str;

    /// Tiered Compilation Status:
    /// Returns `true` if compiled via `core-ui` (Tier 2).
    /// Returns `false` if interpreted dynamically via `hotplug-ui` (Tier 1).
    fn is_compiled(&self) -> bool;

    /// Render the entire window structure for this frame
    fn render_window(
        &self,
        ctx: &egui::Context,
        state: &AppState,
        store: &Arc<AppStore>,
        rt: &tokio::runtime::Runtime,
        ui_manager: &mut UiPluginManager,
    );
}

/// Manages active KDL UI plugins and handles switching between Hotplug and Core Compiled modes.
pub struct UiPluginManager {
    plugins: Vec<Arc<dyn UiRenderer>>,
    active_plugin_id: String,
    pub device_id: String,
    pub target_override: Option<String>,
    file_timestamps: HashMap<PathBuf, SystemTime>,
}

impl UiPluginManager {
    pub fn new() -> Self {
        let device_id = std::env::var("RIVER_DEVICE_ID")
            .or_else(|_| std::env::var("RIVER_PLATFORM"))
            .unwrap_or_else(|_| format!("{}_{}", std::env::consts::OS, std::env::consts::ARCH));

        Self {
            plugins: Vec::new(),
            active_plugin_id: String::new(),
            device_id,
            target_override: None,
            file_timestamps: HashMap::new(),
        }
    }

    pub fn register_plugin(&mut self, plugin: Arc<dyn UiRenderer>) {
        if let Some(pos) = self.plugins.iter().position(|p| p.id() == plugin.id()) {
            self.plugins[pos] = plugin;
        } else {
            self.plugins.push(plugin);
        }
        if self.active_plugin_id.is_empty() || !self.plugins.iter().any(|p| p.id() == self.active_plugin_id) {
            if let Some(default_p) = self.plugins.iter().find(|p| p.id().to_lowercase().contains("default") || p.name().to_lowercase().contains("default")) {
                self.active_plugin_id = default_p.id().to_string();
            } else if let Some(first_p) = self.plugins.first() {
                self.active_plugin_id = first_p.id().to_string();
            }
        }
    }

    /// Automatically scan a directory for KDL theme files and load or hot-reload them on the fly!
    /// No need to ever edit lib.rs when adding or modifying themes!
    pub fn scan_plugins_dir(&mut self, dir_path: &str) {
        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("kdl") {
                    if let Ok(metadata) = std::fs::metadata(&path) {
                        if let Ok(modified) = metadata.modified() {
                            if self.file_timestamps.get(&path) == Some(&modified) {
                                continue;
                            }
                            self.file_timestamps.insert(path.clone(), modified);
                        }
                    }
                    if let Ok(raw_kdl) = std::fs::read_to_string(&path) {
                        if let Ok(doc) = load_kdl_plugin(&raw_kdl) {
                            let config = UiThemeConfig::from_kdl(&doc);
                            let mode = if config.mode == "core" || config.mode == "compiled" {
                                UiExecutionMode::CoreCompiled
                            } else {
                                UiExecutionMode::HotplugDynamic
                            };
                            let plugin = dispatch_ui_job(&config.id, &config.name, doc, mode);
                            self.register_plugin(plugin);
                        }
                    }
                }
            }
        }
    }

    pub fn active_plugin(&self) -> Option<Arc<dyn UiRenderer>> {
        self.plugins
            .iter()
            .find(|p| p.id() == self.active_plugin_id)
            .cloned()
    }

    pub fn list_plugins(&self) -> Vec<(String, String, bool)> {
        self.plugins
            .iter()
            .map(|p| (p.id().to_string(), p.name().to_string(), p.is_compiled()))
            .collect()
    }

    pub fn switch_to(&mut self, id: &str) -> bool {
        if self.plugins.iter().any(|p| p.id() == id) {
            self.active_plugin_id = id.to_string();
            true
        } else {
            false
        }
    }

    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    pub fn target_override(&self) -> Option<&str> {
        self.target_override.as_deref()
    }

    pub fn set_target_override(&mut self, target: Option<&str>) {
        self.target_override = target.map(|s| s.to_string());
    }

    /// Automatically calculate target device ("desktop", "mobile", or "tv")
    /// primarily by window aspect ratio, while respecting device ID and manual override!
    pub fn resolve_target_device(&self, screen_width: f32, screen_height: f32) -> &str {
        if let Some(override_target) = &self.target_override {
            return override_target.as_str();
        }

        let aspect_ratio = screen_width / screen_height.max(1.0);

        // Check explicit device ID hints first if they strongly indicate mobile or TV!
        let dev_id_lower = self.device_id.to_lowercase();
        if dev_id_lower.contains("android") || dev_id_lower.contains("phone") || dev_id_lower.contains("mobile") {
            if aspect_ratio < 1.2 {
                return "mobile";
            }
        }
        if dev_id_lower.contains("tv") || dev_id_lower.contains("bigscreen") || dev_id_lower.contains("console") || dev_id_lower.contains("steam_deck") {
            if aspect_ratio >= 1.5 {
                return "tv";
            }
        }

        // Primarily calculate by Aspect Ratio!
        if aspect_ratio < 0.85 || screen_width < 520.0 {
            "mobile"
        } else if aspect_ratio >= 1.70 && screen_width >= 1350.0 {
            "tv"
        } else {
            "desktop"
        }
    }
}
