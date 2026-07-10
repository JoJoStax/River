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

/// A single keyframe in a custom animation timeline.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationKeyframe {
    pub at: f32,  // 0.0 to 1.0 normalized time
    pub properties: HashMap<String, AnimPropertyValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnimPropertyValue {
    Float(f32),
    Color(egui::Color32),
    StringVal(String),
}

/// A complete animation definition.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationDef {
    pub id: String,
    pub keyframes: Vec<AnimationKeyframe>,
    pub duration: f32,      // seconds
    pub easing: String,     // "linear", "ease-in", "ease-out", "ease-in-out", "sine", "bounce", "elastic"
    pub loop_mode: bool,    // true = repeat forever
    pub ping_pong: bool,    // true = reverse on loop
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
        effect: String,
        speed: f32,
        animations: Vec<AnimationDef>,
    },
    Container {
        kind: String, // "row", "column", "box", "scroll", "dropdown", "combo-box"
        id: String,
        text: String,
        color: String,
        style: String,
        spacing: f32,
        padding: f32,
        children: Vec<UiNode>,
        effect: String,
        speed: f32,
        animations: Vec<AnimationDef>,
    },
    Widget {
        kind: String, // "heading", "label", "button", "nav-item", "separator", "spacer", "menu-bar", "catalog-view", "theme-switcher", "device-switcher", "image", "svg"
        text: String,
        url: String,
        action: String,
        color: String,
        size: f32,
        height: f32,
        style: String,
        columns: usize,
        rounding: f32,
        border_width: f32,
        effect: String,
        speed: f32,
        animations: Vec<AnimationDef>,
    },
    /// Generic responsive grid layout — replaces hardcoded catalog-view grids.
    /// KDL plugins use this to create any grid arrangement with dynamic columns.
    Grid {
        id: String,
        columns: usize,
        spacing_x: f32,
        spacing_y: f32,
        min_cell_width: f32,
        fill: egui::Color32,
        rounding: f32,
        border_width: f32,
        children: Vec<UiNode>,
        effect: String,
        speed: f32,
        animations: Vec<AnimationDef>,
    },
    /// Data-bound iteration node. Iterates over a data source and stamps out
    /// the template children for each item, with per-item data bindings.
    ForEach {
        source: String,
        item_var: String,
        template: Vec<UiNode>,
        effect: String,
        speed: f32,
        animations: Vec<AnimationDef>,
    },
    /// Conditional visibility node. Shows children only when a data binding
    /// matches (or doesn't match) a specified value.
    Condition {
        source: String,
        equals: String,
        not_equals: String,
        children: Vec<UiNode>,
        else_children: Vec<UiNode>,
    },
}

impl UiNode {
    pub fn effect(&self) -> &str {
        match self {
            UiNode::Panel { effect, .. } => effect,
            UiNode::Container { effect, .. } => effect,
            UiNode::Widget { effect, .. } => effect,
            UiNode::Grid { effect, .. } => effect,
            UiNode::ForEach { effect, .. } => effect,
            UiNode::Condition { .. } => "none",
        }
    }

    pub fn speed(&self) -> f32 {
        match self {
            UiNode::Panel { speed, .. } => *speed,
            UiNode::Container { speed, .. } => *speed,
            UiNode::Widget { speed, .. } => *speed,
            UiNode::Grid { speed, .. } => *speed,
            UiNode::ForEach { speed, .. } => *speed,
            UiNode::Condition { .. } => 1.0,
        }
    }

    pub fn animations(&self) -> &[AnimationDef] {
        match self {
            UiNode::Panel { animations, .. } => animations,
            UiNode::Container { animations, .. } => animations,
            UiNode::Widget { animations, .. } => animations,
            UiNode::Grid { animations, .. } => animations,
            UiNode::ForEach { animations, .. } => animations,
            UiNode::Condition { .. } => &[],
        }
    }

    pub fn has_active_animation(&self) -> bool {
        (self.effect() != "none" && !self.effect().is_empty()) || !self.animations().is_empty()
    }
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
    pub background_color: egui::Color32,
    pub background_type: String, // "solid", "gradient", "grid", "matrix", "stars", "waves", "image"
    pub background_url: String,
    pub background_color_2: egui::Color32,
    pub background_speed: f32,
    pub background_kdl: String,
    pub animation_effect: String,
    pub animation_speed: f32,
    pub title: String,
    pub tagline: String,
    pub button_style: String,
    pub layout_mode: String,
    pub columns: usize,
    /// Multi-Device AST Layout Trees mapped by target ("desktop", "mobile", "tv")!
    pub layouts: HashMap<String, Vec<UiNode>>,
    /// Named custom animation definitions from `animations { define-animation ... }`
    pub animation_defs: HashMap<String, AnimationDef>,
    /// Nodes to render on the background layer
    pub background_nodes: Vec<UiNode>,
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
            background_color: egui::Color32::from_rgb(10, 10, 15),
            background_type: "solid".to_string(),
            background_url: String::new(),
            background_color_2: egui::Color32::from_rgb(10, 15, 30),
            background_speed: 1.0,
            background_kdl: String::new(),
            animation_effect: "none".to_string(),
            animation_speed: 1.0,
            title: "🌊 RIVER MEDIA HUB".to_string(),
            tagline: "Declarative KDL UI".to_string(),
            button_style: "brackets".to_string(),
            layout_mode: "grid".to_string(),
            columns: 3,
            layouts: HashMap::new(),
            animation_defs: HashMap::new(),
            background_nodes: Vec::new(),
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
                            config.background_kdl = child.to_string();
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
                                        "from" | "color" => config.background_color = parse_hex_color_alpha(&val_str, config.background_color),
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
                            
                            let mut bg_nodes = Vec::new();
                            if let Some(bg_children) = child.children() {
                                for ast_node in bg_children.nodes() {
                                    if let Some(ui_node) = parse_ast_node(ast_node, config.fill_color) {
                                        bg_nodes.push(ui_node);
                                    }
                                }
                            }
                            config.background_nodes = bg_nodes;
                        } else if child_name == "animations" {
                            if let Some(anim_children) = child.children() {
                                for anim_node in anim_children.nodes() {
                                    if anim_node.name().to_string() == "define-animation" {
                                        if let Some(def) = parse_animation_def(anim_node, config.fill_color) {
                                            config.animation_defs.insert(def.id.clone(), def);
                                        }
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
                                        ("palette", "background") => {
                                            config.background_color = parse_hex_color_alpha(&val_str, config.background_color);
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
                                        ("style", "speed") => {
                                            if let Ok(s) = val_str.parse::<f32>() {
                                                config.background_speed = s;
                                                config.animation_speed = s;
                                            }
                                        }
                                        ("style", "animation-effect") => config.animation_effect = val_str,
                                        ("style", "animation-speed") => {
                                            if let Ok(s) = val_str.parse::<f32>() {
                                                config.animation_speed = s;
                                            }
                                        }
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

                            if child_name == "style" {
                                if let Some(style_children) = child.children() {
                                    for style_child in style_children.nodes() {
                                        let sub_name = style_child.name().to_string();
                                        for entry in style_child.entries() {
                                            if let Some(key) = entry.name() {
                                                let key_str = key.to_string();
                                                let val_str = match entry.value().as_string() {
                                                    Some(s) => s.to_string(),
                                                    None => entry.value().to_string(),
                                                };
                                                match (sub_name.as_str(), key_str.as_str()) {
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
                                                    ("palette", "background") => {
                                                        config.background_color = parse_hex_color_alpha(&val_str, config.background_color);
                                                    }
                                                    ("typography", "family") => config.font_family = val_str.clone(),
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
                                                    ("spacing", "item-x") => {
                                                        if let Ok(s) = val_str.parse::<f32>() {
                                                            config.spacing_x = s;
                                                        }
                                                    }
                                                    ("spacing", "item-y") => {
                                                        if let Ok(s) = val_str.parse::<f32>() {
                                                            config.spacing_y = s;
                                                        }
                                                    }
                                                    ("spacing", "padding") => {
                                                        if let Ok(s) = val_str.parse::<f32>() {
                                                            config.rounding = s;
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }

                                        if let Some(entry) = style_child.entries().get(0) {
                                            let val = entry.value();
                                            let val_str = match val.as_string() {
                                                Some(s) => s.to_string(),
                                                None => val.to_string(),
                                            };
                                            match sub_name.as_str() {
                                                "background-type" => config.background_type = val_str,
                                                "background-url" => config.background_url = val_str,
                                                "fill" => config.fill_color = parse_hex_color_alpha(&val_str, config.fill_color),
                                                "background" => config.background_color = parse_hex_color_alpha(&val_str, config.background_color),
                                                "secondary" => config.background_color_2 = parse_hex_color_alpha(&val_str, config.background_color_2),
                                                "speed" => {
                                                    if let Ok(s) = val_str.parse::<f32>() {
                                                        config.background_speed = s;
                                                        config.animation_speed = s;
                                                    }
                                                }
                                                "rounding" => {
                                                    if let Ok(s) = val_str.parse::<f32>() {
                                                        config.rounding = s;
                                                    }
                                                }
                                                "border-width" => {
                                                    if let Ok(s) = val_str.parse::<f32>() {
                                                        config.border_width = s;
                                                    }
                                                }
                                                "animation-effect" => config.animation_effect = val_str,
                                                "animation-speed" => {
                                                    if let Ok(s) = val_str.parse::<f32>() {
                                                        config.animation_speed = s;
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
            }
        }

        if config.background_kdl.is_empty() || !config.background_kdl.contains("layer") {
            config.background_kdl = generate_fallback_background_kdl(
                &config.background_type,
                config.background_color,
                config.background_color_2,
                config.accent_color,
                config.background_speed,
                &config.background_url,
            );
        }

        config
    }
}

fn generate_fallback_background_kdl(
    bg_type: &str,
    color1: egui::Color32,
    color2: egui::Color32,
    accent: egui::Color32,
    speed: f32,
    url: &str,
) -> String {
    let hex1 = format!("#{:02x}{:02x}{:02x}", color1.r(), color1.g(), color1.b());
    let hex2 = format!("#{:02x}{:02x}{:02x}", color2.r(), color2.g(), color2.b());
    let hex_accent = format!("#{:02x}{:02x}{:02x}", accent.r(), accent.g(), accent.b());

    match bg_type {
        "gradient" => format!(
            r#"background type="gradient" {{
    layer shape="rect" count=30 speed={speed:.2} {{
        x "0"
        y "i * (h / 29)"
        width "w"
        height "h / 29 + 1"
        color from="{hex1}" to="{hex2}" mix="clamp((i/29) * 0.7 + (sin(time*speed + (i/29)*pi) * 0.5 + 0.5) * 0.3, 0, 1)"
    }}
}}"#
        ),
        "grid" => format!(
            r#"background type="grid" {{
    layer shape="fill" count=1 {{
        color value="{hex1}"
    }}
    layer shape="line" count=40 speed={:.2} {{
        x "wrap(i * 36 + time*speed, w + 36) - 36"
        y "0"
        x2 "wrap(i * 36 + time*speed, w + 36) - 36"
        y2 "h"
        stroke_width "1"
        color value="{hex2}"
    }}
    layer shape="line" count=30 speed={:.2} {{
        x "0"
        y "wrap(i * 36 + time*speed, h + 36) - 36"
        x2 "w"
        y2 "wrap(i * 36 + time*speed, h + 36) - 36"
        stroke_width "1"
        color value="{hex2}"
    }}
}}"#,
            speed * 15.0,
            speed * 25.0
        ),
        "waves" => format!(
            r#"background type="waves" {{
    layer shape="fill" count=1 {{
        color value="{hex1}"
    }}
    layer shape="line" count=60 speed={:.2} {{
        x "(i/59) * w"
        y "h/6*1 + sin((i/59)*w*0.006 + time*speed) * 35"
        x2 "((i+1)/59) * w"
        y2 "h/6*1 + sin(((i+1)/59)*w*0.006 + time*speed) * 35"
        stroke_width "2.5"
        color from="{hex_accent}" to="{hex2}" mix="0"
    }}
    layer shape="line" count=60 speed={:.2} {{
        x "(i/59) * w"
        y "h/6*2 + sin((i/59)*w*0.008 + time*speed) * 50"
        x2 "((i+1)/59) * w"
        y2 "h/6*2 + sin(((i+1)/59)*w*0.008 + time*speed) * 50"
        stroke_width "2.5"
        color from="{hex_accent}" to="{hex2}" mix="0.25"
    }}
    layer shape="line" count=60 speed={:.2} {{
        x "(i/59) * w"
        y "h/6*3 + sin((i/59)*w*0.010 + time*speed) * 65"
        x2 "((i+1)/59) * w"
        y2 "h/6*3 + sin(((i+1)/59)*w*0.010 + time*speed) * 65"
        stroke_width "2.5"
        color from="{hex_accent}" to="{hex2}" mix="0.5"
    }}
    layer shape="line" count=60 speed={:.2} {{
        x "(i/59) * w"
        y "h/6*4 + sin((i/59)*w*0.012 + time*speed) * 80"
        x2 "((i+1)/59) * w"
        y2 "h/6*4 + sin(((i+1)/59)*w*0.012 + time*speed) * 80"
        stroke_width "2.5"
        color from="{hex_accent}" to="{hex2}" mix="0.75"
    }}
    layer shape="line" count=60 speed={:.2} {{
        x "(i/59) * w"
        y "h/6*5 + sin((i/59)*w*0.014 + time*speed) * 95"
        x2 "((i+1)/59) * w"
        y2 "h/6*5 + sin(((i+1)/59)*w*0.014 + time*speed) * 95"
        stroke_width "2.5"
        color from="{hex_accent}" to="{hex2}" mix="1"
    }}
}}"#,
            speed * 1.0,
            speed * 1.3,
            speed * 1.6,
            speed * 1.9,
            speed * 2.2
        ),
        "matrix" => format!(
            r#"background type="matrix" {{
    layer shape="fill" count=1 {{
        color value="{hex1}"
    }}
    layer shape="line" count=40 speed={speed:.2} {{
        x "i * 25 + 10"
        y "wrap(time*speed*(100 + (i % 5)*30) + i*137, h + 200) - 100 - 40"
        x2 "i * 25 + 10"
        y2 "wrap(time*speed*(100 + (i % 5)*30) + i*137, h + 200) - 100"
        stroke_width "2"
        color value="{hex2}"
    }}
    layer shape="circle" count=40 speed={speed:.2} {{
        x "i * 25 + 10"
        y "wrap(time*speed*(100 + (i % 5)*30) + i*137, h + 200) - 100"
        radius "2.5"
        color value="{hex_accent}"
    }}
}}"#
        ),
        "stars" => format!(
            r#"background type="stars" {{
    layer shape="fill" count=1 {{
        color value="{hex1}"
    }}
    layer shape="circle" count=60 speed={speed:.2} {{
        x "wrap(i * 313, w)"
        y "wrap(i * 701, h)"
        radius "1.0 + (sin(time*speed + i) * 0.5 + 0.5) * 1.5"
        color from="{hex2}" to="{hex_accent}" mix="sin(time*speed + i) * 0.5 + 0.5"
    }}
}}"#
        ),
        "image" | "svg" => format!(
            r#"background type="image" {{
    layer shape="fill" count=1 {{
        color value="{hex1}"
    }}
    layer shape="image" {{
        image url="{url}" fit="cover"
    }}
}}"#
        ),
        _ => format!(
            r#"background type="solid" {{
    layer shape="fill" count=1 {{
        color value="{hex1}"
    }}
}}"#
        ),
    }
}

impl UiThemeConfig {

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

    /// Check if the theme or any of its active AST nodes require continuous repaints.
    pub fn requires_continuous_repaint(&self, target_device: &str) -> bool {
        if self.animation_effect != "none" && !self.animation_effect.is_empty() {
            return true;
        }
        if self.background_kdl.contains("time") {
            return true;
        }
        fn tree_has_animation(nodes: &[UiNode]) -> bool {
            for node in nodes {
                if node.has_active_animation() {
                    return true;
                }
                match node {
                    UiNode::Panel { children, .. }
                    | UiNode::Container { children, .. }
                    | UiNode::Grid { children, .. } => {
                        if tree_has_animation(children) {
                            return true;
                        }
                    }
                    UiNode::ForEach { template, .. } => {
                        if tree_has_animation(template) {
                            return true;
                        }
                    }
                    UiNode::Condition { children, else_children, .. } => {
                        if tree_has_animation(children) || tree_has_animation(else_children) {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
            false
        }
        tree_has_animation(self.get_active_layout(target_device)) || tree_has_animation(&self.background_nodes)
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
    let mut action = String::new();
    let mut color = "text".to_string();
    let mut font_size = 14.0;
    let mut style = "default".to_string();
    let mut columns = 3;
    let mut rounding = 0.0;
    let mut border_width = 0.0;
    let mut effect = "none".to_string();
    let mut speed = 1.0;
    let mut source = String::new();
    let mut item_var = "item".to_string();
    let mut equals = String::new();
    let mut not_equals = String::new();
    let mut min_cell_width = 0.0f32;
    let mut spacing_x = 0.0f32;
    let mut spacing_y = 0.0f32;

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
                "action" | "intent" | "on-click" => action = val_str,
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
                "effect" | "animation-effect" => effect = val_str,
                "speed" | "animation-speed" => {
                    if let Ok(f) = val_str.parse::<f32>() {
                        speed = f;
                    }
                }
                "source" => source = val_str,
                "item" | "item-var" => item_var = val_str,
                "equals" => equals = val_str,
                "not-equals" => not_equals = val_str,
                "min-cell-width" => {
                    if let Ok(f) = val_str.parse::<f32>() {
                        min_cell_width = f;
                    }
                }
                "spacing-x" => {
                    if let Ok(f) = val_str.parse::<f32>() {
                        spacing_x = f;
                    }
                }
                "spacing-y" => {
                    if let Ok(f) = val_str.parse::<f32>() {
                        spacing_y = f;
                    }
                }
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
    let mut animations = Vec::new();
    if let Some(child_doc) = node.children() {
        for child_node in child_doc.nodes() {
            let child_name = child_node.name().to_string();
            if child_name == "animation" && child_node.entries().iter().any(|e| e.name().map(|n| n.to_string()).as_deref() == Some("effect")) {
                for entry in child_node.entries() {
                    if let Some(key) = entry.name() {
                        let k = key.to_string();
                        let v = match entry.value().as_string() {
                            Some(s) => s.to_string(),
                            None => entry.value().to_string(),
                        };
                        match k.as_str() {
                            "effect" => effect = v,
                            "speed" => {
                                if let Ok(f) = v.parse::<f32>() {
                                    speed = f;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            } else if child_name == "animate" || (child_name == "animation" && child_node.children().is_some()) {
                if let Some(def) = parse_inline_animation(child_node, default_fill) {
                    animations.push(def);
                }
            } else if let Some(ui_child) = parse_ast_node(child_node, default_fill) {
                children.push(ui_child);
            }
        }
    }

    match name.as_str() {
        "top-panel" | "left-panel" | "bottom-panel" | "right-panel" | "central-panel" => {
            Some(UiNode::Panel { kind: name, id, size, fill, children, effect, speed, animations })
        }
        "row" | "column" | "box" | "scroll" | "dropdown" | "combo-box" | "menu" => {
            Some(UiNode::Container { kind: name, id, text, color, style, spacing, padding, children, effect, speed, animations })
        }
        "heading" | "label" | "button" | "nav-item" | "separator" | "spacer" | "menu-bar" | "catalog-view" | "theme-switcher" | "device-switcher" | "image" | "svg" => {
            Some(UiNode::Widget {
                kind: name,
                text,
                url,
                action,
                color,
                size: if size > 0.0 { size } else { font_size },
                height,
                style,
                columns,
                rounding,
                border_width,
                effect,
                speed,
                animations,
            })
        }
        "grid" => {
            let gsx = if spacing_x > 0.0 { spacing_x } else { spacing };
            let gsy = if spacing_y > 0.0 { spacing_y } else { spacing };
            let mcw = if min_cell_width > 0.0 { min_cell_width } else if size > 0.0 { size } else { 180.0 };
            Some(UiNode::Grid {
                id,
                columns,
                spacing_x: gsx,
                spacing_y: gsy,
                min_cell_width: mcw,
                fill,
                rounding,
                border_width,
                children,
                effect,
                speed,
                animations,
            })
        }
        "for-each" => {
            let src = if source.is_empty() { text } else { source };
            let ivar = if style != "default" && !style.is_empty() { style } else { item_var };
            Some(UiNode::ForEach {
                source: src,
                item_var: ivar,
                template: children,
                effect,
                speed,
                animations,
            })
        }
        "if-state" | "condition" => {
            let src = if source.is_empty() { text } else { source };
            let eq = if equals.is_empty() && !style.is_empty() && style != "default" { style } else { equals };
            // Extract else-children from any child Container with kind="else"
            let mut main_children = Vec::new();
            let mut else_children = Vec::new();
            for child in children {
                if let UiNode::Container { ref kind, children: ref inner, .. } = child {
                    if kind == "else" {
                        else_children.extend(inner.clone());
                        continue;
                    }
                }
                main_children.push(child);
            }
            Some(UiNode::Condition {
                source: src,
                equals: eq,
                not_equals,
                children: main_children,
                else_children,
            })
        }
        // Any unknown tag name becomes a generic container — total plugin freedom!
        _ => {
            Some(UiNode::Container { kind: name, id, text, color, style, spacing, padding, children, effect, speed, animations })
        }
    }
}

pub fn parse_hex_color(hex: &str) -> Option<egui::Color32> {
    let hex = hex.trim_start_matches('#').trim();
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

pub fn parse_hex_color_alpha(hex: &str, default: egui::Color32) -> egui::Color32 {
    parse_hex_color(hex).unwrap_or(default)
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

pub fn parse_animation_def(node: &KdlNode, default_color: egui::Color32) -> Option<AnimationDef> {
    let mut id = String::new();
    let mut duration = 1.0;
    let mut easing = "linear".to_string();
    let mut loop_mode = false;
    let mut ping_pong = false;

    for entry in node.entries() {
        if let Some(key) = entry.name() {
            let k = key.to_string();
            let v = match entry.value().as_string() {
                Some(s) => s.to_string(),
                None => entry.value().to_string(),
            };
            match k.as_str() {
                "id" => id = v,
                "duration" => {
                    if let Ok(f) = v.parse::<f32>() {
                        duration = f;
                    }
                }
                "easing" => easing = v,
                "loop" | "loop-mode" => loop_mode = v == "true" || v == "1",
                "ping-pong" => ping_pong = v == "true" || v == "1",
                _ => {}
            }
        } else if let Some(s) = entry.value().as_string() {
            if id.is_empty() {
                id = s.to_string();
            }
        }
    }

    if id.is_empty() {
        return None;
    }

    let mut keyframes = Vec::new();
    if let Some(children) = node.children() {
        for child in children.nodes() {
            if child.name().to_string() == "keyframe" {
                let mut at = 0.0f32;
                let mut properties = HashMap::new();
                for entry in child.entries() {
                    if let Some(key) = entry.name() {
                        let prop_name = key.to_string();
                        let v_str = match entry.value().as_string() {
                            Some(s) => s.to_string(),
                            None => entry.value().to_string(),
                        };
                        if prop_name == "at" {
                            if let Ok(f) = v_str.parse::<f32>() {
                                at = f.clamp(0.0, 1.0);
                            }
                        } else if let Some(val) = parse_anim_property_val(&prop_name, &v_str, default_color) {
                            properties.insert(prop_name, val);
                        }
                    }
                }
                if !properties.is_empty() {
                    keyframes.push(AnimationKeyframe { at, properties });
                }
            }
        }
    }
    keyframes.sort_by(|a, b| a.at.partial_cmp(&b.at).unwrap_or(std::cmp::Ordering::Equal));

    Some(AnimationDef {
        id,
        keyframes,
        duration,
        easing,
        loop_mode,
        ping_pong,
    })
}

pub fn parse_inline_animation(node: &KdlNode, default_color: egui::Color32) -> Option<AnimationDef> {
    if node.children().is_some() {
        return parse_animation_def(node, default_color);
    }

    let mut prop = String::new();
    let mut from_val = String::new();
    let mut to_val = String::new();
    let mut duration = 1.0f32;
    let mut easing = "linear".to_string();
    let mut loop_mode = false;
    let mut ping_pong = false;

    for entry in node.entries() {
        if let Some(key) = entry.name() {
            let k = key.to_string();
            let v = match entry.value().as_string() {
                Some(s) => s.to_string(),
                None => entry.value().to_string(),
            };
            match k.as_str() {
                "property" => prop = v,
                "from" => from_val = v,
                "to" => to_val = v,
                "duration" => {
                    if let Ok(f) = v.parse::<f32>() {
                        duration = f;
                    }
                }
                "easing" => easing = v,
                "loop" | "loop-mode" => loop_mode = v == "true" || v == "1",
                "ping-pong" => ping_pong = v == "true" || v == "1",
                _ => {}
            }
        }
    }

    if !prop.is_empty() && !from_val.is_empty() && !to_val.is_empty() {
        let mut keyframes = Vec::new();
        if let Some(p_from) = parse_anim_property_val(&prop, &from_val, default_color) {
            let mut props0 = HashMap::new();
            props0.insert(prop.clone(), p_from);
            keyframes.push(AnimationKeyframe { at: 0.0, properties: props0 });
        }
        if let Some(p_to) = parse_anim_property_val(&prop, &to_val, default_color) {
            let mut props1 = HashMap::new();
            props1.insert(prop.clone(), p_to);
            keyframes.push(AnimationKeyframe { at: 1.0, properties: props1 });
        }
        if !keyframes.is_empty() {
            return Some(AnimationDef {
                id: format!("inline-{}", prop),
                keyframes,
                duration,
                easing,
                loop_mode,
                ping_pong,
            });
        }
    }
    None
}

pub fn parse_anim_property_val(prop: &str, v: &str, default_color: egui::Color32) -> Option<AnimPropertyValue> {
    match prop {
        "offset-x" | "offset-y" | "opacity" | "scale-x" | "scale-y" | "rotation"
        | "border-width" | "rounding" | "padding" | "size" => {
            if let Ok(f) = v.parse::<f32>() {
                Some(AnimPropertyValue::Float(f))
            } else {
                Some(AnimPropertyValue::StringVal(v.to_string()))
            }
        }
        "border-color" | "fill" | "text-color" => {
            if v.contains('{') || (!v.starts_with('#') && v != "accent" && v != "secondary" && v != "border" && v != "text") {
                Some(AnimPropertyValue::StringVal(v.to_string()))
            } else {
                Some(AnimPropertyValue::Color(parse_hex_color_alpha(v, default_color)))
            }
        }
        _ => {
            if let Ok(f) = v.parse::<f32>() {
                Some(AnimPropertyValue::Float(f))
            } else if !v.is_empty() {
                Some(AnimPropertyValue::StringVal(v.to_string()))
            } else {
                None
            }
        }
    }
}
