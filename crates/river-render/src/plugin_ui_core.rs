use crate::core_ui::{compile_kdl, CompiledUiLayout};
use crate::hotplug_ui::run_ui_plugin;
use crate::ui_plugin::{UiPluginManager, UiRenderer};
use eframe::egui;
use kdl::{KdlDocument, KdlNode};
use river_presentation::{AppState, AppStore};
use std::collections::HashMap;
use std::sync::Arc;

/// The execution mode of a KDL UI Plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiExecutionMode {
    /// Tier 1: Dynamic hot-plug interpretation ("I ran your ui plugin!")
    HotplugDynamic,
    /// Tier 2: Pre-compiled native layout ("I compile KDL to be more efficient!")
    CoreCompiled,
}

/// A recursive AST Node representing any UI building block in River's DOM engine.
#[derive(Debug, Clone, PartialEq)]
pub enum UiNode {
    Panel {
        kind: String, // "top-panel", "left-panel", "bottom-panel", "right-panel", "central-panel"
        id: String,
        size: f32,
        fill: egui::Color32,
        children: Vec<UiNode>,
    },
    Container {
        kind: String, // "row", "column", "box", "scroll"
        spacing: f32,
        padding: f32,
        children: Vec<UiNode>,
    },
    Widget {
        kind: String, // "heading", "label", "button", "separator", "spacer", "menu-bar", "catalog-view", "theme-switcher", "device-switcher", "image", "svg"
        text: String,
        url: String,
        color: String,
        size: f32,
        height: f32,
        style: String,
        columns: usize,
        rounding: f32,
        border_width: f32,
        effect: String,
    },
}

/// Extracted configuration representing a user-defined UI theme from a KDL document.
#[derive(Debug, Clone)]
pub struct UiThemeConfig {
    pub id: String,
    pub name: String,
    pub mode: String,
    pub window_layout: String,
    pub sidebar_width: f32,
    pub max_width: f32,
    pub font_family: String,
    pub header_size: f32,
    pub body_size: f32,
    pub spacing_x: f32,
    pub spacing_y: f32,
    pub accent_color: egui::Color32,
    pub secondary_color: egui::Color32,
    pub text_color: egui::Color32,
    pub rounding: f32,
    pub border_width: f32,
    pub border_color: egui::Color32,
    pub fill_color: egui::Color32,
    pub background_type: String, // "solid", "gradient", "grid", "matrix", "stars", "waves", "image"
    pub background_url: String,
    pub background_color_2: egui::Color32,
    pub background_speed: f32,
    pub animation_effect: String,
    pub animation_speed: f32,
    pub title: String,
    pub tagline: String,
    pub button_style: String,
    pub layout_mode: String,
    pub columns: usize,
    /// Multi-Device AST Layout Trees mapped by target ("desktop", "mobile", "tv")!
    pub layouts: HashMap<String, Vec<UiNode>>,
}

impl UiThemeConfig {
    pub fn from_kdl(doc: &KdlDocument) -> Self {
        let mut config = Self {
            id: "custom-kdl".to_string(),
            name: "Custom KDL Theme".to_string(),
            mode: "hotplug".to_string(),
            window_layout: "top_bottom_bars".to_string(),
            sidebar_width: 230.0,
            max_width: 800.0,
            font_family: "proportional".to_string(),
            header_size: 22.0,
            body_size: 14.0,
            spacing_x: 8.0,
            spacing_y: 8.0,
            accent_color: egui::Color32::from_rgb(100, 200, 255),
            secondary_color: egui::Color32::from_rgb(150, 150, 150),
            text_color: egui::Color32::WHITE,
            rounding: 0.0,
            border_width: 0.0,
            border_color: egui::Color32::TRANSPARENT,
            fill_color: egui::Color32::from_rgb(30, 30, 30),
            background_type: "solid".to_string(),
            background_url: String::new(),
            background_color_2: egui::Color32::from_rgb(10, 15, 30),
            background_speed: 1.0,
            animation_effect: "none".to_string(),
            animation_speed: 1.0,
            title: "🌊 RIVER MEDIA HUB".to_string(),
            tagline: "Declarative KDL UI".to_string(),
            button_style: "brackets".to_string(),
            layout_mode: "grid".to_string(),
            columns: 3,
            layouts: HashMap::new(),
        };

        for node in doc.nodes() {
            let node_name = node.name().to_string();
            if node_name == "plugin" {
                for entry in node.entries() {
                    if let Some(key) = entry.name() {
                        let key_str = key.to_string();
                        let val_str = match entry.value().as_string() {
                            Some(s) => s.to_string(),
                            None => entry.value().to_string(),
                        };
                        match key_str.as_str() {
                            "id" => config.id = val_str,
                            "name" => config.name = val_str,
                            "mode" => config.mode = val_str,
                            _ => {}
                        }
                    }
                }

                if let Some(children) = node.children() {
                    for child in children.nodes() {
                        let child_name = child.name().to_string();
                        if child_name == "layout" {
                            // Extract target device property ("desktop", "mobile", "tv")!
                            let mut target = "desktop".to_string();
                            for entry in child.entries() {
                                if entry.name().map(|n| n.to_string()).as_deref() == Some("target") {
                                    if let Some(val) = entry.value().as_string() {
                                        target = val.to_string();
                                    }
                                }
                            }

                            let mut nodes = Vec::new();
                            if let Some(layout_children) = child.children() {
                                for ast_node in layout_children.nodes() {
                                    if let Some(ui_node) = parse_ast_node(ast_node, config.fill_color) {
                                        nodes.push(ui_node);
                                    }
                                }
                            }
                            config.layouts.insert(target, nodes);
                        } else if child_name == "background" {
                            for entry in child.entries() {
                                if let Some(key) = entry.name() {
                                    let key_str = key.to_string();
                                    let val_str = match entry.value().as_string() {
                                        Some(s) => s.to_string(),
                                        None => entry.value().to_string(),
                                    };
                                    match key_str.as_str() {
                                        "type" => config.background_type = val_str,
                                        "url" | "src" => config.background_url = val_str,
                                        "from" | "color" => config.fill_color = parse_hex_color_alpha(&val_str, config.fill_color),
                                        "to" | "secondary" => config.background_color_2 = parse_hex_color_alpha(&val_str, config.background_color_2),
                                        "speed" => {
                                            if let Ok(s) = val_str.parse::<f32>() {
                                                config.background_speed = s;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        } else {
                            for entry in child.entries() {
                                if let Some(key) = entry.name() {
                                    let key_str = key.to_string();
                                    let val_str = match entry.value().as_string() {
                                        Some(s) => s.to_string(),
                                        None => entry.value().to_string(),
                                    };
                                    match (child_name.as_str(), key_str.as_str()) {
                                        ("window", "layout") => config.window_layout = val_str,
                                        ("window", "sidebar-width") => {
                                            if let Ok(w) = val_str.parse::<f32>() {
                                                config.sidebar_width = w;
                                            }
                                        }
                                        ("window", "max-width") => {
                                            if let Ok(w) = val_str.parse::<f32>() {
                                                config.max_width = w;
                                            }
                                        }
                                        ("typography", "family") => config.font_family = val_str,
                                        ("typography", "header-size") => {
                                            if let Ok(s) = val_str.parse::<f32>() {
                                                config.header_size = s;
                                            }
                                        }
                                        ("typography", "body-size") => {
                                            if let Ok(s) = val_str.parse::<f32>() {
                                                config.body_size = s;
                                            }
                                        }
                                        ("typography", "spacing-x") => {
                                            if let Ok(s) = val_str.parse::<f32>() {
                                                config.spacing_x = s;
                                            }
                                        }
                                        ("typography", "spacing-y") => {
                                            if let Ok(s) = val_str.parse::<f32>() {
                                                config.spacing_y = s;
                                            }
                                        }
                                        ("palette", "accent") => {
                                            config.accent_color = parse_hex_color_alpha(&val_str, config.accent_color);
                                        }
                                        ("palette", "secondary") => {
                                            config.secondary_color = parse_hex_color_alpha(&val_str, config.secondary_color);
                                        }
                                        ("palette", "text") => {
                                            config.text_color = parse_hex_color_alpha(&val_str, config.text_color);
                                        }
                                        ("palette", "border") => {
                                            config.border_color = parse_hex_color_alpha(&val_str, config.border_color);
                                        }
                                        ("style", "rounding") => {
                                            if let Ok(r) = val_str.parse::<f32>() {
                                                config.rounding = r;
                                            }
                                        }
                                        ("style", "border-width") => {
                                            if let Ok(w) = val_str.parse::<f32>() {
                                                config.border_width = w;
                                            }
                                        }
                                        ("style", "fill") => {
                                            config.fill_color = parse_hex_color_alpha(&val_str, config.fill_color);
                                        }
                                        ("style", "background-type") => config.background_type = val_str,
                                        ("style", "background-url") => config.background_url = val_str,
                                        ("animation", "effect") => config.animation_effect = val_str,
                                        ("animation", "speed") => {
                                            if let Ok(s) = val_str.parse::<f32>() {
                                                config.animation_speed = s;
                                            }
                                        }
                                        ("header", "title") => config.title = val_str,
                                        ("header", "tagline") => config.tagline = val_str,
                                        ("navigation", "button-style") => config.button_style = val_str,
                                        ("content", "layout") => config.layout_mode = val_str,
                                        ("content", "columns") => {
                                            if let Ok(c) = val_str.parse::<usize>() {
                                                config.columns = c;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        config
    }

    pub fn get_font_family(&self) -> egui::FontFamily {
        if self.font_family == "monospace" {
            egui::FontFamily::Monospace
        } else {
            egui::FontFamily::Proportional
        }
    }

    /// Retrieve the layout AST tree tailored for the requested target device!
    pub fn get_active_layout(&self, target_device: &str) -> &[UiNode] {
        if let Some(nodes) = self.layouts.get(target_device) {
            nodes
        } else if let Some(nodes) = self.layouts.get("desktop") {
            nodes
        } else if let Some(nodes) = self.layouts.values().next() {
            nodes
        } else {
            &[]
        }
    }
}

/// Recursively parse a KDL AST node into a `UiNode`.
fn parse_ast_node(node: &KdlNode, default_fill: egui::Color32) -> Option<UiNode> {
    let name = node.name().to_string();
    let mut id = String::new();
    let mut size = 0.0;
    let mut height = 0.0;
    let mut fill = default_fill;
    let mut spacing = 8.0;
    let mut padding = 8.0;
    let mut text = String::new();
    let mut url = String::new();
    let mut color = "text".to_string();
    let mut font_size = 14.0;
    let mut style = "default".to_string();
    let mut columns = 3;
    let mut rounding = 0.0;
    let mut border_width = 0.0;
    let mut effect = "none".to_string();

    for entry in node.entries() {
        if let Some(key) = entry.name() {
            let key_str = key.to_string();
            let val_str = match entry.value().as_string() {
                Some(s) => s.to_string(),
                None => entry.value().to_string(),
            };
            match key_str.as_str() {
                "id" => id = val_str,
                "height" => {
                    if let Ok(f) = val_str.parse::<f32>() {
                        height = f;
                    }
                }
                "width" | "size" => {
                    if let Ok(f) = val_str.parse::<f32>() {
                        size = f;
                        font_size = f;
                    }
                }
                "url" | "src" => url = val_str,
                "fill" => fill = parse_hex_color_alpha(&val_str, default_fill),
                "spacing" => {
                    if let Ok(f) = val_str.parse::<f32>() {
                        spacing = f;
                    }
                }
                "padding" => {
                    if let Ok(f) = val_str.parse::<f32>() {
                        padding = f;
                    }
                }
                "text" | "label" | "title" => text = val_str,
                "color" => color = val_str,
                "style" => style = val_str,
                "columns" => {
                    if let Ok(c) = val_str.parse::<usize>() {
                        columns = c;
                    }
                }
                "rounding" => {
                    if let Ok(r) = val_str.parse::<f32>() {
                        rounding = r;
                    }
                }
                "border-width" => {
                    if let Ok(w) = val_str.parse::<f32>() {
                        border_width = w;
                    }
                }
                "effect" => effect = val_str,
                _ => {}
            }
        } else if let Some(s) = entry.value().as_string() {
            if text.is_empty() {
                text = s.to_string();
            }
        }
    }

    if (name == "image" || name == "svg") && url.is_empty() && !text.is_empty() {
        url = text.clone();
    }

    let mut children = Vec::new();
    if let Some(child_doc) = node.children() {
        for child_node in child_doc.nodes() {
            let child_name = child_node.name().to_string();
            if child_name == "animation" {
                for entry in child_node.entries() {
                    if let Some(key) = entry.name() {
                        if key.to_string() == "effect" {
                            if let Some(s) = entry.value().as_string() {
                                effect = s.to_string();
                            }
                        }
                    }
                }
            } else if let Some(ui_child) = parse_ast_node(child_node, default_fill) {
                children.push(ui_child);
            }
        }
    }

    match name.as_str() {
        "top-panel" | "left-panel" | "bottom-panel" | "right-panel" | "central-panel" => {
            Some(UiNode::Panel { kind: name, id, size, fill, children })
        }
        "row" | "column" | "box" | "scroll" => {
            Some(UiNode::Container { kind: name, spacing, padding, children })
        }
        "heading" | "label" | "button" | "separator" | "spacer" | "menu-bar" | "catalog-view" | "theme-switcher" | "device-switcher" | "image" | "svg" => {
            Some(UiNode::Widget {
                kind: name,
                text,
                url,
                color,
                size: if size > 0.0 { size } else { font_size },
                height,
                style,
                columns,
                rounding,
                border_width,
                effect,
            })
        }
        _ => None,
    }
}

fn parse_hex_color_alpha(hex: &str, default: egui::Color32) -> egui::Color32 {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 8 {
        if let (Ok(r), Ok(g), Ok(b), Ok(a)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
            u8::from_str_radix(&hex[6..8], 16),
        ) {
            return egui::Color32::from_rgba_unmultiplied(r, g, b, a);
        }
    } else if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return egui::Color32::from_rgb(r, g, b);
        }
    }
    default
}

/// Helper: Draw theme switcher buttons vertically inside a sidebar.
pub fn draw_theme_switcher_vertical(
    ui: &mut egui::Ui,
    ui_manager: &mut UiPluginManager,
    accent: egui::Color32,
    text_col: egui::Color32,
    scale: f32,
) {
    let font_size = 13.0 * scale;
    ui.label(egui::RichText::new("🔌 SWITCH KDL SUITE:").strong().color(accent).size(font_size));
    ui.add_space(6.0 * scale);
    let plugins = ui_manager.list_plugins();
    let active_id = ui_manager
        .active_plugin()
        .map(|p| p.id().to_string())
        .unwrap_or_default();

    for (id, name, is_compiled) in plugins {
        let is_active = id == active_id;
        let badge = if is_compiled { "[Tier 2 AOT]" } else { "[Tier 1 Hotplug]" };
        let label = format!("{} {}", name, badge);
        let btn_text = egui::RichText::new(label)
            .size(font_size)
            .color(if is_active { accent } else { text_col });

        if ui.selectable_label(is_active, btn_text).clicked() && !is_active {
            ui_manager.switch_to(&id);
        }
        ui.add_space(2.0 * scale);
    }
}

/// Helper: Draw theme switcher buttons horizontally inside a header, footer, or central pane.
pub fn draw_theme_switcher_horizontal(
    ui: &mut egui::Ui,
    ui_manager: &mut UiPluginManager,
    accent: egui::Color32,
    text_col: egui::Color32,
    scale: f32,
) {
    let font_size = 13.0 * scale;
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("🔌 KDL Suite:").strong().color(accent).size(font_size));
        let plugins = ui_manager.list_plugins();
        let active_id = ui_manager
            .active_plugin()
            .map(|p| p.id().to_string())
            .unwrap_or_default();

        for (id, name, is_compiled) in plugins {
            let is_active = id == active_id;
            let badge = if is_compiled { "[Tier 2 AOT]" } else { "[Tier 1 Hotplug]" };
            let label = format!("{} {}", name, badge);
            let btn_text = egui::RichText::new(label)
                .size(font_size)
                .color(if is_active { accent } else { text_col });

            if ui.selectable_label(is_active, btn_text).clicked() && !is_active {
                ui_manager.switch_to(&id);
            }
        }
    });
}

/// Helper: Draw device target switcher (Auto Aspect Ratio, Desktop, Mobile, TV) anywhere in the AST tree!
pub fn draw_device_switcher(
    ui: &mut egui::Ui,
    ui_manager: &mut UiPluginManager,
    accent: egui::Color32,
    text_col: egui::Color32,
    scale: f32,
) {
    let font_size = 13.0 * scale;
    ui.horizontal_wrapped(|ui| {
        let width = ui.ctx().screen_rect().width();
        let height = ui.ctx().screen_rect().height();
        let current_resolved = ui_manager.resolve_target_device(width, height).to_string();

        ui.label(
            egui::RichText::new(format!("📱 Mode (ID: {}):", ui_manager.device_id()))
                .strong()
                .color(accent)
                .size(font_size),
        );

        // Auto button
        let is_auto = ui_manager.target_override().is_none();
        let auto_label = format!("🤖 Auto [{}]", current_resolved.to_uppercase());
        let auto_btn = egui::RichText::new(auto_label)
            .size(font_size)
            .color(if is_auto { accent } else { text_col });
        if ui.selectable_label(is_auto, auto_btn).clicked() && !is_auto {
            ui_manager.set_target_override(None);
        }
        ui.add_space(4.0 * scale);

        let targets = [("desktop", "🖥️ Desktop"), ("mobile", "📱 Mobile"), ("tv", "📺 TV (10-Foot)")];
        for (id, label) in targets {
            let is_active = !is_auto && ui_manager.target_override() == Some(id);
            let btn_text = egui::RichText::new(label)
                .size(font_size)
                .color(if is_active { accent } else { text_col });

            if ui.selectable_label(is_active, btn_text).clicked() && !is_active {
                ui_manager.set_target_override(Some(id));
            }
            ui.add_space(4.0 * scale);
        }
    });
}

/// A wrapper struct that holds either a dynamic KDL document or a compiled native layout.
pub struct DeclarativeUiPlugin {
    pub id: String,
    pub name: String,
    pub mode: UiExecutionMode,
    pub doc: KdlDocument,
    pub compiled: Option<CompiledUiLayout>,
}

/// `plugin-ui-core`: The UI Plugin Orchestrator.
pub fn dispatch_ui_job(
    id: &str,
    name: &str,
    doc: KdlDocument,
    mode: UiExecutionMode,
) -> Arc<dyn UiRenderer> {
    let compiled = if mode == UiExecutionMode::CoreCompiled {
        Some(compile_kdl(&doc))
    } else {
        None
    };

    Arc::new(DeclarativeUiPlugin {
        id: id.to_string(),
        name: name.to_string(),
        mode,
        doc,
        compiled,
    })
}

impl UiRenderer for DeclarativeUiPlugin {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_compiled(&self) -> bool {
        self.mode == UiExecutionMode::CoreCompiled
    }

    fn render_window(
        &self,
        ctx: &egui::Context,
        state: &AppState,
        store: &Arc<AppStore>,
        rt: &tokio::runtime::Runtime,
        ui_manager: &mut UiPluginManager,
    ) {
        match self.mode {
            UiExecutionMode::HotplugDynamic => {
                run_ui_plugin(&self.doc, ctx, state, store, rt, ui_manager);
            }
            UiExecutionMode::CoreCompiled => {
                if let Some(compiled_layout) = &self.compiled {
                    compiled_layout.render_compiled_window(ctx, state, store, rt, ui_manager);
                }
            }
        }
    }
}
