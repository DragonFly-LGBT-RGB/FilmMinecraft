//! Text presets panel (M6): one-click creation of text clips from built-in presets
//! (Title, Subtitle, Lower third, TikTok style, Karaoke, Neon, Minimal) plus a
//! "Create text" button for a blank text clip. Presets are shown with a live preview
//! ("Аа") rendered in the preset's font. Clicking a preset creates a graphic clip with
//! a text layer at the playhead via `graphics.newText`, then opens inline text editing.

use egui::{Align2, Color32, Rect, Sense, vec2, pos2};
use serde_json::{Value, json};

use crate::FilmcraftApp;
use crate::icons::{self, Icon};
use crate::theme::Tokens;

/// A text preset: name, description, style overrides, and default duration.
#[derive(Clone, Copy)]
struct TextPreset {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    icon: Icon,
    font: &'static str,
    font_style: &'static str,
    size_px: f32,
    fill: [f32; 4],
    stroke: bool,
    stroke_color: [f32; 4],
    stroke_width: f32,
    shadow: bool,
    bg: bool,
    bg_color: [f32; 4],
    bg_opacity: f32,
    default_seconds: f64,
    animation_in: &'static str,
}

/// Built-in text presets (FilmCraft originals; no third-party preset files consulted).
const PRESETS: [TextPreset; 7] = [
    TextPreset {
        id: "title",
        name: "Title",
        description: "Bold centred title, 3 s",
        icon: Icon::Type,
        font: "Inter",
        font_style: "Bold",
        size_px: 96.0,
        fill: [1.0, 1.0, 1.0, 1.0],
        stroke: false,
        stroke_color: [0.0, 0.0, 0.0, 1.0],
        stroke_width: 2.0,
        shadow: true,
        bg: false,
        bg_color: [0.0, 0.0, 0.0, 1.0],
        bg_opacity: 50.0,
        default_seconds: 3.0,
        animation_in: "Fade",
    },
    TextPreset {
        id: "subtitle",
        name: "Subtitle",
        description: "Centre lower-third subtitle, 4 s",
        icon: Icon::Captions,
        font: "Inter",
        font_style: "Regular",
        size_px: 48.0,
        fill: [1.0, 1.0, 1.0, 1.0],
        stroke: false,
        stroke_color: [0.0, 0.0, 0.0, 1.0],
        stroke_width: 2.0,
        shadow: true,
        bg: true,
        bg_color: [0.0, 0.0, 0.0, 1.0],
        bg_opacity: 60.0,
        default_seconds: 4.0,
        animation_in: "Fade",
    },
    TextPreset {
        id: "lower_third",
        name: "Lower Third",
        description: "Name + role lower third, 5 s",
        icon: Icon::Square,
        font: "Inter",
        font_style: "Medium",
        size_px: 40.0,
        fill: [1.0, 1.0, 1.0, 1.0],
        stroke: false,
        stroke_color: [0.0, 0.0, 0.0, 1.0],
        stroke_width: 2.0,
        shadow: false,
        bg: true,
        bg_color: [0.2, 0.4, 0.8, 1.0],
        bg_opacity: 80.0,
        default_seconds: 5.0,
        animation_in: "Slide up",
    },
    TextPreset {
        id: "tiktok",
        name: "TikTok style",
        description: "Bold white with black outline, 3 s",
        icon: Icon::Export,
        font: "Montserrat",
        font_style: "Bold",
        size_px: 72.0,
        fill: [1.0, 1.0, 1.0, 1.0],
        stroke: true,
        stroke_color: [0.0, 0.0, 0.0, 1.0],
        stroke_width: 6.0,
        shadow: false,
        bg: false,
        bg_color: [0.0, 0.0, 0.0, 1.0],
        bg_opacity: 50.0,
        default_seconds: 3.0,
        animation_in: "Pop",
    },
    TextPreset {
        id: "karaoke",
        name: "Karaoke",
        description: "Word-by-word highlight, 4 s",
        icon: Icon::Captions,
        font: "Inter",
        font_style: "Bold",
        size_px: 56.0,
        fill: [1.0, 1.0, 0.0, 1.0],
        stroke: true,
        stroke_color: [0.0, 0.0, 0.0, 1.0],
        stroke_width: 3.0,
        shadow: false,
        bg: false,
        bg_color: [0.0, 0.0, 0.0, 1.0],
        bg_opacity: 50.0,
        default_seconds: 4.0,
        animation_in: "Fade",
    },
    TextPreset {
        id: "neon",
        name: "Neon",
        description: "Glowing coloured text, 4 s",
        icon: Icon::Sparkle,
        font: "Oswald",
        font_style: "Regular",
        size_px: 64.0,
        fill: [0.0, 1.0, 1.0, 1.0],
        stroke: false,
        stroke_color: [0.0, 0.0, 0.0, 1.0],
        stroke_width: 2.0,
        shadow: true,
        bg: false,
        bg_color: [0.0, 0.0, 0.0, 1.0],
        bg_opacity: 50.0,
        default_seconds: 4.0,
        animation_in: "Fade",
    },
    TextPreset {
        id: "minimal",
        name: "Minimal",
        description: "Thin clean text, 5 s",
        icon: Icon::Square,
        font: "Inter",
        font_style: "Light",
        size_px: 48.0,
        fill: [0.9, 0.9, 0.9, 1.0],
        stroke: false,
        stroke_color: [0.0, 0.0, 0.0, 1.0],
        stroke_width: 1.0,
        shadow: false,
        bg: false,
        bg_color: [0.0, 0.0, 0.0, 1.0],
        bg_opacity: 50.0,
        default_seconds: 5.0,
        animation_in: "Fade",
    },
];

/// Frontend state for the text presets panel.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct TextPresetsState {
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
        crate::i18n::t("Text Presets"),
        Tokens::semibold(18.0),
        t.text,
    );
    ui.painter().text(
        rect.min + vec2(16.0, 48.0),
        Align2::LEFT_CENTER,
        crate::i18n::t("Click a preset to add a text clip at the playhead."),
        Tokens::ui(12.0),
        Color32::from_rgb(160, 160, 170),
    );

    // Search.
    let search_r = Rect::from_min_size(rect.min + vec2(16.0, 72.0), vec2(rect.width() - 32.0, 28.0));
    let mut query = app.ui.text_presets.query.clone();
    ui.painter().rect_filled(search_r, 4.0, Color32::from_rgb(24, 28, 36));
    ui.painter().rect_stroke(search_r, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);
    ui.painter().text(search_r.min + vec2(10.0, 14.0), Align2::LEFT_CENTER, &query, Tokens::ui(12.0), Color32::from_rgb(200, 200, 210));
    app.auto.add("text_presets.search", search_r, "Search");

    // Presets grid.
    let mut y = rect.min.y + 112.0;
    let card_w = 160.0;
    let card_h = 110.0;
    let gap = 14.0;
    let cols = ((rect.width() - 32.0 + gap) / (card_w + gap)).floor().max(1.0) as usize;
    let mut cx = 0usize;

    let q = query.trim().to_lowercase();
    for preset in PRESETS.iter() {
        if !q.is_empty()
            && !preset.name.to_lowercase().contains(&q)
            && !preset.description.to_lowercase().contains(&q)
        {
            continue;
        }
        let x = rect.min.x + 16.0 + cx as f32 * (card_w + gap);
        let card_r = Rect::from_min_size(pos2(x, y), vec2(card_w, card_h));
        let hovered = ui.interact(card_r, egui::Id::new(("text_preset", preset.id)), Sense::click()).hovered();
        let clicked = ui.interact(card_r, egui::Id::new(("text_preset_click", preset.id)), Sense::click()).clicked();
        let sel = app.ui.text_presets.selected.as_deref() == Some(preset.id);
        ui.painter().rect_filled(card_r, 8.0, if sel { Color32::from_rgb(40, 44, 56) } else if hovered { Color32::from_rgb(36, 40, 50) } else { Color32::from_rgb(24, 28, 36) });
        ui.painter().rect_stroke(card_r, 8.0, egui::Stroke::new(if sel { 2.0 } else { 1.0 }, if sel { t.accent } else { Color32::from_rgb(40, 44, 56) }), egui::StrokeKind::Inside);

        // Icon.
        let icon_r = Rect::from_min_size(card_r.min + vec2(12.0, 10.0), vec2(36.0, 36.0));
        icons::paint(ui.painter(), icon_r, preset.icon, Color32::WHITE);

        // Live preview "Аа" in the preset's fill colour.
        let prev_r = Rect::from_min_size(card_r.min + vec2(12.0, 52.0), vec2(card_w - 24.0, 24.0));
        ui.painter().text(
            prev_r.center(),
            Align2::CENTER_CENTER,
            "\u{410}\u{430}",
            Tokens::ui(18.0),
            Color32::from_rgb(
                (preset.fill[0] * 255.0) as u8,
                (preset.fill[1] * 255.0) as u8,
                (preset.fill[2] * 255.0) as u8,
            ),
        );

        ui.painter().text(
            card_r.min + vec2(10.0, 84.0),
            Align2::LEFT_CENTER,
            preset.name,
            Tokens::semibold(12.0),
            Color32::WHITE,
        );
        ui.painter().text(
            card_r.min + vec2(10.0, 98.0),
            Align2::LEFT_CENTER,
            preset.description,
            Tokens::ui(9.5),
            Color32::from_rgb(140, 140, 150),
        );

        let label = format!("text_presets.item.{}", preset.id);
        app.auto.add(&label, card_r, preset.name);

        if clicked {
            app.ui.text_presets.selected = Some(preset.id.into());
            create_text_clip(app, preset, &mut actions);
        }

        cx += 1;
        if cx >= cols {
            cx = 0;
            y += card_h + gap;
        }
    }

    // Create blank text button.
    let blank_r = Rect::from_min_size(rect.min + vec2(16.0, rect.height() - 44.0), vec2(200.0, 30.0));
    let blank_h = ui.interact(blank_r, egui::Id::new("text_presets.blank"), Sense::click()).hovered();
    let blank_c = ui.interact(blank_r, egui::Id::new("text_presets.blank_click"), Sense::click()).clicked();
    ui.painter().rect_filled(blank_r, 6.0, if blank_h { Color32::from_rgb(60, 120, 255) } else { Color32::from_rgb(40, 100, 220) });
    ui.painter().text(blank_r.center(), Align2::CENTER_CENTER, crate::i18n::t("Create text clip"), Tokens::semibold(12.0), Color32::WHITE);
    app.auto.add("text_presets.blank", blank_r, "Create text clip");
    if blank_c {
        let preset = TextPreset {
            id: "blank",
            name: "Blank text",
            description: "",
            icon: Icon::Type,
            font: "Inter",
            font_style: "Regular",
            size_px: 48.0,
            fill: [1.0, 1.0, 1.0, 1.0],
            stroke: false,
            stroke_color: [0.0, 0.0, 0.0, 1.0],
            stroke_width: 2.0,
            shadow: false,
            bg: false,
            bg_color: [0.0, 0.0, 0.0, 1.0],
            bg_opacity: 50.0,
            default_seconds: 5.0,
            animation_in: "Fade",
        };
        create_text_clip(app, &preset, &mut actions);
    }

    // Run pending actions.
    for (cmd, params) in actions {
        let _ = crate::menus::invoke(app, ui.ctx(), &cmd, params);
    }
}

/// Create a graphic clip with a text layer from a preset, at the playhead.
fn create_text_clip(app: &mut FilmcraftApp, preset: &TextPreset, actions: &mut Vec<(String, Value)>) {
    let Some(seq) = app.session.state.active_sequence else { return };
    let (w, h) = (
        app.session.project.sequence(seq).map(|q| q.settings.width).unwrap_or(1920),
        app.session.project.sequence(seq).map(|q| q.settings.height).unwrap_or(1080),
    );

    let mut params = json!({
        "text": preset.name,
        "size": preset.size_px,
        "font": preset.font,
        "fontStyle": preset.font_style,
        "seconds": preset.default_seconds,
        "position": [w as f64 / 2.0, h as f64 / 2.0],
    });

    // Style overrides.
    if preset.fill != [1.0, 1.0, 1.0, 1.0] {
        params["fill_color"] = json!(preset.fill);
    }
    if preset.stroke {
        params["stroke"] = json!(true);
        params["stroke_color"] = json!(preset.stroke_color);
        params["stroke_width"] = json!(preset.stroke_width);
    }
    if preset.shadow {
        params["shadow"] = json!(true);
    }
    if preset.bg {
        params["background"] = json!(true);
        params["background_color"] = json!(preset.bg_color);
        params["background_opacity"] = json!(preset.bg_opacity);
    }

    actions.push(("graphics.newText".into(), params));
}
