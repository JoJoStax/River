use super::*;
use eframe::egui;
use river_presentation::Intent;
use std::sync::Arc;

pub fn register(registry: &mut RenderFunctionRegistry) {
    registry.register(
        "search:bar",
        Arc::new(|ui, ctx, props, _children| {
            let placeholder = props
                .get("placeholder")
                .cloned()
                .unwrap_or_else(|| "Search media...".to_string());
            let width = prop_f32(props, "width", 220.0) * ctx.scale;
            let mut query = String::new();

            ui.horizontal(|ui| {
                if prop_bool(props, "show_icon", true) {
                    let icon_color = prop_color(props, "icon_color", ctx.config, ctx.config.secondary_color);
                    ui.label(egui::RichText::new("🔍").color(icon_color));
                }
                let text_edit = egui::TextEdit::singleline(&mut query)
                    .hint_text(placeholder)
                    .desired_width(width);
                let resp = ui.add(text_edit);
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !query.is_empty() {
                    let store = ctx.store.clone();
                    let q = query.clone();
                    ctx.rt.block_on(async {
                        store.dispatch(Intent::Search { query: q }).await;
                    });
                }
            });
        }),
    );
}
