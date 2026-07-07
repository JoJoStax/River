pub mod android;
pub mod core_ui;
pub mod hotplug_ui;
pub mod plugin_core;
pub mod plugin_ui_core;
pub mod ui_backgrounds;
pub mod ui_plugin;
pub mod ui_renderer;

use crate::ui_plugin::UiPluginManager;
use eframe::egui;
use river_engine::RiverEngine;
use std::sync::Arc;

/// The Main GUI Application driver for River.
///
/// Notice how we have STRIPPED OUT all hardcoded top and bottom bars!
/// We hand over 100% of the window context (`ctx`) directly to the active KDL UI theme,
/// allowing themes to build sidebars, headers, floating docks, or minimalist panes from scratch!
pub struct RiverGuiApp {
    engine: Arc<RiverEngine>,
    rt: tokio::runtime::Runtime,
    ui_manager: UiPluginManager,
    image_loaders_installed: bool,
}

impl RiverGuiApp {
    pub fn new(engine: Arc<RiverEngine>, rt: tokio::runtime::Runtime) -> Self {
        let mut ui_manager = UiPluginManager::new();

        // Ensure ui_plugins/ directory and default seed files exist on clean installs!
        let _ = std::fs::create_dir_all("ui_plugins");
        let default_themes = [
            ("ui_plugins/empty.kdl", include_str!("../../../ui_plugins/empty.kdl")),
            ("ui_plugins/default_android_style.kdl", include_str!("../../../ui_plugins/default_android_style.kdl")),
            ("ui_plugins/cyberdeck_pro_suite.kdl", include_str!("../../../ui_plugins/cyberdeck_pro_suite.kdl")),
            ("ui_plugins/console_plaza_suite.kdl", include_str!("../../../ui_plugins/console_plaza_suite.kdl")),
            ("ui_plugins/studio_hifi_suite.kdl", include_str!("../../../ui_plugins/studio_hifi_suite.kdl")),
            ("ui_plugins/one_ui_suite.kdl", include_str!("../../../ui_plugins/one_ui_suite.kdl")),
            ("ui_plugins/windows_xp_suite.kdl", include_str!("../../../ui_plugins/windows_xp_suite.kdl")),
            ("ui_plugins/iphone_ios_suite.kdl", include_str!("../../../ui_plugins/iphone_ios_suite.kdl")),
        ];
        for (path, content) in default_themes {
            if !std::path::Path::new(path).exists() {
                let _ = std::fs::write(path, content);
            }
        }

        // Automatically scan ui_plugins/ and load all KDL themes!
        ui_manager.scan_plugins_dir("ui_plugins");

        Self {
            engine,
            rt,
            ui_manager,
            image_loaders_installed: false,
        }
    }
}

impl eframe::App for RiverGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.image_loaders_installed {
            egui_extras::install_image_loaders(ctx);
            self.image_loaders_installed = true;
        }

        // Automatically check folder for new or modified KDL themes (hot-reloading / auto-detection!)
        self.ui_manager.scan_plugins_dir("ui_plugins");

        // Fetch current immutable state from our MVI store synchronously
        let state = self.rt.block_on(self.engine.store.get_state());

        // Delegate 100% of window architecture and layout rendering to active KDL theme!
        if let Some(active_plugin) = self.ui_manager.active_plugin() {
            active_plugin.render_window(ctx, &state, &self.engine.store, &self.rt, &mut self.ui_manager);
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("No active KDL UI plugin loaded!");
                ui.label("Please ensure ui_plugins/ contains valid KDL theme files.");
            });
        }
    }
}
