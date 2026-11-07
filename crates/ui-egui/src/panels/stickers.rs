//! Stickers panel (M6): SVG shapes, arrows, bubbles, emoji — one-click creation of sticker
//! clips via `graphics.newShape`. Stickers are graphic clips with a shape layer; colour and
//! transform are editable in the Properties panel like any graphic layer.

use egui::{Align2, Color32, Rect, Sense, vec2, pos2};
use serde_json::{Value, json};

use crate::FilmcraftApp;
use crate::icons::{self, Icon};
use crate::theme::Tokens;

/// A sticker: name, shape kind, default colour, and icon.
#[derive(Clone, Copy)]
struct Sticker {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    icon: Icon,
    shape: &'static str, // "rectangle|ellipse|polygon|path"
    colour: [f32; 4],
    default_size: [f32; 2],
}

/// Built-in stickers (FilmCraft originals, drawn in code — no third-party artwork).
const STICKERS: [Sticker; 12] = [
    Sticker {
        id: "arrow",
        name: "Arrow",
        description: "Direction arrow",
        icon: Icon::ArrowUp,
        shape: "polygon",
        colour: [1.0, 1.0, 1.0, 1.0],
        default_size: [120.0, 60.0],
    },
    Sticker {
        id: "bubble",
        name: "Speech bubble",
        description: "Dialogue bubble",
        icon: Icon::Chat,
        shape: "ellipse",
        colour: [1.0, 1.0, 1.0, 1.0],
        default_size: [140.0, 90.0],
    },
    Sticker {
        id: "heart",
        name: "Heart",
        description: "Like / love",
        icon: Icon::Star,
        shape: "polygon",
        colour: [1.0, 0.2, 0.2, 1.0],
        default_size: [80.0, 80.0],
    },
    Sticker {
        id: "star",
        name: "Star",
        description: "Rating / highlight",
        icon: Icon::Star,
        shape: "polygon",
        colour: [1.0, 0.9, 0.2, 1.0],
        default_size: [80.0, 80.0],
    },
    Sticker {
        id: "badge",
        name: "Badge",
        description: "Notification badge",
        icon: Icon::Bell,
        shape: "ellipse",
        colour: [1.0, 0.3, 0.3, 1.0],
        default_size: [60.0, 60.0],
    },
    Sticker {
        id: "rectangle",
        name: "Rectangle",
        description: "Coloured box",
        icon: Icon::Square,
        shape: "rectangle",
        colour: [0.5, 0.7, 1.0, 1.0],
        default_size: [200.0, 120.0],
    },
    Sticker {
        id: "circle",
        name: "Circle",
        description: "Round shape",
        icon: Icon::Ellipse,
        shape: "ellipse",
        colour: [0.7, 1.0, 0.7, 1.0],
        default_size: [120.0, 120.0],
    },
    Sticker {
        id: "smile",
        name: "Smile",
        description: "Happy face",
        icon: Icon::Star,
        shape: "ellipse",
        colour: [1.0, 0.95, 0.7, 1.0],
        default_size: [100.0, 100.0],
    },
    Sticker {
        id: "cool",
        name: "Cool",
        description: "Sunglasses face",
        icon: Icon::Star,
        shape: "ellipse",
        colour: [0.2, 0.4, 0.8, 1.0],
        default_size: [100.0, 100.0],
    },
    Sticker {
        id: "check",
        name: "Check mark",
        description: "Approved / done",
        icon: Icon::Close,
        shape: "polygon",
        colour: [0.3, 0.8, 0.3, 1.0],
        default_size: [80.0, 80.0],
    },
    Sticker {
        id: "cross",
        name: "Cross",
        description: "Error / cancel",
        icon: Icon::Close,
        shape: "rectangle",
        colour: [1.0, 0.3, 0.3, 1.0],
        default_size: [80.0, 80.0],
    },
    Sticker {
        id: "lightning",
        name: "Lightning",
        description: "Energy / impact",
        icon: Icon::Sparkle,
        shape: "polygon",
        colour: [1.0, 1.0, 0.3, 1.0],
        default_size: [80.0, 120.0],
    },
];

/// Frontend state.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct StickersState {
    pub query: String,
    pub selected: Option<String>,
}

pub fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    ui.painter().rect_filled(rect, 0.0, t.panel_bg);

    let mut actions: Vec<(String, Value)> = Vec::new();

    // Header.
    ui.painter().text(
        rect.min + vec2(16.0, 24.0),
        Align2::LEFT_CENTER,
        crate::i18n::t("Stickers"),
        Tokens::semibold(18.0),
        t.text,
    );
    ui.painter().text(
        rect.min + vec2(16.0, 48.0),
        Align2::LEFT_CENTER,
        crate::i18n::t("Click a sticker to add it at the playhead."),
        Tokens::ui(12.0),
        Color32::from_rgb(160, 160, 170),
    );

    // Search.
    let search_r = Rect::from_min_size(rect.min + vec2(16.0, 72.0), vec2(rect.width() - 32.0, 28.0));
    ui.painter().rect_filled(search_r, 4.0, Color32::from_rgb(24, 28, 36));
    ui.painter().rect_stroke(search_r, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);
    ui.painter().text(search_r.min + vec2(10.0, 14.0), Align2::LEFT_CENTER, &app.ui.stickers.query, Tokens::ui(12.0), Color32::from_rgb(200, 200, 210));
    app.auto.add("stickers.search", search_r, "Search");

    // Stickers grid.
    let mut y = rect.min.y + 112.0;
    let card_w = 120.0;
    let card_h = 100.0;
    let gap = 12.0;
    let cols = ((rect.width() - 32.0 + gap) / (card_w + gap)).floor().max(1.0) as usize;
    let mut cx = 0usize;

    let q = app.ui.stickers.query.trim().to_lowercase();
    for sticker in STICKERS.iter() {
        if !q.is_empty()
            && !sticker.name.to_lowercase().contains(&q)
            && !sticker.description.to_lowercase().contains(&q)
        {
            continue;
        }
        let x = rect.min.x + 16.0 + cx as f32 * (card_w + gap);
        let card_r = Rect::from_min_size(pos2(x, y), vec2(card_w, card_h));
        let hovered = ui.interact(card_r, egui::Id::new(("sticker", sticker.id)), Sense::click()).hovered();
        let clicked = ui.interact(card_r, egui::Id::new(("sticker_click", sticker.id)), Sense::click()).clicked();
        let sel = app.ui.stickers.selected.as_deref() == Some(sticker.id);
        ui.painter().rect_filled(card_r, 8.0, if sel { Color32::from_rgb(40, 44, 56) } else if hovered { Color32::from_rgb(36, 40, 50) } else { Color32::from_rgb(24, 28, 36) });
        ui.painter().rect_stroke(card_r, 8.0, egui::Stroke::new(if sel { 2.0 } else { 1.0 }, if sel { t.accent } else { Color32::from_rgb(40, 44, 56) }), egui::StrokeKind::Inside);

        // Icon.
        let icon_r = Rect::from_min_size(card_r.min + vec2(6.0, 8.0), vec2(28.0, 28.0));
        icons::paint(ui.painter(), icon_r, sticker.icon, Color32::WHITE);

        // Coloured shape preview.
        let prev_r = Rect::from_min_size(card_r.min + vec2(6.0, 42.0), vec2(card_w - 12.0, 32.0));
        ui.painter().rect_filled(prev_r, 4.0, Color32::from_rgb(
            (sticker.colour[0] * 255.0) as u8,
            (sticker.colour[1] * 255.0) as u8,
            (sticker.colour[2] * 255.0) as u8,
        ));

        ui.painter().text(
            card_r.min + vec2(6.0, 80.0),
            Align2::LEFT_CENTER,
            sticker.name,
            Tokens::semibold(11.0),
            Color32::WHITE,
        );
        ui.painter().text(
            card_r.min + vec2(6.0, 92.0),
            Align2::LEFT_CENTER,
            sticker.description,
            Tokens::ui(8.5),
            Color32::from_rgb(140, 140, 150),
        );

        let label = format!("stickers.item.{}", sticker.id);
        app.auto.add(&label, card_r, sticker.name);

        if clicked {
            app.ui.stickers.selected = Some(sticker.id.into());
            create_sticker_clip(app, sticker, &mut actions);
        }

        cx += 1;
        if cx >= cols {
            cx = 0;
            y += card_h + gap;
        }
    }

    // Run pending actions.
    for (cmd, params) in actions {
        let _ = crate::menus::invoke(app, ui.ctx(), &cmd, params);
    }
}

/// Create a graphic clip with a shape layer from a sticker, at the playhead.
fn create_sticker_clip(app: &mut FilmcraftApp, sticker: &Sticker, actions: &mut Vec<(String, Value)>) {
    let Some(seq) = app.session.state.active_sequence else { return };
    let (w, h) = (
        app.session.project.sequence(seq).map(|q| q.settings.width).unwrap_or(1920),
        app.session.project.sequence(seq).map(|q| q.settings.height).unwrap_or(1080),
    );

    let points = match sticker.shape {
        "polygon" => {
            // Simple 10-point star shape as normalized points.
            Some(json!([
                [0.5, 0.1], [0.6, 0.4], [0.9, 0.4], [0.65, 0.6], [0.75, 0.9],
                [0.5, 0.7], [0.25, 0.9], [0.35, 0.6], [0.1, 0.4], [0.4, 0.4],
            ]))
        }
        _ => None,
    };

    let mut params = json!({
        "shape": sticker.shape,
        "position": [w as f64 / 2.0, h as f64 / 2.0],
        "size": [sticker.default_size[0] as f64, sticker.default_size[1] as f64],
        "seconds": 5.0,
    });

    if let Some(pts) = points {
        params["points"] = pts;
    }

    actions.push(("graphics.newShape".into(), params));
}
