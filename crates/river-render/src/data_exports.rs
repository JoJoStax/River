//! # River Data Exports Module
//!
//! Bridges River's domain/presentation layer with the declarative KDL UI engine.
//!
//! The backend **exports** all dynamic data (categories, catalogs, media items, theme info)
//! as structured `DataContext` bindings. KDL UI plugins consume these bindings through
//! `{key}` text interpolation in their layout templates, enabling total creative freedom
//! without any hardcoded domain knowledge in the renderer.

use crate::plugin_ui_core::UiThemeConfig;
use crate::ui_plugin::UiPluginManager;
use river_core::MediaCategory;
use river_presentation::{AppState, CatalogState};
use std::collections::HashMap;

// ============================================================================
// DATA VALUE — Any bindable value in the template system
// ============================================================================

/// Represents any value that can be bound and interpolated in KDL UI templates.
#[derive(Debug, Clone)]
pub enum DataValue {
    /// A string value (most common: titles, labels, IDs, state names).
    Str(String),
    /// A floating-point number (sizes, ratings, speeds).
    Float(f32),
    /// An integer value (years, counts, indices).
    Int(i64),
    /// A boolean flag (active states, visibility toggles).
    Bool(bool),
    /// An iterable list of child contexts (used by `for-each` nodes).
    List(Vec<DataContext>),
    /// No value / missing binding.
    None,
}

// ============================================================================
// DATA CONTEXT — Scoped binding environment for template resolution
// ============================================================================

/// A scoped environment of named data bindings.
///
/// During rendering, each `for-each` iteration creates a child `DataContext` that
/// layers item-specific bindings (e.g. `item.title`) on top of the parent context
/// (e.g. `categories`, `catalog_state`). This enables nested data access without
/// losing access to global state.
#[derive(Debug, Clone, Default)]
pub struct DataContext {
    pub bindings: HashMap<String, DataValue>,
}

/// Sentinel for missing list bindings.
static EMPTY_LIST: Vec<DataContext> = Vec::new();

impl DataContext {
    /// Look up a binding by key. Returns `DataValue::None` if not found.
    pub fn get(&self, key: &str) -> &DataValue {
        static NONE: DataValue = DataValue::None;
        self.bindings.get(key).unwrap_or(&NONE)
    }

    /// Convenience: get a string value, or `""` if missing/wrong type.
    pub fn get_str(&self, key: &str) -> &str {
        match self.get(key) {
            DataValue::Str(s) => s.as_str(),
            _ => "",
        }
    }

    /// Convenience: get a boolean value, or `false` if missing/wrong type.
    pub fn get_bool(&self, key: &str) -> bool {
        match self.get(key) {
            DataValue::Bool(b) => *b,
            DataValue::Str(s) => s == "true",
            _ => false,
        }
    }

    /// Convenience: get a float value, or `0.0` if missing/wrong type.
    pub fn get_float(&self, key: &str) -> f32 {
        match self.get(key) {
            DataValue::Float(f) => *f,
            DataValue::Int(i) => *i as f32,
            _ => 0.0,
        }
    }

    /// Convenience: get a list for iteration, or empty slice if missing/wrong type.
    pub fn get_list(&self, key: &str) -> &[DataContext] {
        match self.get(key) {
            DataValue::List(list) => list.as_slice(),
            _ => &EMPTY_LIST,
        }
    }

    /// Create a child context that layers `child_bindings` on top of this context.
    /// Child bindings override parent bindings with the same key.
    pub fn with_child(&self, child_bindings: HashMap<String, DataValue>) -> DataContext {
        let mut merged = self.bindings.clone();
        merged.extend(child_bindings);
        DataContext { bindings: merged }
    }

    /// Resolve `{key}` placeholders in a template string.
    ///
    /// Supports flattened dot-notation keys: `{item.title}`, `{cat.active}`, etc.
    /// If a binding is not found, the placeholder is left as-is (no crash, no empty string).
    ///
    /// # Examples
    /// ```text
    /// template: "Now playing: {item.title} ({item.year})"
    /// bindings: { "item.title" => "Inception", "item.year" => "2010" }
    /// result:   "Now playing: Inception (2010)"
    /// ```
    pub fn resolve_text(&self, template: &str) -> String {
        if !template.contains('{') {
            return template.to_string();
        }

        let mut result = String::with_capacity(template.len());
        let mut chars = template.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '{' {
                // Collect the key until '}'
                let mut key = String::new();
                let mut found_close = false;
                for inner in chars.by_ref() {
                    if inner == '}' {
                        found_close = true;
                        break;
                    }
                    key.push(inner);
                }
                if found_close && !key.is_empty() {
                    // Try to resolve the binding
                    match self.get(&key) {
                        DataValue::Str(s) => result.push_str(s),
                        DataValue::Float(f) => result.push_str(&format!("{}", f)),
                        DataValue::Int(i) => result.push_str(&format!("{}", i)),
                        DataValue::Bool(b) => result.push_str(if *b { "true" } else { "false" }),
                        DataValue::None | DataValue::List(_) => {
                            // Leave placeholder as-is for unresolved bindings
                            result.push('{');
                            result.push_str(&key);
                            result.push('}');
                        }
                    }
                } else {
                    // Malformed placeholder — emit literally
                    result.push('{');
                    result.push_str(&key);
                    if !found_close {
                        // unclosed brace
                    }
                }
            } else {
                result.push(ch);
            }
        }

        result
    }
}

// ============================================================================
// DATA CONTEXT BUILDER — Populates bindings from AppState
// ============================================================================

/// Build a complete `DataContext` from the current application state.
///
/// This is the central bridge that makes all backend data available to KDL plugins.
/// The renderer calls this once per frame and threads the context through all
/// `render_ui_nodes` / `ForEach` / `Condition` evaluations.
pub fn build_data_context(
    state: &AppState,
    ui_manager: &UiPluginManager,
    _config: &UiThemeConfig,
) -> DataContext {
    let mut bindings = HashMap::new();

    // ── Categories ──────────────────────────────────────────────────────
    let all_categories = [
        (MediaCategory::Video, "Video", "🎬"),
        (MediaCategory::Music, "Music", "🎵"),
        (MediaCategory::Manga, "Manga", "📖"),
        (MediaCategory::Podcast, "Podcasts", "🎙️"),
    ];

    let category_contexts: Vec<DataContext> = all_categories
        .iter()
        .map(|(cat, name, icon)| {
            let is_active = state.selected_category == *cat;
            let mut cb = HashMap::new();
            cb.insert("cat.id".to_string(), DataValue::Str(name.to_string()));
            cb.insert("cat.name".to_string(), DataValue::Str(name.to_string()));
            cb.insert("cat.icon".to_string(), DataValue::Str(icon.to_string()));
            cb.insert("cat.active".to_string(), DataValue::Bool(is_active));
            DataContext { bindings: cb }
        })
        .collect();

    bindings.insert("categories".to_string(), DataValue::List(category_contexts));

    // ── Active Category ─────────────────────────────────────────────────
    let active_cat_str = match state.selected_category {
        MediaCategory::Video => "Video",
        MediaCategory::Music => "Music",
        MediaCategory::Manga => "Manga",
        MediaCategory::Podcast => "Podcasts",
    };
    bindings.insert(
        "active_category".to_string(),
        DataValue::Str(active_cat_str.to_string()),
    );

    // ── Catalog State ───────────────────────────────────────────────────
    let (catalog_state_str, catalog_error) = match &state.catalog_state {
        CatalogState::Idle => ("idle", String::new()),
        CatalogState::Loading => ("loading", String::new()),
        CatalogState::Loaded(_) => ("loaded", String::new()),
        CatalogState::Error(e) => ("error", e.clone()),
    };
    bindings.insert(
        "catalog_state".to_string(),
        DataValue::Str(catalog_state_str.to_string()),
    );
    bindings.insert(
        "catalog_error".to_string(),
        DataValue::Str(catalog_error),
    );

    // ── Catalogs (when loaded) ──────────────────────────────────────────
    if let CatalogState::Loaded(catalogs) = &state.catalog_state {
        let catalog_contexts: Vec<DataContext> = catalogs
            .iter()
            .map(|catalog| {
                let mut cb = HashMap::new();
                cb.insert(
                    "catalog.id".to_string(),
                    DataValue::Str(catalog.id.clone()),
                );
                cb.insert(
                    "catalog.name".to_string(),
                    DataValue::Str(catalog.name.clone()),
                );

                // Build item contexts
                let item_contexts: Vec<DataContext> = catalog
                    .items
                    .iter()
                    .enumerate()
                    .map(|(idx, item)| {
                        let mut ib = HashMap::new();
                        ib.insert(
                            "item.id".to_string(),
                            DataValue::Str(item.id.clone()),
                        );
                        ib.insert(
                            "item.title".to_string(),
                            DataValue::Str(item.title.clone()),
                        );
                        ib.insert(
                            "item.description".to_string(),
                            DataValue::Str(
                                item.description.clone().unwrap_or_default(),
                            ),
                        );
                        ib.insert(
                            "item.poster_url".to_string(),
                            DataValue::Str(
                                item.poster_url.clone().unwrap_or_default(),
                            ),
                        );
                        ib.insert(
                            "item.backdrop_url".to_string(),
                            DataValue::Str(
                                item.backdrop_url.clone().unwrap_or_default(),
                            ),
                        );
                        ib.insert(
                            "item.year".to_string(),
                            DataValue::Str(
                                item.year
                                    .map(|y| y.to_string())
                                    .unwrap_or_default(),
                            ),
                        );
                        ib.insert(
                            "item.genres".to_string(),
                            DataValue::Str(item.genres.join(", ")),
                        );
                        ib.insert(
                            "item.rating".to_string(),
                            DataValue::Str(
                                item.rating
                                    .map(|r| format!("{:.1}", r))
                                    .unwrap_or_default(),
                            ),
                        );
                        ib.insert(
                            "item.plugin_id".to_string(),
                            DataValue::Str(item.plugin_id.0.clone()),
                        );
                        ib.insert(
                            "item.author".to_string(),
                            DataValue::Str(
                                item.author_or_creator.clone().unwrap_or_default(),
                            ),
                        );
                        ib.insert(
                            "item.index".to_string(),
                            DataValue::Int(idx as i64),
                        );
                        DataContext { bindings: ib }
                    })
                    .collect();

                cb.insert(
                    "catalog.items".to_string(),
                    DataValue::List(item_contexts),
                );
                DataContext { bindings: cb }
            })
            .collect();

        bindings.insert("catalogs".to_string(), DataValue::List(catalog_contexts));
    } else {
        bindings.insert("catalogs".to_string(), DataValue::List(Vec::new()));
    }

    // ── Theme Info ──────────────────────────────────────────────────────
    let (active_theme_id, active_theme_name) =
        if let Some(active) = ui_manager.active_plugin() {
            (active.id().to_string(), active.name().to_string())
        } else {
            (String::new(), String::new())
        };
    bindings.insert(
        "active_theme".to_string(),
        DataValue::Str(active_theme_id),
    );
    bindings.insert(
        "active_theme_name".to_string(),
        DataValue::Str(active_theme_name),
    );

    let theme_contexts: Vec<DataContext> = ui_manager
        .list_plugins()
        .into_iter()
        .map(|(id, name, compiled)| {
            let mut tb = HashMap::new();
            tb.insert("theme.id".to_string(), DataValue::Str(id));
            tb.insert("theme.name".to_string(), DataValue::Str(name));
            tb.insert("theme.compiled".to_string(), DataValue::Bool(compiled));
            DataContext { bindings: tb }
        })
        .collect();
    bindings.insert("themes".to_string(), DataValue::List(theme_contexts));

    // ── Device Info ─────────────────────────────────────────────────────
    bindings.insert(
        "device_id".to_string(),
        DataValue::Str(ui_manager.device_id().to_string()),
    );

    DataContext { bindings }
}
