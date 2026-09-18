use super::*;
use eframe::egui;
use river_engine::MusicCmd;
use std::sync::Arc;

/// Music Player: Queue Status View
/// Shows the queue information and playback order controls.
pub fn register(registry: &mut RenderFunctionRegistry) {
    registry.register(
        "music-player:queue",
        Arc::new(|ui, ctx, props, _children| {
            let player = &ctx.engine.music_player;
            let state = player.state();

            let show_header = prop_bool(props, "show_header", true);
            let show_controls = prop_bool(props, "show_controls", true);
            let accent = prop_color(props, "accent", ctx.config, ctx.config.accent_color);
            let text_color = prop_color(props, "text_color", ctx.config, ctx.config.text_color);
            let secondary = prop_color(props, "secondary", ctx.config, ctx.config.secondary_color);

            if show_header {
                let header_text = props.get("title").cloned().unwrap_or_else(|| "🎶 Up Next".to_string());
                ui.heading(
                    egui::RichText::new(header_text)
                        .size(16.0 * ctx.scale)
                        .color(text_color),
                );
                ui.add_space(4.0);
            }

            if state.queue_len == 0 {
                ui.label(
                    egui::RichText::new("Queue is empty")
                        .color(secondary)
                        .italics(),
                );
            } else {
                ui.label(
                    egui::RichText::new(format!(
                        "Track {} of {} in queue",
                        state.queue_index + 1,
                        state.queue_len
                    ))
                    .size(13.0 * ctx.scale)
                    .color(accent),
                );

                if !state.title.is_empty() {
                    ui.label(
                        egui::RichText::new(format!("▶ Now Playing: {}", state.title))
                            .size(12.0 * ctx.scale)
                            .color(text_color),
                    );
                }

                if show_controls {
                    ui.horizontal(|ui| {
                        if ui.button("⏮ Previous").clicked() {
                            player.send(MusicCmd::Prev);
                        }
                        if ui.button("⏭ Next").clicked() {
                            player.send(MusicCmd::Next);
                        }
                        let shuffle_btn = if state.shuffle { "🔀 Shuffle On" } else { "🔀 Shuffle Off" };
                        if ui.button(shuffle_btn).clicked() {
                            player.send(MusicCmd::ToggleShuffle);
                        }
                    });
                }
            }
        }),
    );
}

