pub mod catalog_categories;
pub mod catalog_grid;
pub mod comic;
pub mod music_controls;
pub mod music_mini_bar;
pub mod music_queue;
pub mod music_widgets;
pub mod pdf;
pub mod search;
pub mod video;

use crate::data_exports::DataContext;
use crate::plugin_ui_core::{UiNode, UiThemeConfig};
use eframe::egui;
use river_engine::RiverEngine;
use river_presentation::{AppState, AppStore};
use std::collections::HashMap;
use std::sync::Arc;

/// Context passed to all dynamically invoked functional views.
pub struct RenderContext<'a> {
    pub config: &'a UiThemeConfig,
    pub ctx: egui::Context,
    pub state: &'a AppState,
    pub store: &'a Arc<AppStore>,
    pub engine: &'a Arc<RiverEngine>,
    pub rt: &'a tokio::runtime::Runtime,
    pub scale: f32,
    pub time: f64,
    pub data_ctx: &'a DataContext,
}

/// A dynamic render function capable of drawing an interactive UI component.
pub type RenderFn = Arc<
    dyn Fn(&mut egui::Ui, &RenderContext, &HashMap<String, String>, &[UiNode]) + Send + Sync,
>;

/// Registry holding all functional UI renderers available to KDL themes and plugins.
pub struct RenderFunctionRegistry {
    functions: HashMap<String, RenderFn>,
}

impl Default for RenderFunctionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderFunctionRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            functions: HashMap::new(),
        };
        music_controls::register(&mut registry);
        music_mini_bar::register(&mut registry);
        music_queue::register(&mut registry);
        music_widgets::register(&mut registry);
        video::register(&mut registry);
        comic::register(&mut registry);
        pdf::register(&mut registry);
        catalog_categories::register(&mut registry);
        catalog_grid::register(&mut registry);
        search::register(&mut registry);
        registry
    }

    /// Register a custom functional view (can be called by external plugins).
    pub fn register(&mut self, id: &str, f: RenderFn) {
        self.functions.insert(id.to_string(), f);
    }

    /// Render a function by its registered identifier.
    /// Returns `true` if found and rendered, `false` otherwise.
    pub fn render(
        &self,
        id: &str,
        ui: &mut egui::Ui,
        ctx: &RenderContext,
        props: &HashMap<String, String>,
        children: &[UiNode],
    ) -> bool {
        if let Some(f) = self.functions.get(id) {
            f(ui, ctx, props, children);
            true
        } else {
            false
        }
    }

    /// Returns a list of all registered function IDs.
    pub fn list_functions(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
    }
}

// ─── Theme & Prop Helpers ───────────────────────────────────────────────────

pub fn resolve_color(
    color: &str,
    config: &UiThemeConfig,
    default_color: egui::Color32,
) -> egui::Color32 {
    match color {
        "accent" => config.accent_color,
        "secondary" => config.secondary_color,
        "text" => config.text_color,
        "border" => config.border_color,
        "fill" => config.fill_color,
        "background" => config.background_color,
        "transparent" => egui::Color32::TRANSPARENT,
        hex if hex.starts_with('#') => {
            crate::plugin_ui_core::parse_hex_color(hex).unwrap_or(default_color)
        }
        _ => default_color,
    }
}

pub fn prop_bool(props: &HashMap<String, String>, key: &str, default_val: bool) -> bool {
    match props.get(key).map(|s| s.trim().to_lowercase()) {
        Some(s) if s == "false" || s == "0" || s == "no" || s == "disabled" || s == "hide" => false,
        Some(s) if s == "true" || s == "1" || s == "yes" || s == "enabled" || s == "show" => true,
        _ => default_val,
    }
}

pub fn prop_f32(props: &HashMap<String, String>, key: &str, default_val: f32) -> f32 {
    props.get(key).and_then(|s| s.parse::<f32>().ok()).unwrap_or(default_val)
}

pub fn prop_color(
    props: &HashMap<String, String>,
    key: &str,
    config: &UiThemeConfig,
    default_color: egui::Color32,
) -> egui::Color32 {
    props
        .get(key)
        .map(|s| resolve_color(s, config, default_color))
        .unwrap_or(default_color)
}
