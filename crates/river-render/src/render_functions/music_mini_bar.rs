use super::*;
use eframe::egui;
use river_engine::MusicCmd;
use std::sync::Arc;

/// Music Player: Mini-Bar View
/// Compact horizontal playback bar for status bars and headers.
pub fn register(registry: &mut RenderFunctionRegistry) {
    registry.register(
        "music-player:mini-bar",
        Arc::new(|ui, ctx, props, _children| {
            let show_info = prop_bool(props, "show_info", true);
            let show_prev = prop_bool(props, "show_prev", true);
            let show_next = prop_bool(props, "show_next", true);
            let show_volume = prop_bool(props, "show_volume", true);

            let player = &ctx.engine.music_player;
            let state = player.state();

            let accent = prop_color(props, "accent", ctx.config, ctx.config.accent_color);
            let text_color = prop_color(props, "text_color", ctx.config, ctx.config.text_color);
            let secondary = prop_color(props, "secondary", ctx.config, ctx.config.secondary_color);
            let rounding = prop_f32(props, "rounding", ctx.config.rounding);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0 * ctx.scale;

                if show_info {
                    if !state.title.is_empty() {
                        let track_str = if let Some(artist) = &state.artist {
                            if !artist.is_empty() {
                                format!("{} — {}", state.title, artist)
                            } else {
                                state.title.clone()
                            }
                        } else {
                            state.title.clone()
                        };
                        ui.label(
                            egui::RichText::new(format!("🎵 {}", track_str))
                                .size(13.0 * ctx.scale)
                                .color(text_color)
                                .strong(),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("🎵 Music Idle")
                                .size(13.0 * ctx.scale)
                                .color(secondary),
                        );
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if show_volume {
                        let mut vol = state.volume;
                        let vol_w = prop_f32(props, "volume_width", 70.0) * ctx.scale;
                        if ui.add_sized([vol_w, 18.0], egui::Slider::new(&mut vol, 0.0..=1.0).show_value(false)).changed() {
                            player.send(MusicCmd::SetVolume(vol));
                        }
                        ui.label(egui::RichText::new("🔊").color(secondary));
                    }

                    if show_next {
                        let next_btn = egui::Button::new(
                            egui::RichText::new("⏭").size(14.0 * ctx.scale).color(text_color),
                        ).rounding(rounding);
                        if ui.add(next_btn).clicked() {
                            player.send(MusicCmd::Next);
                        }
                    }

                    let is_active = state.status.is_active();
                    let play_icon = if is_active { "⏸" } else { "▶" };
                    let play_btn = egui::Button::new(
                        egui::RichText::new(play_icon).size(14.0 * ctx.scale).color(accent).strong(),
                    ).rounding(rounding);
                    if ui.add(play_btn).clicked() {
                        if is_active {
                            player.send(MusicCmd::Pause);
                        } else {
                            player.send(MusicCmd::Play);
                        }
                    }

                    if show_prev {
                        let prev_btn = egui::Button::new(
                            egui::RichText::new("⏮").size(14.0 * ctx.scale).color(text_color),
                        ).rounding(rounding);
                        if ui.add(prev_btn).clicked() {
                            player.send(MusicCmd::Prev);
                        }
                    }
                });
            });
        }),
    );
}
