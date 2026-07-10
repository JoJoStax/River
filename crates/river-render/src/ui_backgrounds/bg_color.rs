//! Color parsing for plugin-authored backgrounds.
//! Supports "#RRGGBB", "#RRGGBBAA", and a couple of named colors as a convenience.

use eframe::egui::Color32;

pub fn parse_color(s: &str) -> Option<Color32> {
    let s = s.trim();
    if s.starts_with('#') {
        return crate::plugin_ui_core::parse_hex_color(s);
    }
    // A handful of convenience names; extend as you like.
    match s.to_ascii_lowercase().as_str() {
        "transparent" => Some(Color32::TRANSPARENT),
        "black" => Some(Color32::BLACK),
        "white" => Some(Color32::WHITE),
        "red" => Some(Color32::RED),
        "green" => Some(Color32::GREEN),
        "blue" => Some(Color32::BLUE),
        "yellow" => Some(Color32::YELLOW),
        _ => None,
    }
}

pub fn lerp_color(c1: Color32, c2: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (c1.r() as f32 + (c2.r() as f32 - c1.r() as f32) * t).round() as u8,
        (c1.g() as f32 + (c2.g() as f32 - c1.g() as f32) * t).round() as u8,
        (c1.b() as f32 + (c2.b() as f32 - c1.b() as f32) * t).round() as u8,
        (c1.a() as f32 + (c2.a() as f32 - c1.a() as f32) * t).round() as u8,
    )
}

/// A sequence of color stops, used for gradient fills and "index -> color" ramps
/// (e.g. star twinkle color, matrix drop color).
#[derive(Debug, Clone)]
pub struct ColorRamp {
    pub stops: Vec<(f32, Color32)>, // (position 0..1, color), sorted by position
}

impl ColorRamp {
    pub fn new(mut stops: Vec<(f32, Color32)>) -> Self {
        stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        if stops.is_empty() {
            stops.push((0.0, Color32::WHITE));
        }
        Self { stops }
    }

    pub fn single(c: Color32) -> Self {
        Self { stops: vec![(0.0, c)] }
    }

    pub fn sample(&self, t: f32) -> Color32 {
        let t = t.clamp(0.0, 1.0);
        if self.stops.len() == 1 {
            return self.stops[0].1;
        }
        for w in self.stops.windows(2) {
            let (p0, c0) = w[0];
            let (p1, c1) = w[1];
            if t >= p0 && t <= p1 {
                let local_t = if (p1 - p0).abs() < f32::EPSILON { 0.0 } else { (t - p0) / (p1 - p0) };
                return lerp_color(c0, c1, local_t);
            }
        }
        // past the last stop
        self.stops.last().unwrap().1
    }
}
