use super::*;
use eframe::egui;
use river_engine::MusicCmd;
use std::sync::Arc;

/// Music Player Micro-Widgets:
/// Granular individual components that can be placed anywhere in custom KDL layouts.
pub fn register(registry: &mut RenderFunctionRegistry) {
    // ── Micro: Play Button ───────────────────────────────────────────────────
    registry.register(
        "music-player:play-button",
        Arc::new(|ui, ctx, props, _children| {
            let player = &ctx.engine.music_player;
            let is_active = player.state().status.is_active();
            let size = prop_f32(props, "size", 18.0) * ctx.scale;
            let text_color = prop_color(props, "color", ctx.config, ctx.config.text_color);
            let rounding = prop_f32(props, "rounding", ctx.config.rounding);
            let icon = if is_active { "⏸" } else { "▶" };

            let btn = egui::Button::new(
                egui::RichText::new(icon)
                    .size(size)
                    .color(text_color)
                    .strong(),
            )
            .rounding(rounding);

            if ui.add(btn).clicked() {
                if is_active {
                    player.send(MusicCmd::Pause);
                } else {
                    player.send(MusicCmd::Play);
                }
            }
        }),
    );

    // ── Micro: Slider ────────────────────────────────────────────────────────
    registry.register(
        "music-player:slider",
        Arc::new(|ui, ctx, _props, _children| {
            let player = &ctx.engine.music_player;
            let state = player.state();
            let mut pos = state.position_secs;
            let dur = if state.duration_secs > 0.0 {
                state.duration_secs
            } else {
                1.0
            };

            let slider = ui.add(egui::Slider::new(&mut pos, 0.0..=dur).show_value(false));
            if slider.drag_stopped() {
                player.send(MusicCmd::Seek(pos));
            }
        }),
    );

    // ── Micro: Volume ────────────────────────────────────────────────────────
    registry.register(
        "music-player:volume",
        Arc::new(|ui, ctx, props, _children| {
            let player = &ctx.engine.music_player;
            let mut vol = player.state().volume;
            let w = prop_f32(props, "width", 80.0) * ctx.scale;
            if ui
                .add_sized(
                    [w, 18.0],
                    egui::Slider::new(&mut vol, 0.0..=1.0).show_value(false),
                )
                .changed()
            {
                player.send(MusicCmd::SetVolume(vol));
            }
        }),
    );
}

