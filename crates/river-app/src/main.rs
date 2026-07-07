use eframe::egui;
use river_app::RiverGuiApp;
use river_engine::RiverEngine;
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    // Initialize Tokio runtime for background tasks
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let engine = rt.block_on(async {
        RiverEngine::new_with_db_path("river_gui.db")
            .await
            .expect("Failed to initialize RiverEngine")
    });

    // Dispatch initial catalog load
    rt.block_on(async {
        engine
            .store
            .dispatch(river_presentation::Intent::LoadCatalogs(
                river_core::MediaCategory::Video,
            ))
            .await;
    });

    let app = RiverGuiApp::new(Arc::new(engine), rt);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("River Media Hub (Pure Rust)")
            .with_inner_size([1100.0, 750.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "River Media Hub",
        options,
        Box::new(|cc| {
            // Set dark theme
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(app))
        }),
    )
}
