use super::*;
use eframe::egui;
use river_engine::MusicCmd;
use std::sync::Arc;

/// Music Player: Full Controls View
/// Granular theme props and toggleable controls.
pub fn register(registry: &mut RenderFunctionRegistry) {
    registry.register(
        "music-player:controls",
        Arc::new(|ui, ctx, props, _children| {
            let show_title = prop_bool(props, "show_title", true);
            let show_artist = prop_bool(props, "show_artist", true);
            let show_slider = prop_bool(props, "show_slider", true);
            let show_buttons = prop_bool(props, "show_buttons", true);
            let show_prev = prop_bool(props, "show_prev", true);
            let show_next = prop_bool(props, "show_next", true);
            let show_volume = prop_bool(props, "show_volume", true);
            let show_time = prop_bool(props, "show_time", true);

            let player = &ctx.engine.music_player;
            let state = player.state();

            let accent = prop_color(props, "accent", ctx.config, ctx.config.accent_color);
            let text_color = prop_color(props, "text_color", ctx.config, ctx.config.text_color);
            let secondary = prop_color(props, "secondary", ctx.config, ctx.config.secondary_color);
            let rounding = prop_f32(props, "rounding", ctx.config.rounding);

            ui.vertical_centered(|ui| {
                ui.add_space(prop_f32(props, "spacing_top", 4.0) * ctx.scale);

                if show_title {
                    let title = if state.title.is_empty() {
                        "No Track Playing"
                    } else {
                        &state.title
                    };
                    ui.label(
                        egui::RichText::new(title)
                            .size(prop_f32(props, "title_size", 16.0) * ctx.scale)
                            .color(text_color)
                            .strong(),
                    );
                }

                if show_artist {
                    if let Some(artist) = &state.artist {
                        if !artist.is_empty() {
                            ui.label(
                                egui::RichText::new(artist)
                                    .size(prop_f32(props, "artist_size", 13.0) * ctx.scale)
                                    .color(secondary),
                            );
                        }
                    }
                }

                if show_slider {
                    ui.add_space(4.0);
                    let position = state.position_secs;
                    let duration = state.duration_secs;
                    let mut pos_f64 = position;
                    let duration_clamped = if duration > 0.0 { duration } else { 1.0 };

                    ui.horizontal(|ui| {
                        if show_time {
                            let pos_min = (position / 60.0).floor() as u32;
                            let pos_sec = (position % 60.0).floor() as u32;
                            ui.label(egui::RichText::new(format!("{:02}:{:02}", pos_min, pos_sec)).color(secondary));
                        }

                        let slider = ui.add(
                            egui::Slider::new(&mut pos_f64, 0.0..=duration_clamped).show_value(false),
                        );
                        if slider.drag_stopped() {
                            player.send(MusicCmd::Seek(pos_f64));
                        }

                        if show_time {
                            let dur_min = (duration / 60.0).floor() as u32;
                            let dur_sec = (duration % 60.0).floor() as u32;
                            ui.label(egui::RichText::new(format!("{:02}:{:02}", dur_min, dur_sec)).color(secondary));
                        }
                    });
                }

                if show_buttons {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if show_prev {
                            let prev_btn = egui::Button::new(
                                egui::RichText::new("⏮").size(18.0 * ctx.scale).color(text_color),
                            ).rounding(rounding);
                            if ui.add(prev_btn).clicked() {
                                player.send(MusicCmd::Prev);
                            }
                        }

                        let is_active = state.status.is_active();
                        let play_icon = if is_active { "⏸" } else { "▶" };
                        let play_btn = egui::Button::new(
                            egui::RichText::new(play_icon)
                                .size(20.0 * ctx.scale)
                                .color(accent)
                                .strong(),
                        ).rounding(rounding);

                        if ui.add(play_btn).clicked() {
                            if is_active {
                                player.send(MusicCmd::Pause);
                            } else {
                                player.send(MusicCmd::Play);
                            }
                        }

                        if show_next {
                            let next_btn = egui::Button::new(
                                egui::RichText::new("⏭").size(18.0 * ctx.scale).color(text_color),
                            ).rounding(rounding);
                            if ui.add(next_btn).clicked() {
                                player.send(MusicCmd::Next);
                            }
                        }

                        if show_volume {
                            ui.separator();
                            ui.label(egui::RichText::new("🔊").color(secondary));
                            let mut vol = state.volume;
                            let vol_w = prop_f32(props, "volume_width", 70.0) * ctx.scale;
                            if ui.add_sized([vol_w, 18.0], egui::Slider::new(&mut vol, 0.0..=1.0).show_value(false)).changed() {
                                player.send(MusicCmd::SetVolume(vol));
                            }
                        }
                    });
                }
            });
        }),
    );
}
