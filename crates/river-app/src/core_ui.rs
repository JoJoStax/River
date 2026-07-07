use crate::plugin_ui_core::UiThemeConfig;
use crate::ui_plugin::UiPluginManager;
use crate::ui_renderer::render_theme_layout;
use eframe::egui;
use kdl::KdlDocument;
use river_presentation::{AppState, AppStore};
use std::sync::Arc;

/// An optimized, pre-compiled native UI structure.
///
/// We compile KDL AST DOM trees ahead of time into flat Rust structs,
/// eliminating string parsing and AST walking during frame rendering!
pub struct CompiledUiLayout {
    pub config: UiThemeConfig,
    pub is_compiled: bool,
}

/// `core-ui`: The KDL Compiler & Optimizer.
pub fn compile_kdl(doc: &KdlDocument) -> CompiledUiLayout {
    let config = UiThemeConfig::from_kdl(doc);
    CompiledUiLayout {
        config,
        is_compiled: true,
    }
}

impl CompiledUiLayout {
    /// Renders the entire window architecture using the pre-compiled AST structure with responsive scaling!
    pub fn render_compiled_window(
        &self,
        ctx: &egui::Context,
        state: &AppState,
        store: &Arc<AppStore>,
        rt: &tokio::runtime::Runtime,
        ui_manager: &mut UiPluginManager,
    ) {
        render_theme_layout(&self.config, ctx, state, store, rt, ui_manager);
    }
}
