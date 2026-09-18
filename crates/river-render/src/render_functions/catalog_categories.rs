use super::*;
use eframe::egui;
use river_core::MediaCategory;
use river_presentation::{CatalogState, Intent};
use std::sync::Arc;

/// Dynamic Plugin-Driven Categories:
/// Categories are created dynamically by plugins (e.g. "Anime", "Movies", "Podcasts", etc.).
/// There are NO hardcoded categories or emojis — whatever categories plugins expose are displayed.
/// A user can have many categories or just a single category.
pub fn register(registry: &mut RenderFunctionRegistry) {
    registry.register(
        "catalog:categories",
        Arc::new(|ui, ctx, props, _children| {
            struct DynamicCategory {
                id: String,
                name: String,
                icon: Option<String>,
                backend_cat: Option<MediaCategory>,
            }

            let mut categories: Vec<DynamicCategory> = Vec::new();

            // 1. Collect categories created by loaded plugin catalogs
            if let CatalogState::Loaded(catalogs) = &ctx.state.catalog_state {
                for catalog in catalogs {
                    if !catalog.name.is_empty() && !categories.iter().any(|c| c.name == catalog.name) {
                        categories.push(DynamicCategory {
                            id: catalog.id.clone(),
                            name: catalog.name.clone(),
                            icon: None,
                            backend_cat: Some(catalog.category),
                        });
                    }
                }
            }

            // 2. Collect categories from installed plugins
            for plugin in &ctx.state.plugins {
                if !categories.iter().any(|c| c.id == plugin.id.0 || c.name == plugin.name) {
                    categories.push(DynamicCategory {
                        id: plugin.id.0.clone(),
                        name: plugin.name.clone(),
                        icon: plugin.icon_url.clone(),
                        backend_cat: plugin.supported_categories.first().copied(),
                    });
                }
            }

            // If user has only 1 category and theme opts to hide single categories
            if categories.len() <= 1 && prop_bool(props, "hide_if_single", false) {
                return;
            }

            let active_color = prop_color(props, "active_color", ctx.config, ctx.config.accent_color);
            let inactive_color = prop_color(props, "color", ctx.config, ctx.config.text_color);
            let rounding = prop_f32(props, "rounding", ctx.config.rounding);
            let size = prop_f32(props, "size", 13.0) * ctx.scale;

            ui.horizontal_wrapped(|ui| {
                for cat in categories {
                    let is_active = cat.backend_cat.map_or(false, |bc| ctx.state.selected_category == bc);

                    let label = if let Some(icon) = &cat.icon {
                        format!("{} {}", icon, cat.name)
                    } else {
                        cat.name.clone()
                    };

                    let mut btn = egui::Button::new(
                        egui::RichText::new(label)
                            .size(size)
                            .color(if is_active { active_color } else { inactive_color })
                            .strong(),
                    )
                    .rounding(rounding);

                    if is_active && props.contains_key("active_fill") {
                        btn = btn.fill(prop_color(props, "active_fill", ctx.config, ctx.config.accent_color));
                    } else if props.contains_key("fill") {
                        btn = btn.fill(prop_color(props, "fill", ctx.config, ctx.config.fill_color));
                    }

                    if ui.add(btn).clicked() {
                        if let Some(backend_cat) = cat.backend_cat {
                            let store = ctx.store.clone();
                            ctx.rt.block_on(async {
                                store.dispatch(Intent::LoadCatalogs(backend_cat)).await;
                            });
                        }
                    }
                }
            });
        }),
    );
}
