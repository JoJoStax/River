use crate::data_exports::DataContext;
use crate::plugin_ui_core::{
    draw_device_switcher, draw_theme_switcher_horizontal, draw_theme_switcher_vertical, UiNode,
    UiThemeConfig,
};
use crate::ui_backgrounds::draw_complex_background;
use crate::ui_plugin::UiPluginManager;
use eframe::egui;
use river_core::MediaCategory;
use river_presentation::{AppState, AppStore, CatalogState, Intent};
use std::sync::Arc;

// ============================================================================
// RIVER KDL UI RENDERER ENGINE
// ============================================================================
// This module serves as the universal renderer for River's UI system.
// It translates abstract KDL AST layout nodes (UiNode) into immediate-mode
// egui widgets. It is designed to be 100% generic — the renderer knows NOTHING
// about media categories, catalogs, or card styles.
//
// All domain-specific UI is composed by KDL plugins using primitive building
// blocks (row, column, box, grid, for-each, if-state, button, label, etc.)
// combined with data bindings ({categories}, {catalog.items}, {item.title}).
//
// The renderer is 100% shared between:
// 1. Dynamic Hotplug Mode: Evaluates AST nodes on the fly at runtime.
// 2. Core Compiled Mode: Runs pre-parsed/AOT-optimized AST layouts.
//
// Key Responsibilities:
// - Responsive layout scaling based on window dimensions and device aspect ratios.
// - Safe area margin handling for mobile devices (camera notches & gesture bars).
// - Recursive DOM tree walking for containers (row/column/box/scroll) & widgets.
// - Generic grid layout with responsive column calculation.
// - Data-bound iteration (for-each) and conditional rendering (if-state).
// - Text interpolation for {binding} placeholders.
// ============================================================================

/// Common entry point for rendering any KDL UI layout (both Hotplug Dynamic and Core Compiled modes).
///
/// This function sets up the root presentation context:
/// - Requests continuous repaints if background animations are active.
/// - Computes responsive DPI scaling based on screen width (`0.60` to `1.0`).
/// - Resolves the target device layout (`desktop`, `mobile`, `tv`) by checking window aspect ratio.
/// - Builds the DataContext from current AppState for data binding resolution.
/// - Renders background effects before walking the AST panel tree.
pub fn render_theme_layout(
    config: &UiThemeConfig,
    ctx: &egui::Context,
    state: &AppState,
    store: &Arc<AppStore>,
    rt: &tokio::runtime::Runtime,
    ui_manager: &mut UiPluginManager,
) {
    let screen_width = ctx.screen_rect().width();
    let screen_height = ctx.screen_rect().height();
    let target = ui_manager.resolve_target_device(screen_width, screen_height).to_string();

    if config.requires_continuous_repaint(&target) {
        ctx.request_repaint();
    }
    let time = ctx.input(|i| i.time);

    // Calculate responsive scale factor based on screen width!
    let scale = (screen_width / 850.0).clamp(0.60, 1.0);

    // Apply custom layout spacing density scaled for current window dimensions!
    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(config.spacing_x * scale, config.spacing_y * scale);
    });

    // Retrieve active AST DOM layout automatically calculated by aspect ratio and device ID!
    let active_nodes = config.get_active_layout(&target);

    // Build data context from current app state — the bridge between domain and UI!
    let data_ctx = crate::data_exports::build_data_context(state, ui_manager, config);

    draw_complex_background(ctx, config, time);

    render_ast_panels(
        active_nodes,
        ctx,
        state,
        store,
        rt,
        config,
        time,
        scale,
        ui_manager,
        &data_ctx,
    );
}

/// Recursively walk top-level AST nodes and render egui window panels with responsive dimensions.
///
/// Panel Positioning & Mobile Safe Area Handling:
/// On Android and mobile devices, edge-to-edge rendering can cause UI elements to collide
/// with hardware cutouts and system navigation bars:
/// - **Top Safe Area (`38.0 pt`)**: Prevents titles, search bars, and tabs from being obscured by
///   camera hole-punch cutouts or top status bars.
/// - **Bottom Safe Area (`24.0 pt`)**: Prevents bottom navigation controls from overlapping with
///   the Android gesture navigation bar / home indicator.
///
/// If a layout omits a top or bottom panel, the safe area margins fall back onto the `central-panel`.
pub fn render_ast_panels(
    nodes: &[UiNode],
    ctx: &egui::Context,
    state: &AppState,
    store: &Arc<AppStore>,
    rt: &tokio::runtime::Runtime,
    config: &UiThemeConfig,
    time: f64,
    scale: f32,
    ui_manager: &mut UiPluginManager,
    data_ctx: &DataContext,
) {
    let dev_lower = ui_manager.device_id.to_lowercase();
    let is_mobile = cfg!(target_os = "android") || dev_lower.contains("android") || dev_lower.contains("phone") || dev_lower.contains("aarch64") || dev_lower.contains("arm64");
    let has_top_panel = nodes.iter().any(|n| matches!(n, UiNode::Panel { kind, .. } if kind == "top-panel"));
    let has_bottom_panel = nodes.iter().any(|n| matches!(n, UiNode::Panel { kind, .. } if kind == "bottom-panel"));

    let top_safe_margin = if is_mobile { 38.0 * scale } else { 8.0 * scale };
    let bottom_safe_margin = if is_mobile { 24.0 * scale } else { 8.0 * scale };

    for node in nodes {
        match node {
            UiNode::Panel {
                kind,
                id,
                size,
                fill,
                children,
                ..
            } => {
                let panel_id = if id.is_empty() { kind } else { id };
                match kind.as_str() {
                    "top-panel" => {
                        let scaled_height = if *size > 0.0 { *size * scale } else { 0.0 };
                        let margin = egui::Margin {
                            left: 12.0 * scale,
                            right: 12.0 * scale,
                            top: top_safe_margin,
                            bottom: 8.0 * scale,
                        };
                        let mut panel = egui::TopBottomPanel::top(egui::Id::new(panel_id))
                            .frame(egui::Frame::none().fill(*fill).inner_margin(margin));
                        if scaled_height > 0.0 {
                            panel = panel.min_height(scaled_height);
                        }
                        panel.show(ctx, |ui| {
                            render_ui_nodes(
                                ui, children, state, store, rt, config, time, scale, ui_manager, data_ctx,
                            );
                        });
                    }
                    "bottom-panel" => {
                        let scaled_height = if *size > 0.0 { *size * scale } else { 0.0 };
                        let margin = egui::Margin {
                            left: 12.0 * scale,
                            right: 12.0 * scale,
                            top: 8.0 * scale,
                            bottom: bottom_safe_margin,
                        };
                        let mut panel = egui::TopBottomPanel::bottom(egui::Id::new(panel_id))
                            .frame(egui::Frame::none().fill(*fill).inner_margin(margin));
                        if scaled_height > 0.0 {
                            panel = panel.min_height(scaled_height);
                        }
                        panel.show(ctx, |ui| {
                            render_ui_nodes(
                                ui, children, state, store, rt, config, time, scale, ui_manager, data_ctx,
                            );
                        });
                    }
                    "left-panel" => {
                        let scaled_width = if *size > 0.0 { *size * scale } else { 200.0 * scale };
                        let panel = egui::SidePanel::left(egui::Id::new(panel_id))
                            .frame(egui::Frame::none().fill(*fill).inner_margin(10.0 * scale))
                            .default_width(scaled_width)
                            .width_range((70.0 * scale)..=(scaled_width * 1.6))
                            .resizable(true);
                        panel.show(ctx, |ui| {
                            render_ui_nodes(
                                ui, children, state, store, rt, config, time, scale, ui_manager, data_ctx,
                            );
                        });
                    }
                    "right-panel" => {
                        let scaled_width = if *size > 0.0 { *size * scale } else { 200.0 * scale };
                        let panel = egui::SidePanel::right(egui::Id::new(panel_id))
                            .frame(egui::Frame::none().fill(*fill).inner_margin(10.0 * scale))
                            .default_width(scaled_width)
                            .width_range((70.0 * scale)..=(scaled_width * 1.6))
                            .resizable(true);
                        panel.show(ctx, |ui| {
                            render_ui_nodes(
                                ui, children, state, store, rt, config, time, scale, ui_manager, data_ctx,
                            );
                        });
                    }
                    "central-panel" | _ => {
                        let c_top = if !has_top_panel && is_mobile { 38.0 * scale } else { 12.0 * scale };
                        let c_bottom = if !has_bottom_panel && is_mobile { 24.0 * scale } else { 12.0 * scale };
                        let margin = egui::Margin {
                            left: 12.0 * scale,
                            right: 12.0 * scale,
                            top: c_top,
                            bottom: c_bottom,
                        };
                        egui::CentralPanel::default()
                            .frame(egui::Frame::none().fill(*fill).inner_margin(margin))
                            .show(ctx, |ui| {
                                render_ui_nodes(
                                    ui, children, state, store, rt, config, time, scale, ui_manager, data_ctx,
                                );
                            });
                    }
                }
            }
            _ => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    render_ui_nodes(
                        ui,
                        std::slice::from_ref(node),
                        state,
                        store,
                        rt,
                        config,
                        time,
                        scale,
                        ui_manager,
                        data_ctx,
                    );
                });
            }
        }
    }
}

/// Recursively walk and evaluate AST layout nodes within an open egui UI container.
///
/// This engine processes these AST node types:
/// 1. **Containers** (`row`, `column`, `box`, `scroll`, and any custom tag name).
/// 2. **Widgets** (`heading`, `label`, `button`, `separator`, `spacer`, `image`/`svg`,
///    `theme-switcher`, `device-switcher`).
/// 3. **Sugar widgets** (`menu-bar`, `catalog-view`) — expanded into ForEach+primitive
///    trees for backward compatibility with existing KDL themes.
/// 4. **Grid** — generic responsive grid layout.
/// 5. **ForEach** — data-bound iteration.
/// 6. **Condition** — conditional visibility based on data bindings.
///
/// All text/action/color attributes support `{binding}` interpolation via the DataContext.
pub fn render_ui_nodes(
    ui: &mut egui::Ui,
    nodes: &[UiNode],
    state: &AppState,
    store: &Arc<AppStore>,
    rt: &tokio::runtime::Runtime,
    config: &UiThemeConfig,
    time: f64,
    scale: f32,
    ui_manager: &mut UiPluginManager,
    data_ctx: &DataContext,
) {
    let font_fam = config.get_font_family();

    for node in nodes {
        match node {
            UiNode::Container {
                kind,
                spacing,
                padding,
                children,
                effect,
                speed,
            } => {
                let active_effect = if effect == "none" || effect.is_empty() { config.animation_effect.as_str() } else { effect.as_str() };
                let active_speed = if *speed > 0.0 { *speed } else if config.animation_speed > 0.0 { config.animation_speed } else { 1.0 };
                let pulse = ((time * active_speed as f64 * 2.0).sin() * 0.5 + 0.5) as f32;

                let offset_y = match active_effect {
                    "float" => ((time * active_speed as f64 * 2.5).sin() * 5.0 * scale as f64) as f32,
                    "bounce" => -(((time * active_speed as f64 * 3.5).sin().abs()) * 7.0 * scale as f64) as f32,
                    _ => 0.0,
                };
                if offset_y.abs() > 0.1 {
                    ui.add_space(offset_y);
                }

                match kind.as_str() {
                "row" => {
                    ui.horizontal_wrapped(|ui| {
                        if *spacing > 0.0 {
                            ui.style_mut().spacing.item_spacing.x = *spacing * scale;
                        }
                        render_ui_nodes(
                            ui, children, state, store, rt, config, time, scale, ui_manager, data_ctx,
                        );
                    });
                }
                "column" => {
                    ui.vertical(|ui| {
                        if *spacing > 0.0 {
                            ui.style_mut().spacing.item_spacing.y = *spacing * scale;
                        }
                        render_ui_nodes(
                            ui, children, state, store, rt, config, time, scale, ui_manager, data_ctx,
                        );
                    });
                }
                "box" => {
                    let mut frame = egui::Frame::none()
                        .fill(config.fill_color)
                        .inner_margin(*padding * scale)
                        .rounding(config.rounding * scale);
                    if active_effect == "glow" {
                        frame = frame.stroke(egui::Stroke::new((1.5 + pulse * 2.5) * scale, crate::ui_backgrounds::lerp_color(config.border_color, config.accent_color, pulse)));
                    } else if active_effect == "pulse" {
                        frame = frame.stroke(egui::Stroke::new((1.0 + pulse * 1.5) * scale, config.border_color));
                    } else if config.border_width > 0.0 {
                        frame = frame.stroke(egui::Stroke::new(config.border_width * scale, config.border_color));
                    }
                    frame.show(ui, |ui| {
                        render_ui_nodes(
                            ui, children, state, store, rt, config, time, scale, ui_manager, data_ctx,
                        );
                    });
                }
                "scroll" => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        render_ui_nodes(
                            ui, children, state, store, rt, config, time, scale, ui_manager, data_ctx,
                        );
                    });
                }
                // Any custom container tag — render as a vertical group
                _ => {
                    ui.vertical(|ui| {
                        if *spacing > 0.0 {
                            ui.style_mut().spacing.item_spacing.y = *spacing * scale;
                        }
                        render_ui_nodes(
                            ui, children, state, store, rt, config, time, scale, ui_manager, data_ctx,
                        );
                    });
                }
                }
            },

            // ── Grid Node ───────────────────────────────────────────────
            UiNode::Grid {
                id,
                columns,
                spacing_x,
                spacing_y,
                min_cell_width,
                fill,
                rounding,
                border_width,
                children,
                effect,
                speed,
            } => {
                render_generic_grid(
                    ui, id, *columns, *spacing_x, *spacing_y, *min_cell_width,
                    *fill, *rounding, *border_width, children,
                    effect, *speed,
                    state, store, rt, config, time, scale, ui_manager, data_ctx,
                );
            }

            // ── ForEach Node ────────────────────────────────────────────
            UiNode::ForEach {
                source,
                item_var,
                template,
                ..
            } => {
                render_for_each(
                    ui, source, item_var, template,
                    state, store, rt, config, time, scale, ui_manager, data_ctx,
                );
            }

            // ── Condition Node ──────────────────────────────────────────
            UiNode::Condition {
                source,
                equals,
                not_equals,
                children,
                else_children,
            } => {
                render_condition(
                    ui, source, equals, not_equals, children, else_children,
                    state, store, rt, config, time, scale, ui_manager, data_ctx,
                );
            }

            // ── Widget Nodes ────────────────────────────────────────────
            UiNode::Widget {
                kind,
                text,
                url,
                action,
                color,
                size,
                height,
                style,
                columns,
                rounding,
                border_width,
                effect,
                speed,
            } => {
                // Resolve data bindings in text and action strings
                let resolved_text = data_ctx.resolve_text(text);
                let resolved_action = data_ctx.resolve_text(action);

                match kind.as_str() {
                "heading" => {
                    let col = resolve_color(color, config, data_ctx);
                    ui.heading(
                        egui::RichText::new(&resolved_text)
                            .size(*size * scale)
                            .family(font_fam.clone())
                            .strong()
                            .color(col),
                    );
                }
                "label" => {
                    let col = resolve_color(color, config, data_ctx);
                    ui.label(
                        egui::RichText::new(&resolved_text)
                            .size(*size * scale)
                            .family(font_fam.clone())
                            .color(col),
                    );
                }
                "button" | "nav-item" => {
                    let col = resolve_color(color, config, data_ctx);
                    let base_size = if *size > 0.0 { *size } else { config.body_size + 2.0 };
                    let mut anim_size = base_size * scale;
                    if effect == "pulse" {
                        let active_speed = if *speed > 0.0 { *speed } else { 1.0 };
                        anim_size += ((time as f32 * active_speed * 3.0).sin() * 2.0) * scale;
                    }
                    let btn_text = egui::RichText::new(&resolved_text)
                        .size(anim_size)
                        .family(font_fam.clone())
                        .color(col);
                    let mut btn = egui::Button::new(btn_text);
                    if *rounding > 0.0 {
                        btn = btn.rounding(egui::Rounding::same(*rounding * scale));
                    } else {
                        btn = btn.rounding(egui::Rounding::same(config.rounding * scale));
                    }
                    if effect == "glow" {
                        let active_speed = if *speed > 0.0 { *speed } else { 1.0 };
                        btn = btn.stroke(egui::Stroke::new(1.5 * scale, crate::ui_backgrounds::lerp_color(config.border_color, config.accent_color, (time as f32 * active_speed * 2.5).sin() * 0.5 + 0.5)));
                    }

                    // Check if this button represents an active state
                    let is_active = check_active_state(&resolved_action, state, ui_manager);
                    if is_active {
                        btn = btn.fill(config.accent_color);
                    }
                    if ui.add(btn).clicked() {
                        if !resolved_action.is_empty() {
                            dispatch_kdl_action(&resolved_action, store, rt, ui_manager, state);
                        }
                    }
                }
                "separator" => {
                    ui.separator();
                }
                "spacer" => {
                    if *size > 0.0 {
                        ui.add_space(*size * scale);
                    } else {
                        ui.add_space((ui.available_width() / 6.0).max(8.0));
                    }
                }
                // ── Sugar: menu-bar ─────────────────────────────────────
                // Backward compatibility: expands into ForEach + buttons
                "menu-bar" => {
                    render_menu_bar_sugar(
                        ui, state, store, rt, config, style, *size, scale, ui_manager, data_ctx,
                    );
                }
                // ── Sugar: catalog-view ─────────────────────────────────
                // Backward compatibility: expands into condition + ForEach + grid
                "catalog-view" => {
                    render_catalog_view_sugar(
                        ui, state, store, rt, config, time, style, *columns, *rounding, *border_width, effect, *speed, scale, ui_manager, data_ctx,
                    );
                }
                "theme-switcher" => {
                    if style == "vertical" {
                        draw_theme_switcher_vertical(
                            ui,
                            ui_manager,
                            config.accent_color,
                            config.text_color,
                            scale,
                        );
                    } else {
                        draw_theme_switcher_horizontal(
                            ui,
                            ui_manager,
                            config.accent_color,
                            config.text_color,
                            scale,
                        );
                    }
                }
                "device-switcher" => {
                    draw_device_switcher(
                        ui,
                        ui_manager,
                        config.accent_color,
                        config.text_color,
                        scale,
                    );
                }
                "image" | "svg" => {
                    let target_url = if !url.is_empty() {
                        data_ctx.resolve_text(url)
                    } else {
                        resolved_text.clone()
                    };
                    if !target_url.is_empty() {
                        let mut img = egui::Image::new(target_url);
                        if *size > 0.0 && *height > 0.0 {
                            img = img.fit_to_exact_size(egui::vec2(*size * scale, *height * scale));
                        } else if *size > 0.0 {
                            img = img.max_width(*size * scale);
                        } else if *height > 0.0 {
                            img = img.max_height(*height * scale);
                        }
                        if *rounding > 0.0 {
                            img = img.rounding(*rounding * scale);
                        }
                        ui.add(img);
                    } else {
                        ui.label(egui::RichText::new("[ 🖼️ Image/SVG: Missing URL ]").color(egui::Color32::RED));
                    }
                }
                _ => {}
            }},
            UiNode::Panel { .. } => {}
        }
    }
}

// ============================================================================
// GENERIC GRID RENDERER
// ============================================================================

/// Render a responsive grid layout. Dynamically calculates column count based on
/// available width and `min_cell_width`, then renders children in grid cells.
fn render_generic_grid(
    ui: &mut egui::Ui,
    id: &str,
    requested_columns: usize,
    spacing_x: f32,
    spacing_y: f32,
    min_cell_width: f32,
    _fill: egui::Color32,
    _rounding: f32,
    _border_width: f32,
    children: &[UiNode],
    _effect: &str,
    _speed: f32,
    state: &AppState,
    store: &Arc<AppStore>,
    rt: &tokio::runtime::Runtime,
    config: &UiThemeConfig,
    time: f64,
    scale: f32,
    ui_manager: &mut UiPluginManager,
    data_ctx: &DataContext,
) {
    let available_width = ui.available_width();
    let scaled_min = (min_cell_width * scale).max(100.0);
    let actual_cols = if requested_columns > 1 {
        ((available_width / scaled_min) as usize).clamp(1, requested_columns)
    } else {
        1
    };

    let cell_width = (available_width - ((actual_cols.saturating_sub(1)) as f32 * (spacing_x * scale))) / (actual_cols as f32);

    let grid_id = if id.is_empty() { "generic_grid" } else { id };
    egui::Grid::new(grid_id)
        .num_columns(actual_cols)
        .spacing([spacing_x * scale, spacing_y * scale])
        .show(ui, |ui| {
            for (idx, child) in children.iter().enumerate() {
                ui.allocate_ui_with_layout(
                    egui::vec2(cell_width, 0.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        render_ui_nodes(
                            ui,
                            std::slice::from_ref(child),
                            state, store, rt, config, time, scale, ui_manager, data_ctx,
                        );
                    },
                );
                if (idx + 1) % actual_cols == 0 {
                    ui.end_row();
                }
            }
        });
}

// ============================================================================
// FOR-EACH RENDERER
// ============================================================================

/// Evaluate a ForEach node: look up the data source list from the DataContext,
/// then for each item create a child DataContext and render the template nodes.
fn render_for_each(
    ui: &mut egui::Ui,
    source: &str,
    _item_var: &str,
    template: &[UiNode],
    state: &AppState,
    store: &Arc<AppStore>,
    rt: &tokio::runtime::Runtime,
    config: &UiThemeConfig,
    time: f64,
    scale: f32,
    ui_manager: &mut UiPluginManager,
    data_ctx: &DataContext,
) {
    // Strip curly braces if present: "{categories}" -> "categories"
    let key = source.trim_start_matches('{').trim_end_matches('}');

    let items = data_ctx.get_list(key);
    if items.is_empty() {
        return;
    }

    for item_ctx in items {
        // Create a child context that layers item bindings on top of parent context.
        // The item's bindings (e.g. "cat.id", "cat.name") are promoted to be accessible
        // directly AND with the item_var prefix (e.g. "item.id" if item_var="item").
        let child_ctx = data_ctx.with_child(item_ctx.bindings.clone());

        render_ui_nodes(
            ui, template, state, store, rt, config, time, scale, ui_manager, &child_ctx,
        );
    }
}

// ============================================================================
// CONDITION RENDERER
// ============================================================================

/// Evaluate a Condition node: resolve the source binding and compare against
/// equals/not_equals. Render children if true, else_children if false.
fn render_condition(
    ui: &mut egui::Ui,
    source: &str,
    equals: &str,
    not_equals: &str,
    children: &[UiNode],
    else_children: &[UiNode],
    state: &AppState,
    store: &Arc<AppStore>,
    rt: &tokio::runtime::Runtime,
    config: &UiThemeConfig,
    time: f64,
    scale: f32,
    ui_manager: &mut UiPluginManager,
    data_ctx: &DataContext,
) {
    let key = source.trim_start_matches('{').trim_end_matches('}');
    let actual_value = data_ctx.get_str(key);

    let condition_met = if !equals.is_empty() {
        actual_value == equals
    } else if !not_equals.is_empty() {
        actual_value != not_equals
    } else {
        // If no condition specified, check truthiness
        !actual_value.is_empty() && actual_value != "false" && actual_value != "0"
    };

    if condition_met {
        render_ui_nodes(
            ui, children, state, store, rt, config, time, scale, ui_manager, data_ctx,
        );
    } else if !else_children.is_empty() {
        render_ui_nodes(
            ui, else_children, state, store, rt, config, time, scale, ui_manager, data_ctx,
        );
    }
}

// ============================================================================
// ACTION DISPATCH
// ============================================================================

fn dispatch_kdl_action(
    action: &str,
    store: &Arc<AppStore>,
    rt: &tokio::runtime::Runtime,
    ui_manager: &mut UiPluginManager,
    state: &AppState,
) {
    if let Some(cat_str) = action.strip_prefix("SelectCategory:") {
        let cat = match cat_str {
            "Video" => MediaCategory::Video,
            "Music" => MediaCategory::Music,
            "Manga" => MediaCategory::Manga,
            "Podcast" | "Podcasts" => MediaCategory::Podcast,
            _ => return,
        };
        if state.selected_category != cat {
            let store_clone = store.clone();
            rt.spawn(async move {
                store_clone.dispatch(Intent::SelectCategory(cat)).await;
                store_clone.dispatch(Intent::LoadCatalogs(cat)).await;
            });
        }
    } else if let Some(theme_id) = action.strip_prefix("SwitchTheme:") {
        ui_manager.switch_to(theme_id);
    } else if let Some(target) = action.strip_prefix("SwitchDevice:") {
        if target == "auto" {
            ui_manager.set_target_override(None);
        } else {
            ui_manager.set_target_override(Some(target));
        }
    } else if let Some(_item_id) = action.strip_prefix("AddToLibrary:") {
        // Dynamic add-to-library: the item ID is resolved from data bindings
        // For now this is a placeholder — full implementation needs item lookup from catalogs
        if let CatalogState::Loaded(catalogs) = &state.catalog_state {
            for catalog in catalogs {
                if let Some(item) = catalog.items.iter().find(|i| i.id == _item_id) {
                    let store_clone = store.clone();
                    let item_clone = item.clone();
                    rt.spawn(async move {
                        store_clone.dispatch(Intent::AddToLibrary(item_clone)).await;
                    });
                    return;
                }
            }
        }
    }
}

/// Check if an action string represents a currently-active state (for button highlighting).
fn check_active_state(action: &str, state: &AppState, ui_manager: &UiPluginManager) -> bool {
    if let Some(cat_str) = action.strip_prefix("SelectCategory:") {
        matches!(
            (cat_str, state.selected_category),
            ("Video", MediaCategory::Video)
            | ("Music", MediaCategory::Music)
            | ("Manga", MediaCategory::Manga)
            | ("Podcast" | "Podcasts", MediaCategory::Podcast)
        )
    } else if let Some(theme_id) = action.strip_prefix("SwitchTheme:") {
        ui_manager.active_plugin().map(|p| p.id() == theme_id).unwrap_or(false)
    } else {
        false
    }
}

// ============================================================================
// COLOR RESOLUTION
// ============================================================================

/// Map semantic color token strings from KDL layouts to concrete `Color32` theme values.
///
/// Supported tokens:
/// - `"accent"` -> `config.accent_color`
/// - `"secondary"` -> `config.secondary_color`
/// - `"border"` -> `config.border_color`
/// - `"text"` / default -> `config.text_color`
/// - `"#rrggbb"` / `"#rrggbbaa"` -> parsed hex color (direct specification)
/// - `"{binding}"` -> resolved from DataContext
fn resolve_color(color: &str, config: &UiThemeConfig, data_ctx: &DataContext) -> egui::Color32 {
    // Check for data binding first
    let resolved = if color.contains('{') {
        data_ctx.resolve_text(color)
    } else {
        color.to_string()
    };

    match resolved.as_str() {
        "accent" => config.accent_color,
        "secondary" => config.secondary_color,
        "text" => config.text_color,
        "border" => config.border_color,
        hex if hex.starts_with('#') => {
            parse_inline_hex(hex).unwrap_or(config.text_color)
        }
        _ => config.text_color,
    }
}

/// Parse a `#rrggbb` or `#rrggbbaa` hex color string.
fn parse_inline_hex(hex: &str) -> Option<egui::Color32> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 8 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
        Some(egui::Color32::from_rgba_unmultiplied(r, g, b, a))
    } else if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(egui::Color32::from_rgb(r, g, b))
    } else {
        None
    }
}

// ============================================================================
// SUGAR EXPANSION: menu-bar
// ============================================================================
// Backward compatibility layer. When existing KDL themes use `menu-bar`,
// we expand it into category navigation buttons using data from the DataContext.
// New themes should use `for-each source="{categories}" ...` directly.

/// Render category navigation buttons. This is the sugar expansion of the
/// legacy `menu-bar` widget — it reads categories from the DataContext
/// and renders them as buttons with the specified style.
fn render_menu_bar_sugar(
    ui: &mut egui::Ui,
    state: &AppState,
    store: &Arc<AppStore>,
    rt: &tokio::runtime::Runtime,
    config: &UiThemeConfig,
    style: &str,
    font_size: f32,
    scale: f32,
    _ui_manager: &mut UiPluginManager,
    _data_ctx: &DataContext,
) {
    let font_fam = config.get_font_family();
    let categories = [
        MediaCategory::Video,
        MediaCategory::Music,
        MediaCategory::Manga,
        MediaCategory::Podcast,
    ];

    if style == "vertical" || style == "sidebar" || style == "m3_sidebar" {
        ui.vertical(|ui| {
            for &cat in &categories {
                let (default_name, icon_tag) = match cat {
                    MediaCategory::Video => ("Video", "🎬"),
                    MediaCategory::Music => ("Music", "🎵"),
                    MediaCategory::Manga => ("Manga", "📖"),
                    MediaCategory::Podcast => ("Podcasts", "🎙️"),
                };
                let label = format!("{}   {}", icon_tag, default_name);
                let is_selected = state.selected_category == cat;
                let base_size = if font_size > 0.0 { font_size } else { config.body_size + 4.0 };
                let size = base_size * scale;
                let text_color = if is_selected {
                    config.fill_color
                } else {
                    config.text_color
                };
                let btn_text = egui::RichText::new(label)
                    .size(size)
                    .family(font_fam.clone())
                    .color(text_color);

                let mut btn = egui::Button::new(btn_text)
                    .rounding(egui::Rounding::same(14.0 * scale))
                    .min_size(egui::vec2(ui.available_width(), 42.0 * scale));
                if is_selected {
                    btn = btn.fill(config.accent_color);
                }

                if ui.add(btn).clicked() && !is_selected {
                    let store_clone = store.clone();
                    rt.spawn(async move {
                        store_clone.dispatch(Intent::SelectCategory(cat)).await;
                        store_clone.dispatch(Intent::LoadCatalogs(cat)).await;
                    });
                }
                ui.add_space(8.0 * scale);
            }
        });
        return;
    }

    ui.horizontal_wrapped(|ui| {
        for &cat in &categories {
            let (default_name, icon_tag) = match cat {
                MediaCategory::Video => ("Video", "🎬"),
                MediaCategory::Music => ("Music", "🎵"),
                MediaCategory::Manga => ("Manga", "📖"),
                MediaCategory::Podcast => ("Podcasts", "🎙️"),
            };

            let label = match style {
                "icons" | "m3_icons" => icon_tag.to_string(),
                "pills" => format!("• {} •", default_name),
                "brackets" => format!("[ {} ]", default_name),
                _ => format!("{} {}", icon_tag, default_name),
              };

            let is_selected = state.selected_category == cat;
            let base_size = if font_size > 0.0 { font_size } else { config.body_size + 6.0 };
            let size = base_size * scale;
            let text_color = if is_selected && (style == "icons" || style == "m3_icons" || style == "pills") {
                config.fill_color
            } else if is_selected {
                config.accent_color
            } else {
                config.text_color
            };
            let btn_text = egui::RichText::new(label)
                .size(size)
                .family(font_fam.clone())
                .color(text_color);

            let mut btn = egui::Button::new(btn_text);
            if style == "icons" || style == "m3_icons" || style == "pills" {
                btn = btn.rounding(egui::Rounding::same(24.0 * scale)).min_size(egui::vec2(60.0 * scale, 40.0 * scale));
                if is_selected {
                    btn = btn.fill(config.accent_color);
                }
            }

            if ui.add(btn).clicked() && !is_selected {
                let store_clone = store.clone();
                rt.spawn(async move {
                    store_clone.dispatch(Intent::SelectCategory(cat)).await;
                    store_clone.dispatch(Intent::LoadCatalogs(cat)).await;
                });
            }
            ui.add_space(6.0 * scale);
        }
    });
}

// ============================================================================
// SUGAR EXPANSION: catalog-view
// ============================================================================
// Backward compatibility layer. When existing KDL themes use `catalog-view`,
// we expand it into catalog rendering using data from the DataContext.
// New themes should use `if-state`, `for-each`, and `grid` directly.

/// Helper to eliminate duplicate async dispatch boilerplate across all catalog card layouts.
fn dispatch_add_to_library(store: &Arc<AppStore>, rt: &tokio::runtime::Runtime, item: &river_core::MediaItem) {
    let store_clone = store.clone();
    let item_clone = item.clone();
    rt.spawn(async move {
        store_clone.dispatch(Intent::AddToLibrary(item_clone)).await;
    });
}

/// Render media catalog items using responsive fluid grids and themed card layouts.
/// This is the sugar expansion of the legacy `catalog-view` widget.
fn render_catalog_view_sugar(
    ui: &mut egui::Ui,
    state: &AppState,
    store: &Arc<AppStore>,
    rt: &tokio::runtime::Runtime,
    config: &UiThemeConfig,
    time: f64,
    layout_style: &str,
    requested_columns: usize,
    rounding: f32,
    border_width: f32,
    effect: &str,
    speed: f32,
    scale: f32,
    _ui_manager: &mut UiPluginManager,
    _data_ctx: &DataContext,
) {
    let font_fam = config.get_font_family();

    match &state.catalog_state {
        CatalogState::Loading => {
            ui.horizontal_wrapped(|ui| {
                ui.spinner();
                ui.label(
                    egui::RichText::new("AST Engine evaluating DOM layout nodes... scanning media...")
                        .size(config.body_size * scale)
                        .family(font_fam.clone())
                        .color(config.secondary_color),
                );
            });
        }
        CatalogState::Loaded(catalogs) => {
            if catalogs.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0 * scale);
                    ui.label(
                        egui::RichText::new("🌊 No Active Catalogs Found")
                            .size(20.0 * scale)
                            .family(font_fam.clone())
                            .color(config.accent_color)
                            .strong(),
                    );
                    ui.add_space(10.0 * scale);
                    ui.label(
                        egui::RichText::new("Please select a media category above or check plugin settings.")
                            .size(config.body_size * scale)
                            .family(font_fam.clone())
                            .color(config.text_color),
                    );
                });
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for cat in catalogs {
                        ui.group(|ui| {
                            ui.heading(
                                egui::RichText::new(format!("// {}", cat.name))
                                    .size((config.header_size - 4.0) * scale)
                                    .family(font_fam.clone())
                                    .color(config.accent_color),
                            );
                            ui.add_space(8.0 * scale);

                            // RESPONSIVE COLUMN CALCULATION!
                            let available_width = ui.available_width();
                            let min_card_width = match layout_style {
                                "live_tiles" | "channel_grid" => (190.0 * scale).max(130.0),
                                "achievement_cards" | "terminal_feed" => (240.0 * scale).max(160.0),
                                _ => (180.0 * scale).max(120.0),
                            };
                            let actual_cols = if requested_columns > 1 {
                                ((available_width / min_card_width) as usize).clamp(1, requested_columns)
                            } else {
                                1
                            };

                            let cell_width = (available_width - ((actual_cols.saturating_sub(1)) as f32 * (config.spacing_x * scale))) / (actual_cols as f32);

                            egui::Grid::new(format!("grid_{}", cat.name))
                                .num_columns(actual_cols)
                                .spacing([config.spacing_x * scale, config.spacing_y * scale])
                                .show(ui, |ui| {
                                    for (idx, item) in cat.items.iter().enumerate() {
                                        let phase = idx as f64 * 0.4;
                                        let card_pulse = ((time * speed as f64 * 2.0 + phase).sin() * 0.5 + 0.5) as f32;
                                        let card_wave = ((time * speed as f64 * 1.5 + phase).sin() * 0.5 + 0.5) as f32;

                                        let stroke = match effect {
                                            "pulse" => egui::Stroke::new((border_width + card_pulse * 2.0) * scale, config.border_color),
                                            "glow" => egui::Stroke::new((border_width.max(1.5) + card_pulse * 3.0) * scale, crate::ui_backgrounds::lerp_color(config.border_color, config.accent_color, card_pulse)),
                                            "shimmer" => egui::Stroke::new((border_width.max(1.0) + card_wave * 1.5) * scale, crate::ui_backgrounds::lerp_color(config.border_color, config.secondary_color, card_wave)),
                                            _ => egui::Stroke::new(border_width * scale, config.border_color),
                                        };

                                        ui.allocate_ui_with_layout(
                                            egui::vec2(cell_width, 0.0),
                                            egui::Layout::top_down(egui::Align::LEFT),
                                            |ui| {
                                                // Default card style for all sugar layouts
                                                egui::Frame::none()
                                                    .rounding(rounding * scale)
                                                    .stroke(stroke)
                                                    .fill(config.fill_color)
                                                    .inner_margin((10.0 + card_pulse * if effect == "pulse" { 1.5 } else { 0.0 }) * scale)
                                                    .show(ui, |ui| {
                                                        ui.horizontal_wrapped(|ui| {
                                                            ui.label(
                                                                egui::RichText::new("►")
                                                                    .size(12.0 * scale)
                                                                    .color(config.accent_color),
                                                            );
                                                            ui.strong(
                                                                egui::RichText::new(&item.title)
                                                                    .size((config.body_size + 1.0) * scale)
                                                                    .family(font_fam.clone())
                                                                    .color(config.text_color),
                                                            );
                                                            ui.with_layout(
                                                                egui::Layout::right_to_left(
                                                                    egui::Align::Center,
                                                                ),
                                                                |ui| {
                                                                    if ui.button(egui::RichText::new("⚡ LINK").size(11.0 * scale)).clicked()
                                                                    {
                                                                        dispatch_add_to_library(store, rt, item);
                                                                    }
                                                                },
                                                            );
                                                        });
                                                    });
                                            },
                                        );

                                        if (idx + 1) % actual_cols == 0 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        });
                        ui.add_space(14.0 * scale);
                    }
                });
            }
        }
        CatalogState::Error(e) => {
            ui.colored_label(egui::Color32::RED, format!("AST Engine Error: {}", e));
        }
        CatalogState::Idle => {
            ui.label(
                egui::RichText::new("AST Engine Idle. Select a media category above.")
                    .size(config.body_size * scale)
                    .family(font_fam.clone()),
            );
        }
    }
}
