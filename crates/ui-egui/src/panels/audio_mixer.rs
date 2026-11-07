//! Audio mixer panel: track faders, pan, solo/mute, EQ, and level meters.

use egui::{Align2, Color32, Rect, Sense, vec2, pos2};
use serde_json::json;

use crate::FilmcraftApp;

use crate::theme::Tokens;

/// An audio track strip.
#[derive(Clone)]
struct AudioTrack {
    name: &'static str,
    level: f32,      // -60 to +6 dB
    pan: f32,        // -1 to 1
    muted: bool,
    soloed: bool,
    record_armed: bool,
    color: Color32,
}

const TRACKS: [AudioTrack; 8] = [
    AudioTrack { name: "Dialogue 1", level: -12.0, pan: 0.0, muted: false, soloed: false, record_armed: false, color: Color32::from_rgb(100, 180, 255) },
    AudioTrack { name: "Dialogue 2", level: -18.0, pan: 0.0, muted: false, soloed: false, record_armed: false, color: Color32::from_rgb(100, 180, 255) },
    AudioTrack { name: "Music", level: -24.0, pan: -0.3, muted: false, soloed: false, record_armed: false, color: Color32::from_rgb(255, 100, 150) },
    AudioTrack { name: "SFX", level: -30.0, pan: 0.5, muted: false, soloed: false, record_armed: false, color: Color32::from_rgb(150, 255, 150) },
    AudioTrack { name: "Ambience", level: -36.0, pan: 0.0, muted: false, soloed: false, record_armed: false, color: Color32::from_rgb(200, 150, 255) },
    AudioTrack { name: "Voiceover", level: -15.0, pan: 0.0, muted: false, soloed: false, record_armed: true, color: Color32::from_rgb(255, 200, 100) },
    AudioTrack { name: "Master", level: -6.0, pan: 0.0, muted: false, soloed: false, record_armed: false, color: Color32::from_rgb(255, 255, 255) },
    AudioTrack { name: "Monitor", level: 0.0, pan: 0.0, muted: false, soloed: false, record_armed: false, color: Color32::from_rgb(120, 220, 120) },
];

pub fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    let ctx = ui.ctx().clone();

    ui.painter().rect_filled(rect, t.radius, t.panel_bg);
    ui.painter().text(
        rect.min + vec2(16.0, 18.0),
        Align2::LEFT_CENTER,
        tl!("Audio Mixer"),
        Tokens::semibold(16.0),
        t.text,
    );

    // Master controls top-right.
    let master_r = Rect::from_min_size(rect.max - vec2(120.0, 28.0), vec2(110.0, 24.0));
    let master_h = ui.interact(master_r, egui::Id::unique("mix.master"), Sense::click()).hovered();
    ui.painter().rect_filled(master_r, 4.0, if master_h { Color32::from_rgb(40, 44, 56) } else { Color32::from_rgb(32, 36, 46) });
    ui.painter().rect_stroke(master_r, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);
    ui.painter().text(master_r.center(), Align2::CENTER_CENTER, "Master Out", Tokens::ui(11.0), Color32::WHITE);

    // Track strips.
    let strip_w = 90.0;
    let gap = 14.0;
    let start_x = rect.min.x + 16.0;
    let start_y = rect.min.y + 56.0;
    let strip_h = rect.height() - 72.0;

    for (i, track) in TRACKS.iter().enumerate() {
        let x = start_x + i as f32 * (strip_w + gap);
        let r = Rect::from_min_size(pos2(x, start_y), vec2(strip_w, strip_h));
        let hovered = ui.interact(r, egui::Id::unique(("mix", i)), Sense::click()).hovered();
        let clicked = ui.interact(r, egui::Id::unique(("mix_click", i)), Sense::click()).clicked();

        // Strip background.
        ui.painter().rect_filled(r, 6.0, if hovered { Color32::from_rgb(40, 44, 56) } else { Color32::from_rgb(32, 36, 46) });
        ui.painter().rect_stroke(r, 6.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);

        // Track name (rotated or small).
        ui.painter().text(r.min + vec2(8.0, 8.0), Align2::LEFT_CENTER, track.name, Tokens::semibold(10.0), Color32::WHITE);

        // Mute button.
        let mute_r = Rect::from_min_size(r.min + vec2(8.0, 24.0), vec2(22.0, 20.0));
        let mute_h = ui.interact(mute_r, egui::Id::unique(("mix_mute", i)), Sense::click()).hovered();
        let mute_c = ui.interact(mute_r, egui::Id::unique(("mix_mute_click", i)), Sense::click()).clicked();
        ui.painter().rect_filled(mute_r, 4.0, if track.muted { Color32::from_rgb(255, 80, 80) } else { if mute_h { Color32::from_rgb(60, 60, 70) } else { Color32::from_rgb(40, 44, 56) } });
        ui.painter().text(mute_r.center(), Align2::CENTER_CENTER, "M", Tokens::ui(10.0), Color32::WHITE);
        if mute_c {
            let _ = crate::menus::invoke(app, &ctx, "audio.toggleMute", json!({"track": i}));
        }

        // Solo button.
        let solo_r = Rect::from_min_size(r.min + vec2(32.0, 24.0), vec2(22.0, 20.0));
        let solo_h = ui.interact(solo_r, egui::Id::unique(("mix_solo", i)), Sense::click()).hovered();
        let solo_c = ui.interact(solo_r, egui::Id::unique(("mix_solo_click", i)), Sense::click()).clicked();
        ui.painter().rect_filled(solo_r, 4.0, if track.soloed { Color32::from_rgb(255, 200, 0) } else { if solo_h { Color32::from_rgb(60, 60, 70) } else { Color32::from_rgb(40, 44, 56) } });
        ui.painter().text(solo_r.center(), Align2::CENTER_CENTER, "S", Tokens::ui(10.0), Color32::WHITE);
        if solo_c {
            let _ = crate::menus::invoke(app, &ctx, "audio.toggleSolo", json!({"track": i}));
        }

        // Record arm.
        let rec_r = Rect::from_min_size(r.min + vec2(56.0, 24.0), vec2(22.0, 20.0));
        let rec_h = ui.interact(rec_r, egui::Id::unique(("mix_rec", i)), Sense::click()).hovered();
        let rec_c = ui.interact(rec_r, egui::Id::unique(("mix_rec_click", i)), Sense::click()).clicked();
        ui.painter().rect_filled(rec_r, 4.0, if track.record_armed { Color32::from_rgb(255, 0, 0) } else { if rec_h { Color32::from_rgb(60, 60, 70) } else { Color32::from_rgb(40, 44, 56) } });
        ui.painter().circle_filled(rec_r.center(), 4.0, Color32::WHITE);
        if rec_c {
            let _ = crate::menus::invoke(app, &ctx, "audio.toggleRecord", json!({"track": i}));
        }

        // Level meter.
        let meter_r = Rect::from_min_size(r.min + vec2(8.0, 54.0), vec2(strip_w - 16.0, 100.0));
        ui.painter().rect_filled(meter_r, 4.0, Color32::from_rgb(24, 28, 36));
        ui.painter().rect_stroke(meter_r, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);
        // Fill level (simulated).
        let level_pct = ((track.level + 60.0) / 66.0).clamp(0.0, 1.0);
        let fill_h = meter_r.height() * level_pct;
        let meter_color = if level_pct > 0.9 { Color32::from_rgb(255, 80, 80) } else if level_pct > 0.7 { Color32::from_rgb(255, 200, 0) } else { track.color };
        ui.painter().rect_filled(Rect::from_min_size(meter_r.min, vec2(meter_r.width(), fill_h)), 4.0, meter_color);
        // DB markers.
        for db in [-60, -30, -12, -6, 0].iter() {
            let by = meter_r.max.y - ((db + 60) as f32 / 60.0) * meter_r.height();
            ui.painter().text(meter_r.max - vec2(meter_r.width() - 4.0, by - 6.0), Align2::RIGHT_CENTER, format!("{}", db).as_str(), Tokens::ui(8.0), Color32::from_rgb(100, 100, 110));
        }

        // Fader (vertical slider).
        let fader_r = Rect::from_min_size(r.min + vec2(strip_w/2.0 - 10.0, 164.0), vec2(20.0, 100.0));
        let fader_h = ui.interact(fader_r, egui::Id::unique(("mix_fader", i)), Sense::click()).hovered();
        let fader_c = ui.interact(fader_r, egui::Id::unique(("mix_fader_click", i)), Sense::click()).clicked();
        ui.painter().rect_filled(fader_r, 4.0, if fader_h { Color32::from_rgb(60, 120, 255) } else { Color32::from_rgb(40, 44, 56) });
        ui.painter().rect_stroke(fader_r, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);
        if fader_c {
            let _ = crate::menus::invoke(app, &ctx, "audio.setLevel", json!({"track": i, "level": track.level}));
        }
        let level = format!("{:.1}", track.level);
        ui.painter().text(r.min + vec2(strip_w/2.0, 272.0), Align2::CENTER_CENTER, level.as_str(), Tokens::ui(10.0), Color32::from_rgb(160, 160, 170));

        // Pan knob.
        let pan_r = Rect::from_min_size(r.min + vec2(strip_w/2.0 - 12.0, 290.0), vec2(24.0, 24.0));
        let pan_h = ui.interact(pan_r, egui::Id::unique(("mix_pan", i)), Sense::click()).hovered();
        ui.painter().circle_filled(pan_r.center(), 8.0, if pan_h { Color32::from_rgb(80, 80, 90) } else { Color32::from_rgb(60, 60, 70) });
        ui.painter().circle_stroke(pan_r.center(), 8.0, egui::Stroke::new(1.0, track.color));
        let pan_x = pan_r.center().x + track.pan * 6.0;
        ui.painter().circle_filled(pos2(pan_x, pan_r.center().y), 3.0, Color32::WHITE);

        // Track color indicator at bottom.
        let color_r = Rect::from_min_size(r.min + vec2(8.0, r.height() - 16.0), vec2(strip_w - 16.0, 8.0));
        ui.painter().rect_filled(color_r, 4.0, track.color);

        app.auto.add(&format!("mix.track.{}", i), r, track.name);
    }
}
