use super::*;
use eframe::egui;
use river_engine::ComicCmd;
use std::sync::Arc;

//  renders settings from every aspect of the app, including plugins, and the core app itself. This is a dynamic settings page that can be themed and customized by plugins.
pub fn register(registry: &mut RenderFunctionRegistry) {
    registry.register(
        "list-settings",
        Arc::new(|ui, ctx, props, _children| {
            let config = ctx.config.clone();
            let state = ctx.state.clone();
            let store = ctx.store.clone();
            let engine = &ctx.engine;

            let render_ctx = crate::render_functions::RenderContext {
                config,
                ctx: ui.ctx().clone(),
                state,
                store,
                engine,
            };

      

            // Render the settings list using the provided context
            render_settings_list(ui, &render_ctx);

            
        }),
    );
}