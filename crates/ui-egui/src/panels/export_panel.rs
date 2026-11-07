//! Export panel: format presets, resolution, codec, bitrate, destination, and export queue.

use egui::{Align2, Color32, Rect, Sense, vec2, pos2};
use serde_json::json;

use crate::FilmcraftApp;
use crate::icons::Icon;
use crate::theme::Tokens;

/// Export preset.
#[derive(Clone)]
struct Preset {
    name: &'static str,
    description: &'static str,
    icon: Icon,
    color: Color32,
}

const PRESETS: [Preset; 8] = [
    Preset { name: "YouTube 1080p", description: "H.264 • 1920×1080 • 8 Mbps", icon: Icon::Export, color: Color32::from_rgb(255, 0, 0) },
    Preset { name: "YouTube 4K", description: "H.264 • 3840×2160 • 35 Mbps", icon: Icon::Export, color: Color32::from_rgb(255, 0, 0) },
    Preset { name: "TikTok/Reels", description: "H.264 • 1080×1920 • 6 Mbps", icon: Icon::Export, color: Color32::from_rgb(0, 0, 0) },
    Preset { name: "Vimeo 1080p", description: "H.264 • 1920×1080 • 10 Mbps", icon: Icon::Export, color: Color32::from_rgb(0, 130, 200) },
    Preset { name: "ProRes 422", description: "ProRes • 1920×1080 • Master", icon: Icon::Export, color: Color32::from_rgb(0, 180, 255) },
    Preset { name: "DNxHR HQ", description: "DNxHR • 1920×1080 • Master", icon: Icon::Export, color: Color32::from_rgb(0, 180, 255) },
    Preset { name: "MP4 Small", description: "H.264 • 1280×720 • 3 Mbps", icon: Icon::Export, color: Color32::from_rgb(100, 200, 255) },
    Preset { name: "GIF", description: "Animated GIF • Looping", icon: Icon::Export, color: Color32::from_rgb(255, 100, 200) },
];

/// Export destination.
#[derive(Clone)]
struct Destination {
    name: &'static str,
    icon: Icon,
    color: Color32,
}

const DESTINATIONS: [Destination; 4] = [
    Destination { name: "Local File", icon: Icon::Folder, color: Color32::from_rgb(100, 180, 255) },
    Destination { name: "YouTube", icon: Icon::Globe, color: Color32::from_rgb(255, 0, 0) },
    Destination { name: "TikTok", icon: Icon::Globe, color: Color32::from_rgb(0, 0, 0) },
    Destination { name: "Vimeo", icon: Icon::Globe, color: Color32::from_rgb(0, 130, 200) },
];

pub fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    let ctx = ui.ctx().clone();

    ui.painter().rect_filled(rect, t.radius, t.panel_bg);
    ui.painter().text(
        rect.min + vec2(16.0, 18.0),
        Align2::LEFT_CENTER,
        tl!("Export"),
        Tokens::semibold(16.0),
        t.text,
    );

    // Preset tabs.
    let mut px = rect.min.x + 16.0;
    for (i, preset) in PRESETS.iter().enumerate() {
        let pr = Rect::from_min_size(pos2(px, rect.min.y + 52.0), vec2(120.0, 32.0));
        let hovered = ui.interact(pr, egui::Id::unique(("exp_preset", i)), Sense::click()).hovered();
        let clicked = ui.interact(pr, egui::Id::unique(("exp_preset_click", i)), Sense::click()).clicked();
        ui.painter().rect_filled(pr, 6.0, if hovered || clicked { preset.color.linear_multiply(0.8) } else { Color32::from_rgb(32, 36, 46) });
        ui.painter().rect_stroke(pr, 6.0, egui::Stroke::new(1.0, preset.color.linear_multiply(0.5)), egui::StrokeKind::Inside);
        crate::icons::paint(ui.painter(), Rect::from_min_size(pr.min + vec2(8.0, 4.0), vec2(20.0, 20.0)), preset.icon, Color32::WHITE);
        ui.painter().text(pr.min + vec2(32.0, 16.0), Align2::LEFT_CENTER, preset.name, Tokens::semibold(11.0), Color32::WHITE);
        ui.painter().text(pr.min + vec2(32.0, 30.0), Align2::LEFT_CENTER, preset.description, Tokens::ui(8.0), Color32::from_rgb(140, 140, 150));
        if clicked {
            let _ = crate::menus::invoke(app, &ctx, "export.setPreset", json!({"preset": preset.name}));
            app.ui.status = format!("Export preset: {}", preset.name);
        }
        px += 130.0;
    }

    // Settings column (left).
    let settings_w = 340.0;
    let settings_rect = Rect::from_min_size(rect.min + vec2(16.0, 96.0), vec2(settings_w, rect.height() - 112.0));
    ui.painter().rect_filled(settings_rect, 8.0, Color32::from_rgb(24, 28, 36));

    // Format.
    ui.painter().text(settings_rect.min + vec2(16.0, 16.0), Align2::LEFT_CENTER, "Format", Tokens::ui(12.0), Color32::from_rgb(160, 160, 170));
    let formats = ["H.264", "H.265 (HEVC)", "ProRes 422", "DNxHR", "AV1"];
    let mut fy = settings_rect.min.y + 36.0;
    for fmt in formats.iter() {
        let fr = Rect::from_min_size(settings_rect.min + vec2(16.0, fy), vec2(settings_w - 32.0, 28.0));
        let hovered = ui.interact(fr, egui::Id::unique(("exp_fmt", fmt)), Sense::click()).hovered();
        ui.painter().rect_filled(fr, 4.0, if hovered { Color32::from_rgb(40, 44, 56) } else { Color32::TRANSPARENT });
        ui.painter().text(fr.min + vec2(10.0, 14.0), Align2::LEFT_CENTER, *fmt, Tokens::ui(12.0), Color32::WHITE);
        fy += 34.0;
    }

    // Resolution.
    ui.painter().text(settings_rect.min + vec2(16.0, fy + 8.0), Align2::LEFT_CENTER, "Resolution", Tokens::ui(12.0), Color32::from_rgb(160, 160, 170));
    fy += 28.0;
    let resolutions = ["1920×1080", "3840×2160", "1080×1920", "1280×720"];
    for res in resolutions.iter() {
        let fr = Rect::from_min_size(settings_rect.min + vec2(16.0, fy), vec2(settings_w - 32.0, 28.0));
        let hovered = ui.interact(fr, egui::Id::unique(("exp_res", res)), Sense::click()).hovered();
        ui.painter().rect_filled(fr, 4.0, if hovered { Color32::from_rgb(40, 44, 56) } else { Color32::TRANSPARENT });
        ui.painter().text(fr.min + vec2(10.0, 14.0), Align2::LEFT_CENTER, *res, Tokens::ui(12.0), Color32::WHITE);
        fy += 34.0;
    }

    // Bitrate.
    ui.painter().text(settings_rect.min + vec2(16.0, fy + 8.0), Align2::LEFT_CENTER, "Bitrate", Tokens::ui(12.0), Color32::from_rgb(160, 160, 170));
    fy += 28.0;
    let bitrates = ["5 Mbps", "10 Mbps", "20 Mbps", "35 Mbps", "50 Mbps", "VBR 2-pass"];
    for br in bitrates.iter() {
        let fr = Rect::from_min_size(settings_rect.min + vec2(16.0, fy), vec2(settings_w - 32.0, 28.0));
        let hovered = ui.interact(fr, egui::Id::unique(("exp_br", br)), Sense::click()).hovered();
        ui.painter().rect_filled(fr, 4.0, if hovered { Color32::from_rgb(40, 44, 56) } else { Color32::TRANSPARENT });
        ui.painter().text(fr.min + vec2(10.0, 14.0), Align2::LEFT_CENTER, *br, Tokens::ui(12.0), Color32::WHITE);
        fy += 34.0;
    }

    // Destination column (right).
    let dest_x = rect.min.x + 16.0 + settings_w + 20.0;
    let dest_w = rect.max.x - dest_x - 16.0;
    let dest_rect = Rect::from_min_size(pos2(dest_x, rect.min.y + 96.0), vec2(dest_w, rect.height() - 112.0));
    ui.painter().rect_filled(dest_rect, 8.0, Color32::from_rgb(24, 28, 36));

    ui.painter().text(dest_rect.min + vec2(16.0, 16.0), Align2::LEFT_CENTER, "Publish to", Tokens::ui(12.0), Color32::from_rgb(160, 160, 170));
    let mut dy = dest_rect.min.y + 36.0;
    for dest in DESTINATIONS.iter() {
        let dr = Rect::from_min_size(dest_rect.min + vec2(16.0, dy), vec2(dest_w - 32.0, 44.0));
        let hovered = ui.interact(dr, egui::Id::unique(("exp_dest", dest.name)), Sense::click()).hovered();
        let clicked = ui.interact(dr, egui::Id::unique(("exp_dest_click", dest.name)), Sense::click()).clicked();
        ui.painter().rect_filled(dr, 6.0, if hovered { Color32::from_rgb(40, 44, 56) } else { Color32::from_rgb(32, 36, 46) });
        crate::icons::paint(ui.painter(), Rect::from_min_size(dr.min + vec2(10.0, 10.0), vec2(24.0, 24.0)), dest.icon, dest.color);
        ui.painter().text(dr.min + vec2(42.0, 12.0), Align2::LEFT_CENTER, dest.name, Tokens::semibold(13.0), Color32::WHITE);
        ui.painter().text(dr.min + vec2(42.0, 28.0), Align2::LEFT_CENTER, "Sign in to publish", Tokens::ui(10.0), Color32::from_rgb(140, 140, 150));
        if clicked {
            let _ = crate::menus::invoke(app, &ctx, "export.setDestination", json!({"dest": dest.name}));
            app.ui.status = format!("Destination: {}", dest.name);
        }
        dy += 52.0;
    }

    // File name input.
    let name_r = Rect::from_min_size(dest_rect.min + vec2(16.0, dy + 16.0), vec2(dest_w - 32.0, 32.0));
    ui.painter().rect_filled(name_r, 4.0, Color32::from_rgb(32, 36, 46));
    ui.painter().rect_stroke(name_r, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);
    ui.painter().text(name_r.min + vec2(8.0, 16.0), Align2::LEFT_CENTER, "Export_001.mp4", Tokens::ui(12.0), Color32::WHITE);

    // Export button (bottom right).
    let btn_r = Rect::from_min_size(rect.max - vec2(130.0, 44.0), vec2(120.0, 36.0));
    let btn_h = ui.interact(btn_r, egui::Id::unique("exp.export"), Sense::click()).hovered();
    let btn_c = ui.interact(btn_r, egui::Id::unique("exp.export_click"), Sense::click()).clicked();
    ui.painter().rect_filled(btn_r, 8.0, if btn_h { Color32::from_rgb(60, 180, 70) } else { Color32::from_rgb(40, 140, 50) });
    ui.painter().text(btn_r.center(), Align2::CENTER_CENTER, tl!("Export"), Tokens::semibold(14.0), Color32::WHITE);
    if btn_c {
        let _ = crate::menus::invoke(app, &ctx, "export.start", json!({}));
        app.ui.status = "Export started...".to_string();
    }

    // Export queue / progress.
    let queue_r = Rect::from_min_size(dest_rect.min + vec2(16.0, dy + 60.0), vec2(dest_w - 32.0, 60.0));
    ui.painter().rect_filled(queue_r, 6.0, Color32::from_rgb(32, 36, 46));
    ui.painter().text(queue_r.min + vec2(10.0, 10.0), Align2::LEFT_CENTER, "Export Queue", Tokens::semibold(11.0), Color32::from_rgb(160, 160, 170));
    let progress_r = Rect::from_min_size(queue_r.min + vec2(10.0, 28.0), vec2(queue_r.width() - 20.0, 8.0));
    ui.painter().rect_filled(progress_r, 4.0, Color32::from_rgb(60, 60, 70));
    ui.painter().rect_filled(Rect::from_min_size(progress_r.min, vec2(progress_r.width() * 0.6, 8.0)), 4.0, Color32::from_rgb(60, 180, 70));
    ui.painter().text(queue_r.min + vec2(10.0, 42.0), Align2::LEFT_CENTER, "Export_001.mp4  •  62%  •  02:14 left", Tokens::ui(10.0), Color32::from_rgb(160, 160, 170));
}
