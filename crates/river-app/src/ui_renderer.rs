use crate::plugin_ui_core::{
    draw_device_switcher, draw_theme_switcher_horizontal, draw_theme_switcher_vertical, UiNode,
    UiThemeConfig,
};
use crate::ui_backgrounds::draw_complex_background;
use crate::ui_plugin::UiPluginManager;
use eframe::egui;
use river_core::{MediaCategory, MediaItem};
use river_presentation::{AppState, AppStore, CatalogState, Intent};
use std::sync::Arc;

// ============================================================================
// RIVER KDL UI RENDERER ENGINE
// ============================================================================
// This module serves as the universal renderer for River's UI system.
// It translates abstract KDL AST layout nodes (UiNode) into immediate-mode
// egui widgets. It is designed to be 100% shared between:
// 1. Dynamic Hotplug Mode: Evaluates AST nodes on the fly at runtime.
// 2. Core Compiled Mode: Runs pre-parsed/AOT-optimized AST layouts.
//
// Key Responsibilities:
// - Responsive layout scaling based on window dimensions and device aspect ratios.
// - Safe area margin handling for mobile devices (camera notches & gesture bars).
// - Recursive DOM tree walking for containers (row/column/box/scroll) & widgets.
// - Fluid responsive grid calculations for media catalog presentation.
// ============================================================================

/// Common entry point for rendering any KDL UI layout (both Hotplug Dynamic and Core Compiled modes).
///
/// This function sets up the root presentation context:
/// - Requests continuous repaints if background animations are active.
/// - Computes responsive DPI scaling based on screen width (`0.60` to `1.0`).
/// - Resolves the target device layout (`desktop`, `mobile`, `tv`) by checking window aspect ratio.
/// - Renders background effects before walking the AST panel tree.
pub fn render_theme_layout(
    config: &UiThemeConfig,
    ctx: &egui::Context,
    state: &AppState,
    store: &Arc<AppStore>,
    rt: &tokio::runtime::Runtime,
    ui_manager: &mut UiPluginManager,
) {
    if config.animation_effect != "none" {
        ctx.request_repaint();
    }
    let time = ctx.input(|i| i.time);

    // Calculate responsive scale factor based on screen width!
    let screen_width = ctx.screen_rect().width();
    let scale = (screen_width / 850.0).clamp(0.60, 1.0);

    // Apply custom layout spacing density scaled for current window dimensions!
    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(config.spacing_x * scale, config.spacing_y * scale);
    });

    // Retrieve active AST DOM layout automatically calculated by aspect ratio and device ID!
    let screen_height = ctx.screen_rect().height();
    let target = ui_manager.resolve_target_device(screen_width, screen_height).to_string();
    let active_nodes = config.get_active_layout(&target);

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
                                ui, children, state, store, rt, config, time, scale, ui_manager,
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
                                ui, children, state, store, rt, config, time, scale, ui_manager,
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
                                ui, children, state, store, rt, config, time, scale, ui_manager,
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
                                ui, children, state, store, rt, config, time, scale, ui_manager,
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
                                    ui, children, state, store, rt, config, time, scale, ui_manager,
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
                    );
                });
            }
        }
    }
}

/// Recursively walk and evaluate AST layout nodes within an open egui UI container.
///
/// This engine processes two primary AST node types:
/// 1. **Containers (`UiNode::Container`)**:
///    - `row`: Renders children horizontally with automatic line wrapping.
///    - `column`: Renders children vertically with themed spacing.
///    - `box`: Wraps children in an styled background frame with padding and corner rounding.
///    - `scroll`: Provides vertical scrollability for overflow content.
///
/// 2. **Widgets (`UiNode::Widget`)**:
///    - Basic UI primitives: `heading`, `label`, `button`, `separator`, `spacer`.
///    - Specialized components: `menu-bar` (category nav), `catalog-view` (media grids),
///      `theme-switcher`, `device-switcher`, and network `image` / `svg` rendering.
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
) {
    let font_fam = config.get_font_family();

    for node in nodes {
        match node {
            UiNode::Container {
                kind,
                spacing,
                padding,
                children,
            } => match kind.as_str() {
                "row" => {
                    ui.horizontal_wrapped(|ui| {
                        if *spacing > 0.0 {
                            ui.style_mut().spacing.item_spacing.x = *spacing * scale;
                        }
                        render_ui_nodes(
                            ui, children, state, store, rt, config, time, scale, ui_manager,
                        );
                    });
                }
                "column" => {
                    ui.vertical(|ui| {
                        if *spacing > 0.0 {
                            ui.style_mut().spacing.item_spacing.y = *spacing * scale;
                        }
                        render_ui_nodes(
                            ui, children, state, store, rt, config, time, scale, ui_manager,
                        );
                    });
                }
                "box" => {
                    egui::Frame::none()
                        .fill(config.fill_color)
                        .inner_margin(*padding * scale)
                        .rounding(config.rounding * scale)
                        .show(ui, |ui| {
                            render_ui_nodes(
                                ui, children, state, store, rt, config, time, scale, ui_manager,
                            );
                        });
                }
                "scroll" | _ => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        render_ui_nodes(
                            ui, children, state, store, rt, config, time, scale, ui_manager,
                        );
                    });
                }
            },
            UiNode::Widget {
                kind,
                text,
                url,
                color,
                size,
                height,
                style,
                columns,
                rounding,
                border_width,
                effect,
            } => match kind.as_str() {
                "heading" => {
                    let col = resolve_color(color, config);
                    ui.heading(
                        egui::RichText::new(text)
                            .size(*size * scale)
                            .family(font_fam.clone())
                            .strong()
                            .color(col),
                    );
                }
                "label" => {
                    let col = resolve_color(color, config);
                    ui.label(
                        egui::RichText::new(text)
                            .size(*size * scale)
                            .family(font_fam.clone())
                            .color(col),
                    );
                }
                "button" => {
                    let col = resolve_color(color, config);
                    let btn_text = egui::RichText::new(text).size(*size * scale).color(col);
                    let mut btn = egui::Button::new(btn_text);
                    if *rounding > 0.0 {
                        btn = btn.rounding(egui::Rounding::same(*rounding * scale));
                    } else {
                        btn = btn.rounding(egui::Rounding::same(24.0 * scale));
                    }
                    if ui.add(btn).clicked() {
                        // Custom action trigger
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
                "menu-bar" => {
                    render_category_nav(ui, state, store, rt, config, style, *size, scale);
                }
                "catalog-view" => {
                    render_catalog_content(
                        ui, state, store, rt, config, time, style, *columns, *rounding, *border_width, effect, scale,
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
                        url.as_str()
                    } else {
                        text.as_str()
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
            },
            UiNode::Panel { .. } => {}
        }
    }
}

/// Map semantic color token strings from KDL layouts to concrete `Color32` theme values.
///
/// Supported tokens:
/// - `"accent"` -> `config.accent_color` (Primary interactive highlights, active tabs, buttons)
/// - `"secondary"` -> `config.secondary_color` (Subtitles, secondary labels, spinners)
/// - `"border"` -> `config.border_color` (Card outlines, dividers, terminal frames)
/// - `"text"` / default -> `config.text_color` (Body text, standard labels)
fn resolve_color(color: &str, config: &UiThemeConfig) -> egui::Color32 {
    match color {
        "accent" => config.accent_color,
        "secondary" => config.secondary_color,
        "text" => config.text_color,
        "border" => config.border_color,
        _ => config.text_color,
    }
}

/// Render media category navigation buttons (`Video`, `Music`, `Manga`, `Podcasts`).
///
/// **IMMUTABLE ARCHITECTURAL RULE**:
/// Core category names (`Video`, `Music`, `Manga`, `Podcasts`) are strictly enforced and can
/// **never** be renamed or overridden by themes. Themes may only alter their visual presentation
/// style (`icons`, `pills`, `brackets`, or standard text).
///
/// User interaction dispatches asynchronous `SelectCategory` and `LoadCatalogs` intents to the
/// MVI store without blocking the UI thread.
fn render_category_nav(
    ui: &mut egui::Ui,
    state: &AppState,
    store: &Arc<AppStore>,
    rt: &tokio::runtime::Runtime,
    config: &UiThemeConfig,
    style: &str,
    font_size: f32,
    scale: f32,
) {
    let font_fam = config.get_font_family();
    let categories = [
        MediaCategory::Video,
        MediaCategory::Music,
        MediaCategory::Manga,
        MediaCategory::Podcast,
    ];

    ui.horizontal_wrapped(|ui| {
        for &cat in &categories {
            let (default_name, icon_tag) = match cat {
                MediaCategory::Video => ("Video", "🎬"),
                MediaCategory::Music => ("Music", "🎵"),
                MediaCategory::Manga => ("Manga", "📖"),
                MediaCategory::Podcast => ("Podcasts", "🎙️"),
            };

            // UI themes can choose visual styling/icons, but can NEVER switch or rename core category names!
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

/// Helper to eliminate duplicate async dispatch boilerplate across all catalog card layouts.
/// Spawns a Tokio background task to send `Intent::AddToLibrary` to the MVI store.
fn dispatch_add_to_library(store: &Arc<AppStore>, rt: &tokio::runtime::Runtime, item: &MediaItem) {
    let store_clone = store.clone();
    let item_clone = item.clone();
    rt.spawn(async move {
        store_clone.dispatch(Intent::AddToLibrary(item_clone)).await;
    });
}

/// Render Media Catalog items using responsive fluid grids and themed card layouts.
///
/// **Responsive Grid Calculation**:
/// - Calculates available width and dynamically divides it by `min_card_width` (derived from card style).
/// - Clamps column count between `1` and `requested_columns` to prevent horizontal clipping.
/// - Calculates precise cell widths accounting for horizontal item spacing.
///
/// **Supported Card Styles**:
/// - `live_tiles`: Windows Phone / Metro style vibrant colored tiles (blue, magenta, green).
/// - `achievement_cards`: Xbox / Gaming style achievement unlock banners with gamerscore badges.
/// - `channel_grid`: TV broadcast channel cards with touch activation buttons.
/// - `terminal_feed`: Cyberpunk / command-line feed rows with monospace font styling.
/// - `default`: Minimalist media rows with play buttons and link triggers.
fn render_catalog_content(
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
    scale: f32,
) {
    let pulse = ((time * config.animation_speed as f64).sin() * 0.5 + 0.5) as f32;
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
                                        let stroke_width = if effect == "pulse" {
                                            (border_width + pulse * 1.5) * scale
                                        } else {
                                            border_width * scale
                                        };

                                        ui.allocate_ui_with_layout(
                                            egui::vec2(cell_width, 0.0),
                                            egui::Layout::top_down(egui::Align::LEFT),
                                            |ui| {
                                                match layout_style {
                                                    "live_tiles" => {
                                                        let tile_color = if idx % 3 == 0 {
                                                            egui::Color32::from_rgb(0, 164, 239)
                                                        } else if idx % 3 == 1 {
                                                            egui::Color32::from_rgb(255, 0, 151)
                                                        } else {
                                                            egui::Color32::from_rgb(0, 138, 0)
                                                        };

                                                        egui::Frame::none()
                                                            .rounding(rounding * scale)
                                                            .fill(tile_color)
                                                            .inner_margin(12.0 * scale)
                                                            .show(ui, |ui| {
                                                                let response = ui.allocate_response(
                                                                    egui::vec2(ui.available_width(), 85.0 * scale),
                                                                    egui::Sense::click(),
                                                                );
                                                                if response.clicked() {
                                                                    dispatch_add_to_library(store, rt, item);
                                                                }
                                                                if response.hovered() {
                                                                    ui.ctx().set_cursor_icon(
                                                                        egui::CursorIcon::PointingHand,
                                                                    );
                                                                }

                                                                ui.horizontal(|ui| {
                                                                    ui.label(
                                                                        egui::RichText::new(" TILE")
                                                                            .size(10.0 * scale)
                                                                            .color(egui::Color32::WHITE),
                                                                    );
                                                                    ui.with_layout(
                                                                        egui::Layout::right_to_left(
                                                                            egui::Align::TOP,
                                                                        ),
                                                                        |ui| {
                                                                            ui.label(
                                                                                egui::RichText::new("◉")
                                                                                    .size(11.0 * scale)
                                                                                    .color(egui::Color32::WHITE),
                                                                            );
                                                                        },
                                                                    );
                                                                });
                                                                ui.add_space(8.0 * scale);

                                                                ui.strong(
                                                                    egui::RichText::new(&item.title)
                                                                        .size((config.body_size + 2.0) * scale)
                                                                        .family(font_fam.clone())
                                                                        .color(egui::Color32::WHITE),
                                                                );
                                                            });
                                                    }
                                                    "achievement_cards" => {
                                                        egui::Frame::none()
                                                            .rounding(rounding * scale)
                                                            .stroke(egui::Stroke::new(
                                                                stroke_width,
                                                                config.border_color,
                                                            ))
                                                            .fill(config.fill_color)
                                                            .inner_margin(12.0 * scale)
                                                            .show(ui, |ui| {
                                                                ui.horizontal_wrapped(|ui| {
                                                                    ui.label(
                                                                        egui::RichText::new(format!(
                                                                            "[ {:02}0G ]",
                                                                            idx + 1
                                                                        ))
                                                                        .size(11.0 * scale)
                                                                        .strong()
                                                                        .color(egui::Color32::from_rgb(
                                                                            255, 180, 0,
                                                                        )),
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
                                                                            if ui
                                                                                .add(
                                                                                    egui::Button::new(
                                                                                        egui::RichText::new(
                                                                                            "⮞ PLAY",
                                                                                        )
                                                                                        .size(11.0 * scale)
                                                                                        .strong()
                                                                                        .color(
                                                                                            egui::Color32::WHITE,
                                                                                        ),
                                                                                    )
                                                                                    .fill(config.accent_color),
                                                                                )
                                                                                .clicked()
                                                                            {
                                                                                dispatch_add_to_library(store, rt, item);
                                                                            }
                                                                        },
                                                                    );
                                                                });
                                                            });
                                                    }
                                                    "channel_grid" => {
                                                        egui::Frame::none()
                                                            .rounding(rounding * scale)
                                                            .stroke(egui::Stroke::new(
                                                                stroke_width,
                                                                config.border_color,
                                                            ))
                                                            .fill(config.fill_color)
                                                            .inner_margin(14.0 * scale)
                                                            .show(ui, |ui| {
                                                                ui.vertical(|ui| {
                                                                    ui.label(
                                                                        egui::RichText::new(format!(
                                                                            "[ CH {:02} ]",
                                                                            idx + 1
                                                                        ))
                                                                        .strong()
                                                                        .size((config.body_size + 1.0) * scale)
                                                                        .color(config.accent_color),
                                                                    );
                                                                    ui.add_space(4.0 * scale);
                                                                    ui.strong(
                                                                        egui::RichText::new(&item.title)
                                                                            .size((config.body_size + 2.0) * scale)
                                                                            .family(font_fam.clone())
                                                                            .color(config.text_color),
                                                                    );
                                                                    ui.add_space(8.0 * scale);
                                                                    if ui
                                                                        .add(
                                                                            egui::Button::new(
                                                                                egui::RichText::new(
                                                                                    "★ TOUCH ★",
                                                                                )
                                                                                .size(11.0 * scale)
                                                                                .strong()
                                                                                .color(
                                                                                    egui::Color32::WHITE,
                                                                                ),
                                                                            )
                                                                            .fill(config.accent_color)
                                                                            .rounding(12.0 * scale),
                                                                        )
                                                                        .clicked()
                                                                    {
                                                                        dispatch_add_to_library(store, rt, item);
                                                                    }
                                                                });
                                                            });
                                                    }
                                                    "terminal_feed" => {
                                                        egui::Frame::none()
                                                            .rounding(rounding * scale)
                                                            .stroke(egui::Stroke::new(
                                                                stroke_width,
                                                                config.border_color,
                                                            ))
                                                            .fill(config.fill_color)
                                                            .inner_margin(10.0 * scale)
                                                            .show(ui, |ui| {
                                                                ui.horizontal_wrapped(|ui| {
                                                                    ui.label(
                                                                        egui::RichText::new(">_ STREAM:")
                                                                            .size(11.0 * scale)
                                                                            .color(config.accent_color)
                                                                            .monospace(),
                                                                    );
                                                                    ui.strong(
                                                                        egui::RichText::new(&item.title)
                                                                            .size(12.0 * scale)
                                                                            .color(config.text_color)
                                                                            .monospace(),
                                                                    );
                                                                    ui.with_layout(
                                                                        egui::Layout::right_to_left(
                                                                            egui::Align::Center,
                                                                        ),
                                                                        |ui| {
                                                                            if ui.button(egui::RichText::new("[ CONN ]").size(10.0 * scale)).clicked() {
                                                                                dispatch_add_to_library(store, rt, item);
                                                                            }
                                                                        },
                                                                    );
                                                                });
                                                            });
                                                    }
                                                    _ => {
                                                        egui::Frame::none()
                                                            .rounding(rounding * scale)
                                                            .stroke(egui::Stroke::new(
                                                                stroke_width,
                                                                config.border_color,
                                                            ))
                                                            .fill(config.fill_color)
                                                            .inner_margin(10.0 * scale)
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
                                                    }
                                                }
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
