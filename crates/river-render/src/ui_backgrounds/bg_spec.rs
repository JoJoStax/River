//! Declarative background framework.
//!
//! Plugin authors write KDL describing one or more *layers*. Each layer has a
//! *shape kind* (rect / line / circle / fullscreen-gradient / image) and is
//! repeated `count` times.
//! Each shape's geometry and color are driven by expressions evaluated with
//! a small set of built-in variables:
//!
//!   time     - seconds since app start (or since background loaded)
//!   speed    - the layer's `speed` value (defaults to 1.0)
//!   i        - index of this repetition within the layer (0-based)
//!   n        - total repetitions in this layer (`count`)
//!   w        - viewport width in points
//!   h        - viewport height in points
//!   t        - i / max(n-1, 1), i.e. 0..1 progress through the layer

use super::bg_color::{parse_color};
use super::expr::{Env, Expr};
use eframe::egui::Color32;
use kdl::{KdlDocument, KdlNode, KdlValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    /// Fills the whole background once (respects color/gradient).
    Fill,
    Rect,
    Line,
    Circle,
    /// Draws `image_url` stretched/fit to the viewport.
    Image,
}

/// A color value that's either fixed or animated between two colors via an expression.
#[derive(Debug, Clone)]
pub enum ColorSpec {
    Fixed(Color32),
    Mix { from: Color32, to: Color32, mix: Expr },
}

impl ColorSpec {
    pub(crate) fn eval(&self, env: &Env) -> Color32 {
        match self {
            ColorSpec::Fixed(c) => *c,
            ColorSpec::Mix { from, to, mix } => {
                let t = mix.eval(env).unwrap_or(0.0) as f32;
                super::bg_color::lerp_color(*from, *to, t)
            }
        }
    }

    pub fn references_time(&self) -> bool {
        match self {
            ColorSpec::Fixed(_) => false,
            ColorSpec::Mix { mix, .. } => mix.references_time(),
        }
    }
}

/// One repeated shape layer.
#[derive(Debug, Clone)]
pub struct Layer {
    pub shape: ShapeKind,
    pub count: usize,
    pub speed: f64,

    // Geometry expressions (interpreted per-shape according to `shape`).
    // Rect/Fill: x, y, w, h define the rect. Line: x,y -> x2,y2. Circle: x,y,radius.
    pub x: Expr,
    pub y: Expr,
    pub x2: Expr,
    pub y2: Expr,
    pub width: Expr,
    pub height: Expr,
    pub radius: Expr,
    pub stroke_width: Expr,

    pub color: ColorSpec,
    pub image_url: Option<String>,
    pub image_fit: ImageFit,
}

impl Layer {
    pub fn references_time(&self) -> bool {
        self.x.references_time()
            || self.y.references_time()
            || self.x2.references_time()
            || self.y2.references_time()
            || self.width.references_time()
            || self.height.references_time()
            || self.radius.references_time()
            || self.stroke_width.references_time()
            || self.color.references_time()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFit {
    Cover,
    Contain,
    Stretch,
}

#[derive(Debug, Clone)]
pub struct BackgroundSpec {
    pub background_type: String,
    pub layers: Vec<Layer>,
}

impl BackgroundSpec {
    pub fn references_time(&self) -> bool {
        self.layers.iter().any(|l| l.references_time())
    }
}

#[derive(Debug)]
pub enum SpecError {
    Kdl(String),
    Expr(String),
    Missing(&'static str),
}

impl std::fmt::Display for SpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpecError::Kdl(s) => write!(f, "KDL parse error: {s}"),
            SpecError::Expr(s) => write!(f, "expression error: {s}"),
            SpecError::Missing(s) => write!(f, "missing required field: {s}"),
        }
    }
}
impl std::error::Error for SpecError {}

impl From<kdl::KdlError> for SpecError {
    fn from(e: kdl::KdlError) -> Self {
        SpecError::Kdl(e.to_string())
    }
}
impl From<super::expr::ExprError> for SpecError {
    fn from(e: super::expr::ExprError) -> Self {
        SpecError::Expr(e.to_string())
    }
}

/// Parse a plugin's background KDL into a spec. Do this once, then reuse the resulting
/// `BackgroundSpec` every frame via `render_background`.
pub fn parse_background_kdl(src: &str) -> Result<BackgroundSpec, SpecError> {
    let doc: KdlDocument = src.parse()?;
    let bg_node = doc
        .nodes()
        .iter()
        .find(|n| n.name().value() == "background")
        .ok_or(SpecError::Missing("background node"))?;

    let background_type = get_string_arg(bg_node, "type").unwrap_or_else(|| "custom".to_string());

    let mut layers = Vec::new();
    if let Some(children) = bg_node.children() {
        for node in children.nodes() {
            if node.name().value() == "layer" {
                layers.push(parse_layer(node)?);
            }
        }
    }

    Ok(BackgroundSpec { background_type, layers })
}

fn parse_layer(node: &KdlNode) -> Result<Layer, SpecError> {
    let shape = match get_string_arg(node, "shape").as_deref() {
        Some("rect") => ShapeKind::Rect,
        Some("line") => ShapeKind::Line,
        Some("circle") => ShapeKind::Circle,
        Some("fill") => ShapeKind::Fill,
        Some("image") => ShapeKind::Image,
        Some(other) => return Err(SpecError::Kdl(format!("unknown shape '{other}'"))),
        None => ShapeKind::Fill,
    };

    let count = get_i64_arg(node, "count").unwrap_or(1).max(1) as usize;
    let speed = get_f64_arg(node, "speed").unwrap_or(1.0);

    // Defaults keep every field optional in KDL; authors only specify what they need.
    let mut x = Expr::parse_or_const("0")?;
    let mut y = Expr::parse_or_const("0")?;
    let mut x2 = Expr::parse_or_const("0")?;
    let mut y2 = Expr::parse_or_const("0")?;
    let mut width = Expr::parse_or_const("w")?;
    let mut height = Expr::parse_or_const("h")?;
    let mut radius = Expr::parse_or_const("2")?;
    let mut stroke_width = Expr::parse_or_const("1")?;
    let mut color = ColorSpec::Fixed(Color32::WHITE);
    let mut image_url: Option<String> = None;
    let mut image_fit = ImageFit::Cover;

    if let Some(children) = node.children() {
        for child in children.nodes() {
            let name = child.name().value();
            match name {
                "x" => x = expr_from_node(child)?,
                "y" => y = expr_from_node(child)?,
                "x2" => x2 = expr_from_node(child)?,
                "y2" => y2 = expr_from_node(child)?,
                "width" => width = expr_from_node(child)?,
                "height" => height = expr_from_node(child)?,
                "radius" => radius = expr_from_node(child)?,
                "stroke_width" => stroke_width = expr_from_node(child)?,
                "color" => color = parse_color_node(child)?,
                "image" => {
                    image_url = get_string_arg(child, "url").or_else(|| first_string_val(child));
                    if let Some(fit) = get_string_arg(child, "fit") {
                        image_fit = match fit.as_str() {
                            "contain" => ImageFit::Contain,
                            "stretch" => ImageFit::Stretch,
                            _ => ImageFit::Cover,
                        };
                    }
                }
                _ => { /* unknown fields are ignored, so specs stay forward-compatible */ }
            }
        }
    }

    Ok(Layer {
        shape,
        count,
        speed,
        x,
        y,
        x2,
        y2,
        width,
        height,
        radius,
        stroke_width,
        color,
        image_url,
        image_fit,
    })
}

fn parse_color_node(node: &KdlNode) -> Result<ColorSpec, SpecError> {
    if let (Some(from), Some(to)) = (get_string_arg(node, "from"), get_string_arg(node, "to")) {
        let from = parse_color(&from).ok_or(SpecError::Missing("valid 'from' color"))?;
        let to = parse_color(&to).ok_or(SpecError::Missing("valid 'to' color"))?;
        let mix = get_string_arg(node, "mix").unwrap_or_else(|| "0".to_string());
        return Ok(ColorSpec::Mix { from, to, mix: Expr::parse_or_const(&mix)? });
    }
    let s = get_string_arg(node, "value").or_else(|| first_string_val(node)).ok_or(SpecError::Missing("color value"))?;
    let c = parse_color(&s).ok_or(SpecError::Missing("valid color"))?;
    Ok(ColorSpec::Fixed(c))
}

fn expr_from_node(node: &KdlNode) -> Result<Expr, SpecError> {
    let s = first_string_val(node)
        .or_else(|| node.entries().first().and_then(|e| e.value().as_integer()).map(|i| i.to_string()))
        .or_else(|| node.entries().first().and_then(|e| e.value().as_float()).map(|f| f.to_string()))
        .ok_or_else(|| SpecError::Kdl(format!("'{}' needs a value", node.name().value())))?;
    Ok(Expr::parse_or_const(&s)?)
}

fn first_string_val(node: &KdlNode) -> Option<String> {
    node.entries().iter().find(|e| e.name().is_none()).and_then(|e| e.value().as_string()).map(|s| s.to_string())
}

fn get_string_arg(node: &KdlNode, key: &str) -> Option<String> {
    node.entries()
        .iter()
        .find(|e| e.name().map(|n| n.value() == key).unwrap_or(false))
        .and_then(|e| match e.value() {
            KdlValue::String(s) => Some(s.clone()),
            _ => None,
        })
}

fn get_i64_arg(node: &KdlNode, key: &str) -> Option<i64> {
    node.entries()
        .iter()
        .find(|e| e.name().map(|n| n.value() == key).unwrap_or(false))
        .and_then(|e| e.value().as_integer().map(|i| i as i64))
}

fn get_f64_arg(node: &KdlNode, key: &str) -> Option<f64> {
    node.entries()
        .iter()
        .find(|e| e.name().map(|n| n.value() == key).unwrap_or(false))
        .and_then(|e| e.value().as_float().or_else(|| e.value().as_integer().map(|i| i as f64)))
}

pub(crate) fn base_env(time: f64, speed: f64, i: usize, n: usize, w: f32, h: f32) -> Env {
    let t = if n > 1 { i as f64 / (n - 1) as f64 } else { 0.0 };
    Env::new()
        .set("time", time)
        .set("speed", speed)
        .set("i", i as f64)
        .set("n", n as f64)
        .set("w", w as f64)
        .set("h", h as f64)
        .set("t", t)
}
