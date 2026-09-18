use super::*;
use eframe::egui;
use river_engine::PdfCmd;
use std::sync::Arc;

pub fn register(registry: &mut RenderFunctionRegistry) {
    registry.register(
        "pdf-viewer:viewport",
        Arc::new(|ui, ctx, props, _children| {
            let height = prop_f32(props, "height", 400.0) * ctx.scale;
            let show_controls = prop_bool(props, "show_controls", true);
            let bg_color = prop_color(props, "fill", ctx.config, ctx.config.fill_color);
            let border_color = prop_color(props, "border", ctx.config, ctx.config.border_color);
            let border_width = prop_f32(props, "border_width", ctx.config.border_width);
            let rounding = prop_f32(props, "rounding", ctx.config.rounding);

            let pdf = &ctx.engine.pdf_viewer;
            let state = pdf.state();

            let avail_width = ui.available_width();
            let (rect, _response) = ui.allocate_exact_size(
                egui::vec2(avail_width, height),
                egui::Sense::click(),
            );

            ui.painter().rect(
                rect,
                rounding,
                bg_color,
                egui::Stroke::new(border_width * ctx.scale, border_color),
            );

            let text_color = prop_color(props, "text_color", ctx.config, ctx.config.text_color);
            let page_info = format!("PDF Viewer — Page {}/{}", state.current_page + 1, state.total_pages);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                &page_info,
                egui::FontId::proportional(15.0 * ctx.scale),
                text_color,
            );

            if show_controls {
                ui.horizontal(|ui| {
                    if ui.button("◀ Prev Page").clicked() {
                        pdf.send(PdfCmd::PrevPage);
                    }
                    if ui.button("Next Page ▶").clicked() {
                        pdf.send(PdfCmd::NextPage);
                    }
                    if prop_bool(props, "show_zoom", true) {
                        let secondary = prop_color(props, "secondary", ctx.config, ctx.config.secondary_color);
                        ui.label(egui::RichText::new(format!("Zoom: {:.1}x", state.zoom)).color(secondary));
                    }
                });
            }
        }),
    );
}
