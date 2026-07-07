use crate::plugin_ui_core::UiThemeConfig;
use eframe::egui;

/// Draws complex, dynamic backgrounds (gradients, grids, matrix rain, twinkling stars, SVG/images)
pub fn draw_complex_background(ctx: &egui::Context, config: &UiThemeConfig, time: f64) {
    if config.background_type == "solid" {
        return;
    }
    let rect = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());

    match config.background_type.as_str() {
        "gradient" => {
            let steps = 30;
            let step_h = rect.height() / steps as f32;
            for i in 0..steps {
                let t = i as f32 / (steps - 1) as f32;
                let col = lerp_color(config.fill_color, config.background_color_2, t);
                let r = egui::Rect::from_min_size(
                    egui::pos2(rect.min.x, rect.min.y + i as f32 * step_h),
                    egui::vec2(rect.width(), step_h + 1.0),
                );
                painter.rect_filled(r, 0.0, col);
            }
        }
        "grid" => {
            painter.rect_filled(rect, 0.0, config.fill_color);
            let grid_size = 35.0;
            let mut x = rect.min.x;
            while x < rect.max.x {
                painter.line_segment(
                    [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                    egui::Stroke::new(1.0, config.background_color_2),
                );
                x += grid_size;
            }
            let mut y = rect.min.y;
            while y < rect.max.y {
                painter.line_segment(
                    [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                    egui::Stroke::new(1.0, config.background_color_2),
                );
                y += grid_size;
            }
        }
        "matrix" => {
            painter.rect_filled(rect, 0.0, config.fill_color);
            ctx.request_repaint();
            let num_drops = (rect.width() / 25.0) as usize;
            for i in 0..num_drops {
                let x = rect.min.x + i as f32 * 25.0 + 10.0;
                let speed = config.background_speed * (100.0 + (i % 5) as f32 * 30.0);
                let y = (time as f32 * speed + (i * 137) as f32) % (rect.height() + 200.0) - 100.0;
                painter.line_segment(
                    [egui::pos2(x, y - 40.0), egui::pos2(x, y)],
                    egui::Stroke::new(2.0, config.background_color_2),
                );
                painter.circle_filled(egui::pos2(x, y), 2.5, config.accent_color);
            }
        }
        "stars" => {
            painter.rect_filled(rect, 0.0, config.fill_color);
            ctx.request_repaint();
            for i in 0..60 {
                let x = rect.min.x + ((i * 313) % (rect.width() as usize + 1)) as f32;
                let y = rect.min.y + ((i * 701) % (rect.height() as usize + 1)) as f32;
                let twinkle = ((time * config.background_speed as f64 + i as f64).sin() * 0.5 + 0.5) as f32;
                let radius = 1.0 + twinkle * 1.5;
                let col = lerp_color(config.background_color_2, config.accent_color, twinkle);
                painter.circle_filled(egui::pos2(x, y), radius, col);
            }
        }
        "image" | "svg" => {
            painter.rect_filled(rect, 0.0, config.fill_color);
            if !config.background_url.is_empty() {
                egui::Area::new(egui::Id::new("bg_image_area"))
                    .order(egui::Order::Background)
                    .fixed_pos(rect.min)
                    .show(ctx, |ui| {
                        let img = egui::Image::new(&config.background_url)
                            .fit_to_exact_size(rect.size());
                        ui.add(img);
                    });
            }
        }
        _ => {
            painter.rect_filled(rect, 0.0, config.fill_color);
        }
    }
}

pub fn lerp_color(c1: egui::Color32, c2: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    egui::Color32::from_rgba_unmultiplied(
        (c1.r() as f32 + (c2.r() as f32 - c1.r() as f32) * t) as u8,
        (c1.g() as f32 + (c2.g() as f32 - c1.g() as f32) * t) as u8,
        (c1.b() as f32 + (c2.b() as f32 - c1.b() as f32) * t) as u8,
        (c1.a() as f32 + (c2.a() as f32 - c1.a() as f32) * t) as u8,
    )
}
