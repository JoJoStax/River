use super::*;
use eframe::egui;
use river_engine::VideoCmd;
use std::sync::Arc;

pub fn register(registry: &mut RenderFunctionRegistry) {
    registry.register(
        "video-player:viewport",
        Arc::new(|ui, ctx, props, _children| {
            let height = prop_f32(props, "height", 360.0) * ctx.scale;
            let show_controls = prop_bool(props, "show_controls", true);
            let bg_color = prop_color(props, "fill", ctx.config, ctx.config.fill_color);
            let border_color = prop_color(props, "border", ctx.config, ctx.config.border_color);
            let border_width = prop_f32(props, "border_width", ctx.config.border_width);
            let rounding = prop_f32(props, "rounding", ctx.config.rounding);

            let (video_handle, frame_rx) = &*ctx.engine.video_player.lock().unwrap();
            let state = video_handle.state();

            let avail_width = ui.available_width();
            let (rect, _response) = ui.allocate_exact_size(
                egui::vec2(avail_width, height),
                egui::Sense::click(),
            );

            ui.painter().rect(
                rect,
                rounding,
                bg_color,
                egui::Stroke::new(border_width * ctx.scale, border_color),
            );

            let frame_opt = frame_rx.borrow().clone();
            let status_text = if frame_opt.is_some() {
                "▶ Video Stream Active"
            } else {
                "🎬 Ready to Play Video"
            };

            let text_color = prop_color(props, "text_color", ctx.config, ctx.config.text_color);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                status_text,
                egui::FontId::proportional(16.0 * ctx.scale),
                text_color,
            );

            if show_controls {
                ui.horizontal(|ui| {
                    let is_active = state.status.is_active();
                    let play_icon = if is_active { "⏸" } else { "▶" };
                    let accent = prop_color(props, "accent", ctx.config, ctx.config.accent_color);
                    if ui.button(egui::RichText::new(play_icon).color(accent)).clicked() {
                        if is_active {
                            video_handle.send(VideoCmd::Pause);
                        } else {
                            video_handle.send(VideoCmd::Play);
                        }
                    }

                    if prop_bool(props, "show_time", true) {
                        let pos = state.position_secs;
                        let dur = state.duration_secs;
                        let secondary = prop_color(props, "secondary", ctx.config, ctx.config.secondary_color);
                        ui.label(
                            egui::RichText::new(format!("{:.1}s / {:.1}s", pos, dur))
                                .color(secondary),
                        );
                    }
                });
            }
        }),
    );
}
