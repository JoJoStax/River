use crate::plugin_ui_core::{AnimationDef, AnimationKeyframe, AnimPropertyValue, UiNode, UiThemeConfig, parse_hex_color_alpha};
use crate::data_exports::DataContext;
use eframe::egui;
use std::collections::HashMap;

/// Resolved animation overrides for the current frame.
/// These overrides are computed by evaluating active `AnimationDef` timelines and presets
/// at the current normalized time `t`.
#[derive(Debug, Clone, Default)]
pub struct AnimStyleOverrides {
    pub offset_x: f32,
    pub offset_y: f32,
    pub opacity: Option<f32>,
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotation: f32,
    pub border_color: Option<egui::Color32>,
    pub border_width: Option<f32>,
    pub fill: Option<egui::Color32>,
    pub text_color: Option<egui::Color32>,
    pub rounding: Option<f32>,
    pub padding: Option<f32>,
    pub size: Option<f32>,
}

/// Linear interpolation of colors with alpha support.
pub fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let r = (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8;
    let g = (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8;
    let b_col = (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8;
    let a_alpha = (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8;
    egui::Color32::from_rgba_premultiplied(r, g, b_col, a_alpha)
}

/// Standard easing curves from the KDL animation DSL specification.
pub fn apply_easing(easing: &str, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match easing {
        "linear" => t,
        "ease-in" => t * t,
        "ease-out" => 1.0 - (1.0 - t) * (1.0 - t),
        "ease-in-out" => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - f32::powf(-2.0 * t + 2.0, 2.0) / 2.0
            }
        }
        "sine" => ((t * std::f32::consts::PI - std::f32::consts::FRAC_PI_2).sin() + 1.0) / 2.0,
        "bounce" => {
            let n1 = 7.5625;
            let d1 = 2.75;
            let mut x = t;
            if x < 1.0 / d1 {
                n1 * x * x
            } else if x < 2.0 / d1 {
                x -= 1.5 / d1;
                n1 * x * x + 0.75
            } else if x < 2.5 / d1 {
                x -= 2.25 / d1;
                n1 * x * x + 0.9375
            } else {
                x -= 2.625 / d1;
                n1 * x * x + 0.984375
            }
        }
        "elastic" => {
            if t == 0.0 || t == 1.0 {
                t
            } else {
                let p = 0.3;
                let s = p / 4.0;
                -(f32::powf(2.0, 10.0 * (t - 1.0)) * (((t - 1.0) - s) * (2.0 * std::f32::consts::PI) / p).sin())
            }
        }
        _ => t,
    }
}

/// Expands preset effect names ("glow", "pulse", "float", "bounce", "shimmer") into declarative `AnimationDef` timelines.
/// This guarantees full backward compatibility while keeping the evaluation engine 100% unified!
pub fn expand_preset_effect(effect: &str, speed: f32, config: &UiThemeConfig) -> Vec<AnimationDef> {
    let active_speed = if speed > 0.0 { speed } else if config.animation_speed > 0.0 { config.animation_speed } else { 1.0 };
    let dur = if active_speed > 0.0 { 1.0 / active_speed } else { 1.0 };

    match effect {
        "glow" => {
            let mut kf0 = HashMap::new();
            kf0.insert("border-width".to_string(), AnimPropertyValue::Float(1.5));
            kf0.insert("border-color".to_string(), AnimPropertyValue::Color(config.border_color));

            let mut kf1 = HashMap::new();
            kf1.insert("border-width".to_string(), AnimPropertyValue::Float(4.0));
            kf1.insert("border-color".to_string(), AnimPropertyValue::Color(config.accent_color));

            let mut kf2 = HashMap::new();
            kf2.insert("border-width".to_string(), AnimPropertyValue::Float(1.5));
            kf2.insert("border-color".to_string(), AnimPropertyValue::Color(config.border_color));

            vec![AnimationDef {
                id: "preset-glow".to_string(),
                keyframes: vec![
                    AnimationKeyframe { at: 0.0, properties: kf0 },
                    AnimationKeyframe { at: 0.5, properties: kf1 },
                    AnimationKeyframe { at: 1.0, properties: kf2 },
                ],
                duration: dur,
                easing: "sine".to_string(),
                loop_mode: true,
                ping_pong: false,
            }]
        }
        "pulse" => {
            let mut kf0 = HashMap::new();
            kf0.insert("scale-x".to_string(), AnimPropertyValue::Float(1.0));
            kf0.insert("scale-y".to_string(), AnimPropertyValue::Float(1.0));
            kf0.insert("border-width".to_string(), AnimPropertyValue::Float(1.0));
            kf0.insert("border-color".to_string(), AnimPropertyValue::Color(config.border_color));

            let mut kf1 = HashMap::new();
            kf1.insert("scale-x".to_string(), AnimPropertyValue::Float(1.03));
            kf1.insert("scale-y".to_string(), AnimPropertyValue::Float(1.03));
            kf1.insert("border-width".to_string(), AnimPropertyValue::Float(2.5));
            kf1.insert("border-color".to_string(), AnimPropertyValue::Color(config.accent_color));

            let mut kf2 = HashMap::new();
            kf2.insert("scale-x".to_string(), AnimPropertyValue::Float(1.0));
            kf2.insert("scale-y".to_string(), AnimPropertyValue::Float(1.0));
            kf2.insert("border-width".to_string(), AnimPropertyValue::Float(1.0));
            kf2.insert("border-color".to_string(), AnimPropertyValue::Color(config.border_color));

            vec![AnimationDef {
                id: "preset-pulse".to_string(),
                keyframes: vec![
                    AnimationKeyframe { at: 0.0, properties: kf0 },
                    AnimationKeyframe { at: 0.5, properties: kf1 },
                    AnimationKeyframe { at: 1.0, properties: kf2 },
                ],
                duration: dur,
                easing: "sine".to_string(),
                loop_mode: true,
                ping_pong: false,
            }]
        }
        "float" => {
            let mut kf0 = HashMap::new();
            kf0.insert("offset-y".to_string(), AnimPropertyValue::Float(0.0));
            let mut kf1 = HashMap::new();
            kf1.insert("offset-y".to_string(), AnimPropertyValue::Float(-6.0));
            let mut kf2 = HashMap::new();
            kf2.insert("offset-y".to_string(), AnimPropertyValue::Float(6.0));
            let mut kf3 = HashMap::new();
            kf3.insert("offset-y".to_string(), AnimPropertyValue::Float(0.0));

            vec![AnimationDef {
                id: "preset-float".to_string(),
                keyframes: vec![
                    AnimationKeyframe { at: 0.0, properties: kf0 },
                    AnimationKeyframe { at: 0.25, properties: kf1 },
                    AnimationKeyframe { at: 0.75, properties: kf2 },
                    AnimationKeyframe { at: 1.0, properties: kf3 },
                ],
                duration: dur * 2.0,
                easing: "sine".to_string(),
                loop_mode: true,
                ping_pong: false,
            }]
        }
        "bounce" => {
            let mut kf0 = HashMap::new();
            kf0.insert("offset-y".to_string(), AnimPropertyValue::Float(0.0));
            let mut kf1 = HashMap::new();
            kf1.insert("offset-y".to_string(), AnimPropertyValue::Float(-8.0));
            let mut kf2 = HashMap::new();
            kf2.insert("offset-y".to_string(), AnimPropertyValue::Float(0.0));

            vec![AnimationDef {
                id: "preset-bounce".to_string(),
                keyframes: vec![
                    AnimationKeyframe { at: 0.0, properties: kf0 },
                    AnimationKeyframe { at: 0.5, properties: kf1 },
                    AnimationKeyframe { at: 1.0, properties: kf2 },
                ],
                duration: dur * 0.8,
                easing: "bounce".to_string(),
                loop_mode: true,
                ping_pong: false,
            }]
        }
        "shimmer" => {
            let mut kf0 = HashMap::new();
            kf0.insert("fill".to_string(), AnimPropertyValue::Color(config.fill_color));
            let mut kf1 = HashMap::new();
            kf1.insert("fill".to_string(), AnimPropertyValue::Color(lerp_color(config.fill_color, config.accent_color, 0.35)));
            let mut kf2 = HashMap::new();
            kf2.insert("fill".to_string(), AnimPropertyValue::Color(config.fill_color));

            vec![AnimationDef {
                id: "preset-shimmer".to_string(),
                keyframes: vec![
                    AnimationKeyframe { at: 0.0, properties: kf0 },
                    AnimationKeyframe { at: 0.5, properties: kf1 },
                    AnimationKeyframe { at: 1.0, properties: kf2 },
                ],
                duration: dur * 1.5,
                easing: "sine".to_string(),
                loop_mode: true,
                ping_pong: false,
            }]
        }
        "slide_in_left" => {
            let mut kf0 = HashMap::new();
            kf0.insert("offset-x".to_string(), AnimPropertyValue::Float(-15.0));
            kf0.insert("opacity".to_string(), AnimPropertyValue::Float(0.2));
            let mut kf1 = HashMap::new();
            kf1.insert("offset-x".to_string(), AnimPropertyValue::Float(0.0));
            kf1.insert("opacity".to_string(), AnimPropertyValue::Float(1.0));

            vec![AnimationDef {
                id: "preset-slide-in-left".to_string(),
                keyframes: vec![
                    AnimationKeyframe { at: 0.0, properties: kf0 },
                    AnimationKeyframe { at: 1.0, properties: kf1 },
                ],
                duration: dur * 0.6,
                easing: "ease-out".to_string(),
                loop_mode: false,
                ping_pong: false,
            }]
        }
        "slide_in_right" => {
            let mut kf0 = HashMap::new();
            kf0.insert("offset-x".to_string(), AnimPropertyValue::Float(15.0));
            kf0.insert("opacity".to_string(), AnimPropertyValue::Float(0.2));
            let mut kf1 = HashMap::new();
            kf1.insert("offset-x".to_string(), AnimPropertyValue::Float(0.0));
            kf1.insert("opacity".to_string(), AnimPropertyValue::Float(1.0));

            vec![AnimationDef {
                id: "preset-slide-in-right".to_string(),
                keyframes: vec![
                    AnimationKeyframe { at: 0.0, properties: kf0 },
                    AnimationKeyframe { at: 1.0, properties: kf1 },
                ],
                duration: dur * 0.6,
                easing: "ease-out".to_string(),
                loop_mode: false,
                ping_pong: false,
            }]
        }
        _ => Vec::new(),
    }
}

/// Evaluates a single `AnimationDef` for the given time `time` (seconds) and returns current properties.
pub fn evaluate_animation(
    anim: &AnimationDef,
    time: f64,
    config: &UiThemeConfig,
    data_ctx: &DataContext,
) -> HashMap<String, AnimPropertyValue> {
    let mut results = HashMap::new();
    if anim.keyframes.is_empty() {
        return results;
    }
    if anim.keyframes.len() == 1 {
        return anim.keyframes[0].properties.clone();
    }

    let mut t = if anim.duration <= 0.0 {
        0.0
    } else if anim.loop_mode {
        let cycles = time / anim.duration as f64;
        let frac = (cycles - cycles.floor()) as f32;
        if anim.ping_pong && (cycles.floor() as i64) % 2 == 1 {
            1.0 - frac
        } else {
            frac
        }
    } else {
        ((time / anim.duration as f64) as f32).clamp(0.0, 1.0)
    };

    t = apply_easing(&anim.easing, t);

    let mut kf0 = &anim.keyframes[0];
    let mut kf1 = &anim.keyframes[anim.keyframes.len() - 1];

    if t <= kf0.at {
        return kf0.properties.clone();
    }
    if t >= kf1.at {
        return kf1.properties.clone();
    }

    for i in 0..(anim.keyframes.len() - 1) {
        if anim.keyframes[i].at <= t && t <= anim.keyframes[i + 1].at {
            kf0 = &anim.keyframes[i];
            kf1 = &anim.keyframes[i + 1];
            break;
        }
    }

    let local_t = if (kf1.at - kf0.at).abs() > 0.0001 {
        ((t - kf0.at) / (kf1.at - kf0.at)).clamp(0.0, 1.0)
    } else {
        0.0
    };

    for (prop, val1) in &kf1.properties {
        let val0 = kf0.properties.get(prop).unwrap_or(val1);
        match (val0, val1) {
            (AnimPropertyValue::Float(f0), AnimPropertyValue::Float(f1)) => {
                results.insert(prop.clone(), AnimPropertyValue::Float(f0 + (f1 - f0) * local_t));
            }
            (AnimPropertyValue::Color(c0), AnimPropertyValue::Color(c1)) => {
                results.insert(prop.clone(), AnimPropertyValue::Color(lerp_color(*c0, *c1, local_t)));
            }
            (AnimPropertyValue::StringVal(s0), AnimPropertyValue::StringVal(s1)) => {
                let resolved0 = data_ctx.resolve_text(s0);
                let resolved1 = data_ctx.resolve_text(s1);
                // Attempt to parse string values as hex/color or float for interpolation
                if let (Ok(f0), Ok(f1)) = (resolved0.parse::<f32>(), resolved1.parse::<f32>()) {
                    results.insert(prop.clone(), AnimPropertyValue::Float(f0 + (f1 - f0) * local_t));
                } else {
                    let c0 = parse_hex_color_alpha(&resolved0, config.fill_color);
                    let c1 = parse_hex_color_alpha(&resolved1, config.fill_color);
                    results.insert(prop.clone(), AnimPropertyValue::Color(lerp_color(c0, c1, local_t)));
                }
            }
            (AnimPropertyValue::Color(c0), AnimPropertyValue::StringVal(s1)) => {
                let resolved1 = data_ctx.resolve_text(s1);
                let c1 = parse_hex_color_alpha(&resolved1, *c0);
                results.insert(prop.clone(), AnimPropertyValue::Color(lerp_color(*c0, c1, local_t)));
            }
            (AnimPropertyValue::StringVal(s0), AnimPropertyValue::Color(c1)) => {
                let resolved0 = data_ctx.resolve_text(s0);
                let c0 = parse_hex_color_alpha(&resolved0, *c1);
                results.insert(prop.clone(), AnimPropertyValue::Color(lerp_color(c0, *c1, local_t)));
            }
            _ => {
                results.insert(prop.clone(), val1.clone());
            }
        }
    }
    results
}

/// Resolves style overrides for the given node by evaluating its custom animations and/or preset effects.
pub fn resolve_anim_overrides(
    node: &UiNode,
    config: &UiThemeConfig,
    time: f64,
    scale: f32,
    data_ctx: &DataContext,
) -> AnimStyleOverrides {
    let mut overrides = AnimStyleOverrides {
        scale_x: 1.0,
        scale_y: 1.0,
        ..Default::default()
    };

    let effect = node.effect();
    let has_preset_or_named = effect != "none" && !effect.is_empty();
    let has_inline = !node.animations().is_empty();

    if !has_preset_or_named && !has_inline && (config.animation_effect == "none" || config.animation_effect.is_empty()) {
        return overrides;
    }

    let mut defs_to_run = Vec::new();

    // 1. Check if node.effect() is a named animation defined in the plugin config
    if has_preset_or_named {
        if let Some(named_anim) = config.animation_defs.get(effect) {
            defs_to_run.push(named_anim.clone());
        } else {
            // Otherwise check preset expansion
            let expanded = expand_preset_effect(effect, node.speed(), config);
            defs_to_run.extend(expanded);
        }
    } else if config.animation_effect != "none" && !config.animation_effect.is_empty() {
        if let Some(named_anim) = config.animation_defs.get(&config.animation_effect) {
            defs_to_run.push(named_anim.clone());
        } else {
            let expanded = expand_preset_effect(&config.animation_effect, config.animation_speed, config);
            defs_to_run.extend(expanded);
        }
    }

    // 2. Add any inline animations directly attached to the node
    defs_to_run.extend_from_slice(node.animations());

    for anim in &defs_to_run {
        let props = evaluate_animation(anim, time, config, data_ctx);
        for (prop, val) in props {
            match prop.as_str() {
                "offset-x" => if let AnimPropertyValue::Float(f) = val { overrides.offset_x += f * scale; }
                "offset-y" => if let AnimPropertyValue::Float(f) = val { overrides.offset_y += f * scale; }
                "opacity" => if let AnimPropertyValue::Float(f) = val { overrides.opacity = Some(f); }
                "scale-x" => if let AnimPropertyValue::Float(f) = val { overrides.scale_x *= f; }
                "scale-y" => if let AnimPropertyValue::Float(f) = val { overrides.scale_y *= f; }
                "rotation" => if let AnimPropertyValue::Float(f) = val { overrides.rotation += f; }
                "border-color" => if let AnimPropertyValue::Color(c) = val { overrides.border_color = Some(c); }
                "border-width" => if let AnimPropertyValue::Float(f) = val { overrides.border_width = Some(f * scale); }
                "fill" => if let AnimPropertyValue::Color(c) = val { overrides.fill = Some(c); }
                "text-color" => if let AnimPropertyValue::Color(c) = val { overrides.text_color = Some(c); }
                "rounding" => if let AnimPropertyValue::Float(f) = val { overrides.rounding = Some(f * scale); }
                "padding" => if let AnimPropertyValue::Float(f) = val { overrides.padding = Some(f * scale); }
                "size" => if let AnimPropertyValue::Float(f) = val { overrides.size = Some(f * scale); }
                _ => {}
            }
        }
    }

    overrides
}
