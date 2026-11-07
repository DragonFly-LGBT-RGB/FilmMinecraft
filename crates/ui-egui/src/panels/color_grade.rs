//! Lumetri color grading panel: exposure, contrast, highlights, shadows, whites, blacks,
//! temperature, tint, vibrance, saturation, and curves.

use egui::{Align2, Color32, Rect, Sense, vec2, pos2};
use serde_json::json;

use crate::FilmcraftApp;
use crate::icons::Icon;
use crate::theme::Tokens;

/// A color control with label, value, range, and icon.
#[derive(Clone)]
struct ColorControl {
    name: &'static str,
    value: f32,
    min: f32,
    max: f32,
    default: f32,
    icon: Icon,
    color: Color32,
    unit: &'static str,
}

const CONTROLS: [ColorControl; 10] = [
    ColorControl { name: "Exposure", value: 0.0, min: -2.0, max: 2.0, default: 0.0, icon: Icon::Adjust, color: Color32::from_rgb(255, 100, 100), unit: "EV" },
    ColorControl { name: "Contrast", value: 0.0, min: -100.0, max: 100.0, default: 0.0, icon: Icon::Adjust, color: Color32::from_rgb(255, 150, 100), unit: "%" },
    ColorControl { name: "Highlights", value: 0.0, min: -100.0, max: 100.0, default: 0.0, icon: Icon::Adjust, color: Color32::from_rgb(255, 200, 100), unit: "%" },
    ColorControl { name: "Shadows", value: 0.0, min: -100.0, max: 100.0, default: 0.0, icon: Icon::Adjust, color: Color32::from_rgb(100, 150, 255), unit: "%" },
    ColorControl { name: "Whites", value: 0.0, min: -100.0, max: 100.0, default: 0.0, icon: Icon::Adjust, color: Color32::from_rgb(255, 255, 255), unit: "%" },
    ColorControl { name: "Blacks", value: 0.0, min: -100.0, max: 100.0, default: 0.0, icon: Icon::Adjust, color: Color32::from_rgb(50, 50, 50), unit: "%" },
    ColorControl { name: "Temperature", value: 0.0, min: -50.0, max: 50.0, default: 0.0, icon: Icon::Adjust, color: Color32::from_rgb(255, 180, 100), unit: "K" },
    ColorControl { name: "Tint", value: 0.0, min: -50.0, max: 50.0, default: 0.0, icon: Icon::Adjust, color: Color32::from_rgb(200, 100, 255), unit: "" },
    ColorControl { name: "Vibrance", value: 0.0, min: -100.0, max: 100.0, default: 0.0, icon: Icon::Adjust, color: Color32::from_rgb(100, 255, 150), unit: "%" },
    ColorControl { name: "Saturation", value: 0.0, min: -100.0, max: 100.0, default: 0.0, icon: Icon::Adjust, color: Color32::from_rgb(100, 200, 255), unit: "%" },
];

pub fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;

    ui.painter().rect_filled(rect, t.radius, t.panel_bg);
    ui.painter().text(
        rect.min + vec2(16.0, 18.0),
        Align2::LEFT_CENTER,
        tl!("Lumetri Color"),
        Tokens::semibold(16.0),
        t.text,
    );

    // Presets row.
    let presets = ["None", "Cinematic", "Vibrant", "Black & White", "Warm", "Cool"];
    let mut px = rect.min.x + 16.0;
    for preset in presets.iter() {
        let pr = Rect::from_min_size(pos2(px, rect.min.y + 52.0), vec2(70.0, 28.0));
        let hovered = ui.interact(pr, egui::Id::unique(("luma_preset", preset)), Sense::click()).hovered();
        let clicked = ui.interact(pr, egui::Id::unique(("luma_preset_click", preset)), Sense::click()).clicked();
        ui.painter().rect_filled(pr, 6.0, if hovered || clicked { Color32::from_rgb(60, 120, 255) } else { Color32::from_rgb(32, 36, 46) });
        ui.painter().text(pr.center(), Align2::CENTER_CENTER, *preset, Tokens::ui(11.0), Color32::WHITE);
        if clicked {
            let _ = crate::menus::invoke(app, &ui.ctx(), "color.applyPreset", json!({"preset": preset}));
            app.ui.status = format!("Applied {} preset", preset);
        }
        px += 80.0;
    }

    // Controls list.
    let list_rect = Rect::from_min_size(rect.min + vec2(16.0, 92.0), vec2(rect.width() - 32.0, rect.height() - 108.0));
    ui.painter().rect_filled(list_rect, 8.0, Color32::from_rgb(24, 28, 36));

    let mut cy = list_rect.min.y + 16.0;
    for control in CONTROLS.iter() {
        let cr = Rect::from_min_size(list_rect.min + vec2(12.0, cy), vec2(list_rect.width() - 24.0, 48.0));
        let hovered = ui.interact(cr, egui::Id::unique(("luma", control.name)), Sense::click()).hovered();

        ui.painter().rect_filled(cr, 6.0, if hovered { Color32::from_rgb(40, 44, 56) } else { Color32::TRANSPARENT });

        // Label row.
        ui.painter().text(cr.min + vec2(12.0, 6.0), Align2::LEFT_CENTER, control.name, Tokens::semibold(12.0), Color32::WHITE);
        ui.painter().text(cr.max - vec2(40.0, 6.0), Align2::RIGHT_CENTER, format!("{}{}", control.value, control.unit).as_str(), Tokens::ui(11.0), Color32::from_rgb(140, 140, 150));

        // Slider bar.
        let slider_r = Rect::from_min_size(cr.min + vec2(12.0, 26.0), vec2(cr.width() - 24.0, 6.0));
        let pct = ((control.value - control.min) / (control.max - control.min)).clamp(0.0, 1.0);
        ui.painter().rect_filled(slider_r, 3.0, Color32::from_rgb(60, 60, 70));
        let fill_w = slider_r.width() * pct;
        ui.painter().rect_filled(Rect::from_min_size(slider_r.min, vec2(fill_w, 6.0)), 3.0, control.color);

        // Slider handle.
        let handle_x = slider_r.min.x + fill_w - 6.0;
        let handle_r = Rect::from_min_size(pos2(handle_x, slider_r.min.y - 5.0), vec2(14.0, 16.0));
        ui.painter().rect_filled(handle_r, 4.0, control.color);
        ui.painter().rect_stroke(handle_r, 4.0, egui::Stroke::new(1.0, Color32::WHITE), egui::StrokeKind::Inside);

        // Reset button.
        let reset_r = Rect::from_min_size(cr.max - vec2(28.0, 20.0), vec2(24.0, 20.0));
        let reset_h = ui.interact(reset_r, egui::Id::unique(("luma_reset", control.name)), Sense::click()).hovered();
        let reset_c = ui.interact(reset_r, egui::Id::unique(("luma_reset_click", control.name)), Sense::click()).clicked();
        ui.painter().rect_filled(reset_r, 4.0, if reset_h { Color32::from_rgb(60, 60, 70) } else { Color32::TRANSPARENT });
        ui.painter().text(reset_r.center(), Align2::CENTER_CENTER, "↻", Tokens::ui(10.0), Color32::from_rgb(120, 120, 130));
        if reset_c {
            let _ = crate::menus::invoke(app, &ui.ctx(), "color.reset", json!({"param": control.name}));
        }

        // Capture drag on slider.
        if ui.interact(slider_r, egui::Id::unique(("luma_slider", control.name)), Sense::drag()).dragged() {
            let delta = ui.input(|i| i.pointer.delta());
            let new_val = control.value + delta.x * 0.1;
            let clamped = new_val.clamp(control.min, control.max);
            let _ = crate::menus::invoke(app, &ui.ctx(), "color.setParam", json!({"param": control.name, "value": clamped}));
            app.ui.status = format!("{} = {}", control.name, clamped);
        }

        cy += 54.0;
    }

    // Curves preview (bottom).
    let curves_r = Rect::from_min_size(list_rect.min + vec2(12.0, cy + 12.0), vec2(list_rect.width() - 24.0, 80.0));
    ui.painter().rect_filled(curves_r, 6.0, Color32::from_rgb(32, 36, 46));
    ui.painter().rect_stroke(curves_r, 6.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);
    ui.painter().text(curves_r.min + vec2(8.0, 10.0), Align2::LEFT_CENTER, "RGB Curves", Tokens::ui(11.0), Color32::from_rgb(150, 150, 160));

    // Draw curve graph.
    let graph_r = Rect::from_min_size(curves_r.min + vec2(8.0, 30.0), vec2(curves_r.width() - 16.0, 40.0));
    ui.painter().rect_filled(graph_r, 3.0, Color32::from_rgb(24, 28, 36));
    // Diagonal line.
    ui.painter().line_segment([graph_r.min, graph_r.max], egui::Stroke::new(1.0, Color32::from_rgb(80, 80, 90)));
}
