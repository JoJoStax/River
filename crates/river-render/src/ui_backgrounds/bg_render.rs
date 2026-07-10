//! Generic interpreter: walks a `BackgroundSpec` and draws it with egui's `Painter`.

use super::bg_spec::{base_env, BackgroundSpec, ImageFit, Layer, ShapeKind, SpecError};
use eframe::egui::{self, Color32, Rect};
use std::collections::HashMap;
use std::sync::Mutex;

/// Caches parsed specs by a caller-supplied key so KDL parsing happens once.
#[derive(Default)]
pub struct BackgroundCache {
    inner: Mutex<HashMap<String, BackgroundSpec>>,
}

impl BackgroundCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a cached spec or parse+insert it if absent/changed.
    pub fn get_or_parse(&self, key: &str, kdl_src: &str) -> Result<BackgroundSpec, SpecError> {
        {
            let map = self.inner.lock().unwrap();
            if let Some(spec) = map.get(key) {
                return Ok(spec.clone());
            }
        }
        let spec = super::bg_spec::parse_background_kdl(kdl_src)?;
        self.inner.lock().unwrap().insert(key.to_string(), spec.clone());
        Ok(spec)
    }

    pub fn invalidate(&self, key: &str) {
        self.inner.lock().unwrap().remove(key);
    }
}


/// Draws a parsed background spec into the given screen rect.
pub fn render_background(ctx: &egui::Context, painter: &egui::Painter, rect: Rect, spec: &BackgroundSpec, time: f64) {
    for layer in &spec.layers {
        render_layer(ctx, painter, rect, layer, time);
    }
}

fn render_layer(ctx: &egui::Context, painter: &egui::Painter, rect: Rect, layer: &Layer, time: f64) {
    let n = layer.count;
    let w = rect.width();
    let h = rect.height();

    match layer.shape {
        ShapeKind::Image => {
            if let Some(url) = &layer.image_url {
                egui::Area::new(egui::Id::new(("bg_image_layer", url.as_str())))
                    .order(egui::Order::Background)
                    .fixed_pos(rect.min)
                    .show(ctx, |ui| {
                        let img = egui::Image::new(url.as_str());
                        let img = match layer.image_fit {
                            ImageFit::Stretch => img.fit_to_exact_size(rect.size()),
                            ImageFit::Cover => img.fit_to_exact_size(rect.size()),
                            ImageFit::Contain => img.max_size(rect.size()),
                        };
                        ui.add(img);
                    });
            }
            return;
        }
        _ => {}
    }

    for i in 0..n {
        let env = base_env(time, layer.speed, i, n, w, h)
            .set("rect_x", rect.min.x as f64)
            .set("rect_y", rect.min.y as f64);

        let color = layer.color.eval(&env);
        if color.a() == 0 {
            continue; // fully transparent, skip drawing
        }

        match layer.shape {
            ShapeKind::Fill => {
                painter.rect_filled(rect, 0.0, color);
            }
            ShapeKind::Rect => {
                let x = rect.min.x + layer.x.eval(&env).unwrap_or(0.0) as f32;
                let y = rect.min.y + layer.y.eval(&env).unwrap_or(0.0) as f32;
                let width = layer.width.eval(&env).unwrap_or(0.0) as f32;
                let height = layer.height.eval(&env).unwrap_or(0.0) as f32;
                let r = Rect::from_min_size(egui::pos2(x, y), egui::vec2(width, height));
                painter.rect_filled(r, 0.0, color);
            }
            ShapeKind::Line => {
                let x1 = rect.min.x + layer.x.eval(&env).unwrap_or(0.0) as f32;
                let y1 = rect.min.y + layer.y.eval(&env).unwrap_or(0.0) as f32;
                let x2 = rect.min.x + layer.x2.eval(&env).unwrap_or(0.0) as f32;
                let y2 = rect.min.y + layer.y2.eval(&env).unwrap_or(0.0) as f32;
                let sw = layer.stroke_width.eval(&env).unwrap_or(1.0) as f32;
                painter.line_segment([egui::pos2(x1, y1), egui::pos2(x2, y2)], egui::Stroke::new(sw, color));
            }
            ShapeKind::Circle => {
                let x = rect.min.x + layer.x.eval(&env).unwrap_or(0.0) as f32;
                let y = rect.min.y + layer.y.eval(&env).unwrap_or(0.0) as f32;
                let r = layer.radius.eval(&env).unwrap_or(1.0) as f32;
                painter.circle_filled(egui::pos2(x, y), r.max(0.0), color);
            }
            ShapeKind::Image => {} // handled earlier
        }
    }
}

/// Convenience entry point: parse-or-fetch from cache, then render.
pub fn draw_kdl_background(
    ctx: &egui::Context,
    cache: &BackgroundCache,
    cache_key: &str,
    kdl_src: &str,
    time: f64,
) {
    let spec = match cache.get_or_parse(cache_key, kdl_src) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("[background] failed to parse '{cache_key}': {err}");
            let rect = ctx.screen_rect();
            let painter = ctx.layer_painter(egui::LayerId::background());
            painter.rect_filled(rect, 0.0, Color32::from_rgb(20, 20, 24));
            return;
        }
    };
    if spec.references_time() {
        ctx.request_repaint();
    }
    let rect = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    render_background(ctx, &painter, rect, &spec, time);
}
