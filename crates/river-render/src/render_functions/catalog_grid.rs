use super::*;
use eframe::egui;
use river_presentation::{CatalogState, Intent};
use std::sync::Arc;

/// Dynamic Media Catalog Grid:
/// Renders media cards for currently active plugin catalogs.
/// Every element is themeable and toggleable via props.
pub fn register(registry: &mut RenderFunctionRegistry) {
    registry.register(
        "catalog:grid",
        Arc::new(|ui, ctx, props, _children| {
            match &ctx.state.catalog_state {
                CatalogState::Loading => {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.spinner();
                        ui.label(
                            egui::RichText::new("Loading media catalog...")
                                .size(14.0 * ctx.scale)
                                .color(ctx.config.secondary_color),
                        );
                    });
                }
                CatalogState::Error(err) => {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(
                            egui::RichText::new(format!("⚠️ Error: {}", err))
                                .size(14.0 * ctx.scale)
                                .color(egui::Color32::from_rgb(255, 100, 100)),
                        );
                    });
                }
                CatalogState::Loaded(catalogs) => {
                    let show_title = prop_bool(props, "show_title", true);
                    let show_year = prop_bool(props, "show_year", true);
                    let show_rating = prop_bool(props, "show_rating", true);
                    let card_w = prop_f32(props, "card_width", 160.0) * ctx.scale;
                    let card_h = prop_f32(props, "card_height", 230.0) * ctx.scale;
                    let rounding = prop_f32(props, "rounding", ctx.config.rounding);
                    let card_fill = prop_color(props, "fill", ctx.config, ctx.config.fill_color);
                    let text_color = prop_color(props, "text_color", ctx.config, ctx.config.text_color);
                    let secondary = prop_color(props, "secondary", ctx.config, ctx.config.secondary_color);
                    let accent = prop_color(props, "accent", ctx.config, ctx.config.accent_color);

                    for catalog in catalogs {
                        if !catalog.name.is_empty() && prop_bool(props, "show_catalog_header", true) {
                            ui.heading(
                                egui::RichText::new(&catalog.name)
                                    .size(16.0 * ctx.scale)
                                    .color(text_color),
                            );
                            ui.add_space(4.0);
                        }

                        let avail_width = ui.available_width();
                        let cols = ((avail_width / (card_w + 12.0)).floor() as usize).max(1);

                        egui::Grid::new(format!("grid_{}", catalog.id))
                            .spacing([12.0 * ctx.scale, 14.0 * ctx.scale])
                            .show(ui, |ui| {
                                for (idx, item) in catalog.items.iter().enumerate() {
                                    ui.vertical(|ui| {
                                        let (rect, response) = ui.allocate_exact_size(
                                            egui::vec2(card_w, card_h),
                                            egui::Sense::click(),
                                        );

                                        // Card Background
                                        ui.painter().rect_filled(rect, rounding, card_fill);

                                        // Poster placeholder
                                        let poster_rect = egui::Rect::from_min_size(
                                            rect.min,
                                            egui::vec2(card_w, card_h - 45.0 * ctx.scale),
                                        );
                                        ui.painter().rect_filled(
                                            poster_rect,
                                            rounding,
                                            egui::Color32::from_rgb(20, 20, 26),
                                        );

                                        // Title
                                        if show_title {
                                            let text_rect = egui::Rect::from_min_size(
                                                egui::pos2(rect.min.x + 6.0, rect.max.y - 40.0 * ctx.scale),
                                                egui::vec2(card_w - 12.0, 18.0 * ctx.scale),
                                            );
                                            ui.painter().text(
                                                text_rect.min,
                                                egui::Align2::LEFT_TOP,
                                                &item.title,
                                                egui::FontId::proportional(12.0 * ctx.scale),
                                                text_color,
                                            );
                                        }

                                        // Year & Rating
                                        let meta_y = rect.max.y - 18.0 * ctx.scale;
                                        if show_year {
                                            if let Some(year) = item.year {
                                                ui.painter().text(
                                                    egui::pos2(rect.min.x + 6.0, meta_y),
                                                    egui::Align2::LEFT_TOP,
                                                    format!("{}", year),
                                                    egui::FontId::proportional(11.0 * ctx.scale),
                                                    secondary,
                                                );
                                            }
                                        }
                                        if show_rating {
                                            if let Some(rating) = item.rating {
                                                ui.painter().text(
                                                    egui::pos2(rect.max.x - 6.0, meta_y),
                                                    egui::Align2::RIGHT_TOP,
                                                    format!("★ {:.1}", rating),
                                                    egui::FontId::proportional(11.0 * ctx.scale),
                                                    accent,
                                                );
                                            }
                                        }

                                        if response.clicked() {
                                            let store = ctx.store.clone();
                                            let plugin_id = item.plugin_id.clone();
                                            let item_id = item.id.clone();
                                            ctx.rt.block_on(async {
                                                store.dispatch(Intent::GetDetails { plugin_id, item_id }).await;
                                            });
                                        }
                                    });

                                    if (idx + 1) % cols == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                    }
                }
                CatalogState::Idle => {
                    ui.label(
                        egui::RichText::new("No catalog loaded")
                            .color(ctx.config.secondary_color)
                            .italics(),
                    );
                }
            }
        }),
    );
}
