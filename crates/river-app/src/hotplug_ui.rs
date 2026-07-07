use crate::plugin_ui_core::UiThemeConfig;
use crate::ui_plugin::UiPluginManager;
use crate::ui_renderer::render_theme_layout;
use eframe::egui;
use kdl::KdlDocument;
use river_presentation::{AppState, AppStore};
use std::sync::Arc;

/// `hotplug-ui`: The Dynamic KDL Runner with Responsive Scaling & Multi-Device Target Layouts!
///
/// All common DOM evaluation, widget rendering, and complex background animations
/// have been extracted into `ui_renderer.rs` and `ui_backgrounds.rs`, eliminating code duplication.
pub fn run_ui_plugin(
    doc: &KdlDocument,
    ctx: &egui::Context,
    state: &AppState,
    store: &Arc<AppStore>,
    rt: &tokio::runtime::Runtime,
    ui_manager: &mut UiPluginManager,
) {
    let config = UiThemeConfig::from_kdl(doc);
    render_theme_layout(&config, ctx, state, store, rt, ui_manager);
}
