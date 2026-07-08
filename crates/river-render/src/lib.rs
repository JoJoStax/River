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

const EXTRA_THEME_DIRS: &[&str] = &[
    "/storage/emulated/0/Documents/River/plugins/ui",
    "/storage/emulated/0/Download/River/plugins/ui",
    "/sdcard/Documents/River/plugins/ui",
    "/sdcard/Download/River/plugins/ui",
    "/storage/emulated/0/River/plugins/ui",
    "/sdcard/River/plugins/ui",
];

impl RiverGuiApp {
    pub fn new(engine: Arc<RiverEngine>, rt: tokio::runtime::Runtime) -> Self {
        let mut ui_manager = UiPluginManager::new();

        // Ensure plugins/ui/ directory and default seed files exist on clean installs!
        let _ = std::fs::create_dir_all("plugins/ui");
        let default_themes = [
            ("plugins/ui/default_android_style.kdl", include_str!("../../../plugins/ui/default_android_style.kdl")),
            ("plugins/ui/empty.kdl", include_str!("../../../plugins/ui/empty.kdl")),
            ("plugins/ui/quantum_glass_suite.kdl", include_str!("../../../plugins/ui/quantum_glass_suite.kdl")),
            ("plugins/ui/hyperpulse_cyber_suite.kdl", include_str!("../../../plugins/ui/hyperpulse_cyber_suite.kdl")),
        ];
        for (path, content) in default_themes {
            // ALWAYS load embedded theme directly from memory first so UI is 100% available even if disk I/O fails!
            ui_manager.load_embedded_theme(content);
            // Update disk file so theme improvements (like gradient backgrounds) propagate on existing installs!
            let _ = std::fs::write(path, content);
        }

        for dir in EXTRA_THEME_DIRS {
            if std::fs::create_dir_all(dir).is_ok() {
                for (path, content) in default_themes {
                    if let Some(filename) = std::path::Path::new(path).file_name() {
                        let full_path = std::path::Path::new(dir).join(filename);
                        let _ = std::fs::write(full_path, content);
                    }
                }
            }
        }

        // Automatically scan plugins/ui/ to pick up any user customizations or additional themes!
        ui_manager.scan_plugins_dir("plugins/ui");
        for dir in EXTRA_THEME_DIRS {
            ui_manager.scan_plugins_dir(dir);
        }

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
        self.ui_manager.scan_plugins_dir("plugins/ui");
        for dir in EXTRA_THEME_DIRS {
            self.ui_manager.scan_plugins_dir(dir);
        }

        // Fetch current immutable state from our MVI store synchronously without blocking Tokio runtime!
        let state = self.engine.store.get_state_sync();

        // Delegate 100% of window architecture and layout rendering to active KDL theme!
        if let Some(active_plugin) = self.ui_manager.active_plugin() {
            active_plugin.render_window(ctx, &state, &self.engine.store, &self.rt, &mut self.ui_manager);
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("No active KDL UI plugin loaded!");
                ui.label("Please ensure plugins/ui/ contains valid KDL theme files.");
            });
        }
    }
}
