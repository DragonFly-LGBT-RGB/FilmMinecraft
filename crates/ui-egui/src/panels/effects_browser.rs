//! Effects browser panel: video/audio effects, transitions, filters, presets.
//! Built from the [`EffectDef`] registry (`crates/project/src/effect.rs`) — every effect
//! is a manifest entry; the UI is generated automatically from parameter schemas.
//! Draggable onto timeline clips.

use std::collections::BTreeSet;

use egui::{Align2, Color32, Rect, Sense, vec2, pos2};
use filmcraft_project::{EffectDef, EffectKind, effect_defs, find_effect};
use serde_json::json;

use crate::FilmcraftApp;
use crate::icons::Icon;
use crate::theme::Tokens;

/// A remembered favorite effect id, persisted in memory for this session.
static FAVORITES: std::sync::OnceLock<std::sync::Mutex<BTreeSet<String>>> =
    std::sync::OnceLock::new();

fn favorites() -> &'static std::sync::Mutex<BTreeSet<String>> {
    FAVORITES.get_or_init(|| std::sync::Mutex::new(BTreeSet::new()))
}

pub fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let ctx = ui.ctx().clone();
    let t = app.tokens;

    ui.painter().rect_filled(rect, t.radius, t.panel_bg);
    ui.painter().text(
        rect.min + vec2(16.0, 18.0),
        Align2::LEFT_CENTER,
        tl!("Effects"),
        Tokens::semibold(16.0),
        t.text,
    );

    // --- Category tabs (built each frame so translated labels work) ---
    let cat_entries: Vec<(&str, Option<EffectKind>, Option<&str>)> = vec![
        (tl!("All"), None, None),
        (tl!("Video Effects"), Some(EffectKind::Video), None),
        (tl!("Audio Effects"), Some(EffectKind::Audio), None),
        (tl!("Transitions"), None, None),
        (tl!("Color"), None, None),
    ];
    let tab_w = 95.0;
    let tab_h = 30.0;
    let tab_y = rect.min.y + 52.0;
    let mut selected_cat = app
        .ui
        .effects_cat
        .clone()
        .unwrap_or_else(|| tl!("All").to_string());

    let mut cx = rect.min.x + 16.0;
    for (i, (label, kind, _sub)) in cat_entries.iter().enumerate() {
        let tr = Rect::from_min_size(pos2(cx, tab_y), vec2(tab_w, tab_h));
        let hovered = ui.interact(tr, egui::Id::new(("fx_cat", i)), Sense::click()).hovered();
        let clicked = ui.interact(tr, egui::Id::new(("fx_cat_click", i)), Sense::click()).clicked();
        let active = selected_cat == *label;
        ui.painter().rect_filled(
            tr,
            6.0,
            if active {
                Color32::from_rgb(40, 100, 220)
            } else if hovered {
                Color32::from_rgb(40, 44, 56)
            } else {
                Color32::TRANSPARENT
            },
        );
        ui.painter().text(
            tr.center(),
            Align2::CENTER_CENTER,
            *label,
            Tokens::ui(12.0),
            if active {
                Color32::WHITE
            } else {
                Color32::from_rgb(180, 180, 190)
            },
        );
        if clicked {
            selected_cat = label.to_string();
            app.ui.effects_cat = Some(selected_cat.clone());
        }
        cx += tab_w + 8.0;
    }

    // --- Search ---
    let search_r = Rect::from_min_size(rect.max - vec2(150.0, 32.0) + vec2(0.0, 52.0), vec2(140.0, 28.0));
    ui.painter().rect_filled(search_r, 6.0, Color32::from_rgb(24, 28, 36));
    ui.painter().rect_stroke(search_r, 6.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);
    ui.painter().text(search_r.min + vec2(8.0, 16.0), Align2::LEFT_CENTER, "🔍", Tokens::ui(12.0), Color32::from_rgb(120, 120, 130));

    // Capture search text input.
    let search_events: Vec<egui::Event> = ctx.input(|i| i.events.iter().cloned().collect());
    for ev in search_events.iter() {
        if let egui::Event::Text(c) = ev {
            app.ui.effects_search.push_str(c);
        }
        if let egui::Event::Key { key: egui::Key::Backspace, pressed: true, .. } = ev {
            app.ui.effects_search.pop();
        }
    }
    ui.painter().text(
        search_r.min + vec2(30.0, 16.0),
        Align2::LEFT_CENTER,
        if app.ui.effects_search.is_empty() { tl!("Search…") } else { &app.ui.effects_search },
        Tokens::ui(12.0),
        if app.ui.effects_search.is_empty() { Color32::from_rgb(120, 120, 130) } else { Color32::WHITE },
    );

    // --- Build filtered effect list from registry ---
    let term = app.ui.effects_search.to_lowercase();
    let all_defs: Vec<&EffectDef> = effect_defs()
        .iter()
        .filter(|e| {
            // Category filter.
            let cat_ok = match selected_cat.as_str() {
                s if s == tl!("All") => true,
                s if s == tl!("Video Effects") => matches!(e.kind, EffectKind::Video),
                s if s == tl!("Audio Effects") => matches!(e.kind, EffectKind::Audio),
                s if s == tl!("Transitions") => {
                    matches!(e.kind, EffectKind::VideoTransition | EffectKind::AudioTransition)
                }
                s if s == tl!("Color") => e.category.iter().any(|c| c.eq_ignore_ascii_case("Color Correction")),
                _ => true,
            };
            if !cat_ok {
                return false;
            }
            // Search term.
            if term.is_empty() {
                true
            } else {
                e.id.to_lowercase().contains(&term)
                    || e.name.to_lowercase().contains(&term)
                    || e.category.iter().any(|c| c.to_lowercase().contains(&term))
            }
        })
        .collect();

    // --- Effects grid ---
    let grid_rect = Rect::from_min_size(rect.min + vec2(16.0, 92.0), vec2(rect.width() - 32.0, rect.height() - 108.0));
    ui.painter().rect_filled(grid_rect, 8.0, Color32::from_rgb(24, 28, 36));

    if all_defs.is_empty() {
        ui.painter().text(
            grid_rect.center(),
            Align2::CENTER_CENTER,
            tl!("No effects found"),
            Tokens::ui(14.0),
            Color32::from_rgb(120, 120, 130),
        );
        return;
    }

    let item_w = 150.0;
    let item_h = 95.0;
    let gap = 14.0;
    let cols = ((grid_rect.width() + gap) / (item_w + gap)).floor().max(1.0) as usize;
    let start_y = grid_rect.min.y + 16.0;

    let has_selection = app.session.active_sequence().is_some()
        && !app.session.state.selection.is_empty();

    for (idx, def) in all_defs.iter().enumerate() {
        let row = idx / cols;
        let col = idx % cols;
        let x = grid_rect.min.x + 12.0 + col as f32 * (item_w + gap);
        let y = start_y + row as f32 * (item_h + gap);
        let r = Rect::from_min_size(pos2(x, y), vec2(item_w, item_h));
        let card_id = egui::Id::new(("fx_card", def.id));
        let hovered = ui.interact(r, card_id, Sense::click()).hovered();
        let clicked = ui.interact(r, card_id, Sense::click()).clicked();
        let is_fav = favorites().lock().unwrap().contains(def.id);

        // Card background.
        ui.painter().rect_filled(r, 8.0, if hovered { Color32::from_rgb(40, 44, 56) } else { Color32::from_rgb(32, 36, 46) });
        if hovered {
            ui.painter().rect_stroke(r, 8.0, egui::Stroke::new(1.0, Color32::from_rgb(0, 180, 216)), egui::StrokeKind::Inside);
        }

        // Favorite star (top-right).
        let star_r = Rect::from_min_size(r.max - vec2(24.0, 10.0), vec2(18.0, 18.0));
        let star_hovered = ui.interact(star_r, egui::Id::new(("fx_star", def.id)), Sense::click()).hovered();
        let star_clicked = ui.interact(star_r, egui::Id::new(("fx_star_click", def.id)), Sense::click()).clicked();
        ui.painter().text(
            star_r.center(),
            Align2::CENTER_CENTER,
            if is_fav { "★" } else { "☆" },
            Tokens::semibold(if star_hovered { 16.0 } else { 14.0 }),
            if is_fav { Color32::from_rgb(255, 200, 60) } else { Color32::from_rgb(100, 100, 110) },
        );
        if star_clicked {
            let mut fav = favorites().lock().unwrap();
            if fav.contains(def.id) {
                fav.remove(def.id);
            } else {
                fav.insert(def.id.to_string());
            }
        }

        // Category color indicator (left edge).
        let cat_color = category_color(def);
        let indicator = Rect::from_min_size(r.min + vec2(10.0, 14.0), vec2(4.0, 12.0));
        ui.painter().rect_filled(indicator, 2.0, cat_color);

        // Icon (category-based).
        let icon = category_icon(def);
        crate::icons::paint(
            ui.painter(),
            Rect::from_min_size(r.min + vec2(16.0, 32.0), vec2(32.0, 32.0)),
            icon,
            Color32::from_rgb(180, 180, 190),
        );

        // Name.
        ui.painter().text(
            r.min + vec2(56.0, 44.0),
            Align2::LEFT_CENTER,
            def.name,
            Tokens::semibold(12.0),
            Color32::WHITE,
        );

        // Category / kind label.
        let sub = def.category.join(" › ");
        ui.painter().text(
            r.min + vec2(56.0, 62.0),
            Align2::LEFT_CENTER,
            sub,
            Tokens::ui(9.0),
            Color32::from_rgb(120, 120, 130),
        );

        // Preview placeholder (animated loop badge).
        let preview_r = Rect::from_min_size(r.min + vec2(56.0, 78.0), vec2(item_w - 72.0, 14.0));
        ui.painter().rect_filled(preview_r, 4.0, Color32::from_rgb(45, 50, 60));
        ui.painter().rect_stroke(preview_r, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);
        // Simple pulsing preview bar (static for now; animated loop render comes with M4 media engine).
        let fill_w = preview_r.width() * 0.6;
        ui.painter().rect_filled(
            Rect::from_min_size(preview_r.min, vec2(fill_w, 14.0)),
            4.0,
            Color32::from_rgb(0, 180, 216),
        );
        ui.painter().text(
            preview_r.center(),
            Align2::CENTER_CENTER,
            tl!("preview"),
            Tokens::ui(8.0),
            Color32::from_rgb(100, 100, 100),
        );

        // Apply on click.
        if clicked {
            if has_selection {
                let clips: Vec<u64> = app
                    .session
                    .state
                    .selection
                    .iter()
                    .filter_map(|c| app.session.active_sequence().and_then(|seq| seq.find_item(*c).map(|(tid, _)| tid.0)))
                    .collect();
                if !clips.is_empty() {
                    let _ = crate::menus::invoke(
                        app,
                        &ctx,
                        "effects.apply",
                        json!({"clips": clips, "effect": def.id}),
                    );
                    app.ui.status = format!("{} {}", tl!("Applied"), def.name);
                }
            } else {
                app.ui.status = tl!("Select a clip on the timeline first").to_string();
            }
        }

        app.auto.add(&format!("fx.{}", def.id), r, def.name);
    }
}

/// Color badge per effect category folder.
fn category_color(def: &EffectDef) -> Color32 {
    match def.category.first().copied().unwrap_or("") {
        "Color Correction" => Color32::from_rgb(255, 80, 80),
        "Blur & Sharpen" => Color32::from_rgb(80, 180, 255),
        "Noise & Grain" => Color32::from_rgb(160, 100, 255),
        "Keying" => Color32::from_rgb(80, 220, 160),
        "Distort" => Color32::from_rgb(255, 140, 60),
        "Stylize" => Color32::from_rgb(255, 100, 150),
        "Transform" => Color32::from_rgb(100, 200, 255),
        "Generate" => Color32::from_rgb(200, 160, 80),
        "Perspective" => Color32::from_rgb(150, 120, 255),
        "Image Control" => Color32::from_rgb(120, 200, 120),
        "Adjust" => Color32::from_rgb(220, 120, 220),
        "Video" => Color32::from_rgb(100, 180, 255),
        "Amplitude and Compression" | "Filter and EQ" | "Delay and Echo" | "Modulation"
        | "Reverb" | "Noise Reduction/Restoration" | "Special" | "Stereo Imagery" | "Time and Pitch"
        | "Audio Effects" => Color32::from_rgb(120, 220, 120),
        _ => Color32::from_rgb(160, 160, 170),
    }
}

/// Icon per effect kind.
fn category_icon(def: &EffectDef) -> Icon {
    match def.kind {
        EffectKind::Video => Icon::Fx,
        EffectKind::Audio => Icon::Audio,
        EffectKind::VideoTransition | EffectKind::AudioTransition => Icon::Sequence,
    }
}
