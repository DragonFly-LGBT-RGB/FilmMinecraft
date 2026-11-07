//! Project manager panel: shows imported media clips, bins, and allows creating new clips
//! from the media pool. Integrates with the home screen's "Recent drafts" and quick actions.

use egui::{Align2, Color32, Rect, Sense, vec2, pos2};
use serde_json::json;

use crate::FilmcraftApp;
use crate::icons::Icon;
use crate::theme::Tokens;

/// A media item in the project bin.
#[derive(Clone)]
struct MediaItem {
    id: u64,
    name: &'static str,
    kind: MediaKind,
    duration: &'static str,
    resolution: &'static str,
    icon: Icon,
    color: Color32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MediaKind {
    Video,
    Audio,
    Image,
}

const MEDIA_POOL: [MediaItem; 12] = [
    MediaItem { id: 0, name: "Panda_4K.mp4", kind: MediaKind::Video, duration: "00:26", resolution: "3840×2160", icon: Icon::Film, color: Color32::from_rgb(60, 120, 200) },
    MediaItem { id: 1, name: "Skateboard.mov", kind: MediaKind::Video, duration: "00:05", resolution: "1920×1080", icon: Icon::Film, color: Color32::from_rgb(200, 80, 60) },
    MediaItem { id: 2, name: "Coffee.jpg", kind: MediaKind::Image, duration: "—", resolution: "4032×3024", icon: Icon::Image, color: Color32::from_rgb(220, 90, 30) },
    MediaItem { id: 3, name: "Street.mp4", kind: MediaKind::Video, duration: "00:05", resolution: "1920×1080", icon: Icon::Film, color: Color32::from_rgb(90, 160, 90) },
    MediaItem { id: 4, name: "Interview.mov", kind: MediaKind::Video, duration: "00:17", resolution: "3840×2160", icon: Icon::Film, color: Color32::from_rgb(140, 90, 200) },
    MediaItem { id: 5, name: "Timelapse.mp4", kind: MediaKind::Video, duration: "00:42", resolution: "3840×2160", icon: Icon::Film, color: Color32::from_rgb(220, 140, 40) },
    MediaItem { id: 6, name: "Voiceover.wav", kind: MediaKind::Audio, duration: "00:45", resolution: "—", icon: Icon::Audio, color: Color32::from_rgb(120, 160, 220) },
    MediaItem { id: 7, name: "Background.mp3", kind: MediaKind::Audio, duration: "02:10", resolution: "—", icon: Icon::Audio, color: Color32::from_rgb(120, 160, 220) },
    MediaItem { id: 8, name: "Logo.png", kind: MediaKind::Image, duration: "—", resolution: "1024×1024", icon: Icon::Image, color: Color32::from_rgb(160, 90, 220) },
    MediaItem { id: 9, name: "B-roll_City.mp4", kind: MediaKind::Video, duration: "00:12", resolution: "1920×1080", icon: Icon::Film, color: Color32::from_rgb(60, 120, 200) },
    MediaItem { id: 10, name: "SFX_Click.wav", kind: MediaKind::Audio, duration: "00:01", resolution: "—", icon: Icon::Audio, color: Color32::from_rgb(120, 160, 220) },
    MediaItem { id: 11, name: "Overlay_Smoke.mov", kind: MediaKind::Video, duration: "00:08", resolution: "1920×1080", icon: Icon::Film, color: Color32::from_rgb(180, 180, 180) },
];

/// Bins (folders) in the project.
#[derive(Clone)]
struct Bin {
    name: &'static str,
    item_ids: &'static [u64],
    icon: Icon,
}

const BINS: [Bin; 4] = [
    Bin { name: "All Media", item_ids: &[0,1,2,3,4,5,6,7,8,9,10,11], icon: Icon::Folder },
    Bin { name: "Video", item_ids: &[0,1,3,4,5,9,11], icon: Icon::Film },
    Bin { name: "Audio", item_ids: &[6,7,10], icon: Icon::Audio },
    Bin { name: "Images", item_ids: &[2,8], icon: Icon::Image },
];

pub fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    let ctx = ui.ctx().clone();

    // Header.
    ui.painter().rect_filled(rect, t.radius, t.panel_bg);
    ui.painter().text(
        rect.min + vec2(16.0, 18.0),
        Align2::LEFT_CENTER,
        tl!("Project"),
        Tokens::semibold(16.0),
        t.text,
    );

    // Import button.
    let import_r = Rect::from_min_size(rect.max - vec2(100.0, 30.0), vec2(90.0, 28.0));
    let import_h = ui.interact(import_r, egui::Id::unique("proj.import"), Sense::click()).hovered();
    let import_c = ui.interact(import_r, egui::Id::unique("proj.import_click"), Sense::click()).clicked();
    ui.painter().rect_filled(import_r, 6.0, if import_h { Color32::from_rgb(60, 120, 255) } else { Color32::from_rgb(40, 100, 220) });
    ui.painter().text(import_r.center(), Align2::CENTER_CENTER, tl!("Import"), Tokens::ui(12.0), Color32::WHITE);
    if import_c {
        let _ = crate::menus::invoke(app, &ctx, "file.import", json!({}));
    }

    // Bin sidebar (left).
    let bin_w = 150.0;
    let bin_rect = Rect::from_min_size(rect.min + vec2(16.0, 56.0), vec2(bin_w, rect.height() - 72.0));
    ui.painter().rect_filled(bin_rect, 8.0, Color32::from_rgb(24, 28, 36));

    let mut by = bin_rect.min.y + 16.0;
    for (i, bin) in BINS.iter().enumerate() {
        let br = Rect::from_min_size(bin_rect.min + vec2(12.0, by), vec2(bin_w - 24.0, 34.0));
        let hovered = ui.interact(br, egui::Id::unique(("bin", i)), Sense::click()).hovered();
        ui.painter().rect_filled(br, 6.0, if hovered { Color32::from_rgb(40, 44, 56) } else { Color32::TRANSPARENT });
        crate::icons::paint(ui.painter(), Rect::from_min_size(br.min + vec2(8.0, 4.0), vec2(24.0, 24.0)), bin.icon, Color32::from_rgb(150, 150, 160));
        ui.painter().text(br.min + vec2(40.0, 17.0), Align2::LEFT_CENTER, bin.name, Tokens::ui(13.0), Color32::from_rgb(200, 200, 210));
        by += 40.0;
    }

    // Media grid (right).
    let grid_x = rect.min.x + 16.0 + bin_w + 20.0;
    let grid_w = rect.max.x - grid_x - 16.0;
    let grid_rect = Rect::from_min_size(pos2(grid_x, rect.min.y + 56.0), vec2(grid_w, rect.height() - 72.0));
    ui.painter().rect_filled(grid_rect, 8.0, Color32::from_rgb(24, 28, 36));

    // Search bar.
    let search_r = Rect::from_min_size(grid_rect.min + vec2(12.0, 12.0), vec2(grid_w - 24.0, 32.0));
    ui.painter().rect_filled(search_r, 6.0, Color32::from_rgb(32, 36, 46));
    ui.painter().rect_stroke(search_r, 6.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);
    ui.painter().text(search_r.min + vec2(10.0, 16.0), Align2::LEFT_CENTER, tl!("Search media…"), Tokens::ui(12.0), Color32::from_rgb(120, 120, 130));

    // Grid of media items.
    let item_w = 140.0;
    let item_h = 110.0;
    let gap = 14.0;
    let cols = (grid_w / (item_w + gap)).floor() as usize;
    let start_y = grid_rect.min.y + 56.0;

    for (i, item) in MEDIA_POOL.iter().enumerate() {
        let row = i / cols;
        let col = i % cols;
        let x = grid_rect.min.x + 12.0 + col as f32 * (item_w + gap);
        let y = start_y + row as f32 * (item_h + gap);
        let r = Rect::from_min_size(pos2(x, y), vec2(item_w, item_h));
        let hovered = ui.interact(r, egui::Id::unique(("media", item.id)), Sense::click()).hovered();
        let clicked = ui.interact(r, egui::Id::unique(("media_click", item.id)), Sense::click()).clicked();

        ui.painter().rect_filled(r, 8.0, if hovered { Color32::from_rgb(40, 44, 56) } else { Color32::from_rgb(32, 36, 46) });
        if hovered {
            ui.painter().rect_stroke(r, 8.0, egui::Stroke::new(1.0, Color32::from_rgb(0, 180, 216)), egui::StrokeKind::Inside);
        }

        // Thumbnail.
        let thumb = Rect::from_min_size(r.min + vec2(10.0, 10.0), vec2(item_w - 20.0, 64.0));
        ui.painter().rect_filled(thumb, 6.0, item.color);
        crate::icons::paint(ui.painter(), Rect::from_min_size(thumb.center() - vec2(16.0, 16.0), vec2(32.0, 32.0)), item.icon, Color32::WHITE);

        // Duration badge.
        let dur = Rect::from_min_size(thumb.max - vec2(40.0, 16.0), vec2(36.0, 16.0));
        ui.painter().rect_filled(dur, 8.0, Color32::BLACK.linear_multiply(0.6));
        ui.painter().text(dur.center(), Align2::CENTER_CENTER, item.duration, Tokens::ui(9.0), Color32::WHITE);

        // Name.
        ui.painter().text(r.min + vec2(8.0, 86.0), Align2::LEFT_CENTER, item.name, Tokens::semibold(11.0), Color32::WHITE);
        ui.painter().text(r.min + vec2(8.0, 100.0), Align2::LEFT_CENTER, item.resolution, Tokens::ui(9.0), Color32::from_rgb(140, 140, 150));

        if clicked {
            // Drag to timeline or add to sequence.
            let _ = crate::menus::invoke(app, &ctx, "file.importMedia", json!({"id": item.id}));
            app.ui.status = format!("Added {} to timeline", item.name);
        }
        app.auto.add(&format!("proj.media.{}", item.id), r, item.name);
    }

    // Empty state hint.
    if MEDIA_POOL.is_empty() {
        ui.painter().text(grid_rect.center(), Align2::CENTER_CENTER, tl!("No media imported yet"), Tokens::ui(14.0), Color32::from_rgb(120, 120, 130));
    }
}
