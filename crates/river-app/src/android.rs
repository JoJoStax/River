#[cfg(target_os = "android")]
use eframe::egui;
#[cfg(target_os = "android")]
use std::sync::Arc;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
#[cfg(target_os = "android")]
use winit::platform::android::EventLoopBuilderExtAndroid;

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn android_main(app: AndroidApp) {
    std::env::set_var("RUST_BACKTRACE", "1");
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let engine = rt.block_on(async {
        river_engine::RiverEngine::new_in_memory()
            .await
            .expect("Failed to initialize RiverEngine")
    });

    rt.block_on(async {
        engine
            .store
            .dispatch(river_presentation::Intent::LoadCatalogs(
                river_core::MediaCategory::Video,
            ))
            .await;
    });

    let gui_app = crate::RiverGuiApp::new(Arc::new(engine), rt);

    let mut options = eframe::NativeOptions::default();
    options.event_loop_builder = Some(Box::new(move |builder| {
        builder.with_android_app(app);
    }));

    let _ = eframe::run_native(
        "River Media Hub",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(gui_app))
        }),
    );
}
