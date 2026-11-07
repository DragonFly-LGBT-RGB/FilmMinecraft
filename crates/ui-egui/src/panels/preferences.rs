//! Preferences / Settings panel: autosave, theme, language, keyboard shortcuts, cache, and app behavior.

use egui::{Align2, Color32, Rect, Sense, vec2, pos2};

use crate::FilmcraftApp;
use crate::icons::Icon;
use crate::theme::Tokens;

/// A settings page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsPage {
    General,
    AutoSave,
    Theme,
    Shortcuts,
    Cache,
    Export,
    About,
}

const PAGES: [(SettingsPage, &'static str, Icon); 7] = [
    (SettingsPage::General, "General", Icon::Gear),
    (SettingsPage::AutoSave, "Auto-Save", Icon::Clock),
    (SettingsPage::Theme, "Appearance", Icon::Adjust),
    (SettingsPage::Shortcuts, "Keyboard", Icon::Keyframe),
    (SettingsPage::Cache, "Cache", Icon::Folder),
    (SettingsPage::Export, "Export", Icon::Export),
    (SettingsPage::About, "About", Icon::Info),
];

pub fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    let _ctx = ui.ctx().clone();

    ui.painter().rect_filled(rect, t.radius, t.panel_bg);

    // Left page selector.
    let nav_w = 160.0;
    let nav_rect = Rect::from_min_size(rect.min, vec2(nav_w, rect.height()));
    ui.painter().rect_filled(nav_rect, 0.0, Color32::from_rgb(26, 28, 36));

    ui.painter().text(
        rect.min + vec2(16.0, 32.0),
        Align2::LEFT_CENTER,
        tl!("Settings"),
        Tokens::semibold(18.0),
        Color32::WHITE,
    );

    let mut ny = rect.min.y + 70.0;
    let current_page = app.ui.settings_page.unwrap_or(SettingsPage::General);
    for (page, label, icon) in PAGES.iter() {
        let pr = Rect::from_min_size(rect.min + vec2(0.0, ny), vec2(nav_w, 48.0));
        let hovered = ui.interact(pr, egui::Id::unique(("settings_page", *page as u8)), Sense::click()).hovered();
        let clicked = ui.interact(pr, egui::Id::unique(("settings_page_click", *page as u8)), Sense::click()).clicked();
        let active = current_page == *page;
        ui.painter().rect_filled(pr, 0.0, if active { Color32::from_rgb(40, 44, 56) } else if hovered { Color32::from_rgb(36, 40, 50) } else { Color32::TRANSPARENT });
        if active {
            ui.painter().rect_filled(Rect::from_min_size(rect.min + vec2(0.0, ny), vec2(3.0, 48.0)), 0.0, Color32::from_rgb(0, 180, 216));
        }
        crate::icons::paint(ui.painter(), Rect::from_min_size(rect.min + vec2(24.0, ny + 12.0), vec2(24.0, 24.0)), *icon, if active { Color32::from_rgb(0, 180, 216) } else { Color32::from_rgb(150, 150, 160) });
        ui.painter().text(rect.min + vec2(60.0, ny + 24.0), Align2::LEFT_CENTER, crate::i18n::t(*label), Tokens::ui(13.0), if active { Color32::WHITE } else { Color32::from_rgb(180, 180, 190) });
        if clicked {
            app.ui.settings_page = Some(*page);
        }
        ny += 52.0;
    }

    // Content area.
    let content_x = rect.min.x + nav_w + 24.0;
    let content_w = rect.width() - nav_w - 40.0;
    let content_rect = Rect::from_min_size(pos2(content_x, rect.min.y + 24.0), vec2(content_w, rect.height() - 48.0));
    ui.painter().rect_filled(content_rect, 8.0, Color32::from_rgb(32, 36, 46));

    match current_page {
        SettingsPage::General => show_general(app, ui, content_rect),
        SettingsPage::AutoSave => show_autosave(app, ui, content_rect),
        SettingsPage::Theme => show_theme(app, ui, content_rect),
        SettingsPage::Shortcuts => show_shortcuts(app, ui, content_rect),
        SettingsPage::Cache => show_cache(app, ui, content_rect),
        SettingsPage::Export => show_export(app, ui, content_rect),
        SettingsPage::About => show_about(app, ui, content_rect),
    }
}

fn show_general(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    ui.painter().text(rect.min + vec2(24.0, 24.0), Align2::LEFT_CENTER, tl!("General"), Tokens::semibold(20.0), t.text);

    // Language.
    ui.painter().text(rect.min + vec2(24.0, 70.0), Align2::LEFT_CENTER, tl!("Interface Language"), Tokens::ui(13.0), Color32::from_rgb(160, 160, 170));
    let lang_label: String = match app.ui.language {
        crate::i18n::Language::Ru => tl!("Russian").to_string(),
        crate::i18n::Language::En => tl!("English").to_string(),
        _ => app.ui.language.name().to_string(),
    };
    let lang_r = Rect::from_min_size(rect.min + vec2(24.0, 90.0), vec2(200.0, 32.0));
    ui.painter().rect_filled(lang_r, 6.0, Color32::from_rgb(24, 28, 36));
    ui.painter().rect_stroke(lang_r, 6.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);
    ui.painter().text(lang_r.min + vec2(10.0, 16.0), Align2::LEFT_CENTER, lang_label, Tokens::ui(13.0), Color32::WHITE);
    // Language toggle RU/EN.
    let toggle_r = Rect::from_min_size(rect.min + vec2(232.0, 90.0), vec2(80.0, 32.0));
    let toggle_c = ui.interact(toggle_r, egui::Id::new("lang.toggle"), Sense::click()).clicked();
    ui.painter().rect_filled(toggle_r, 6.0, Color32::from_rgb(60, 60, 70));
    ui.painter().rect_stroke(toggle_r, 6.0, egui::Stroke::new(1.0, Color32::from_rgb(100, 100, 110)), egui::StrokeKind::Inside);
    ui.painter().text(toggle_r.center(), Align2::CENTER_CENTER, if app.ui.language == crate::i18n::Language::Ru { tl!("EN") } else { tl!("RU") }, Tokens::semibold(12.0), Color32::WHITE);
    if toggle_c {
        app.ui.language = if app.ui.language == crate::i18n::Language::Ru {
            crate::i18n::Language::En
        } else {
            crate::i18n::Language::Ru
        };
        crate::i18n::set_current(app.ui.language);
    }

    // Startup behavior.
    ui.painter().text(rect.min + vec2(24.0, 140.0), Align2::LEFT_CENTER, tl!("At Startup"), Tokens::ui(13.0), Color32::from_rgb(160, 160, 170));
    let startups = [tl!("Show Home"), tl!("Open Most Recent"), tl!("New Empty Project")];
    let mut sy = rect.min.y + 160.0;
    for startup in startups.iter() {
        let sr = Rect::from_min_size(rect.min + vec2(24.0, sy), vec2(200.0, 30.0));
        let hovered = ui.interact(sr, egui::Id::unique(("startup", startup)), Sense::click()).hovered();
        ui.painter().rect_filled(sr, 4.0, if hovered { Color32::from_rgb(40, 44, 56) } else { Color32::TRANSPARENT });
        ui.painter().text(sr.min + vec2(10.0, 15.0), Align2::LEFT_CENTER, startup, Tokens::ui(12.0), Color32::from_rgb(200, 200, 210));
        sy += 36.0;
    }

    // Hardware decoding.
    ui.painter().text(rect.min + vec2(24.0, 270.0), Align2::LEFT_CENTER, tl!("Hardware Decoding"), Tokens::ui(13.0), Color32::from_rgb(160, 160, 170));
    let hw_r = Rect::from_min_size(rect.min + vec2(24.0, 290.0), vec2(24.0, 24.0));
    let hw_h = ui.interact(hw_r, egui::Id::new("hw_decode"), Sense::click()).clicked();
    ui.painter().rect_filled(hw_r, 4.0, Color32::from_rgb(60, 60, 70));
    ui.painter().rect_stroke(hw_r, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(100, 100, 110)), egui::StrokeKind::Inside);
    if hw_h {
        app.ui.hardware_decoding = !app.ui.hardware_decoding;
    }
    ui.painter().text(rect.min + vec2(56.0, 302.0), Align2::LEFT_CENTER, tl!("Enable hardware-accelerated video decoding"), Tokens::ui(11.0), Color32::from_rgb(140, 140, 150));
}

fn show_autosave(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    ui.painter().text(rect.min + vec2(24.0, 24.0), Align2::LEFT_CENTER, tl!("Auto-Save"), Tokens::semibold(20.0), t.text);

    let enabled_r = Rect::from_min_size(rect.min + vec2(24.0, 70.0), vec2(24.0, 24.0));
    let enabled_h = ui.interact(enabled_r, egui::Id::new("as_enabled"), Sense::click()).clicked();
    ui.painter().rect_filled(enabled_r, 4.0, Color32::from_rgb(60, 60, 70));
    ui.painter().rect_stroke(enabled_r, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(100, 100, 110)), egui::StrokeKind::Inside);
    if enabled_h {
        app.ui.autosave.enabled = !app.ui.autosave.enabled;
    }
    ui.painter().text(rect.min + vec2(56.0, 82.0), Align2::LEFT_CENTER, tl!("Auto-save projects"), Tokens::semibold(13.0), Color32::WHITE);
    ui.painter().text(rect.min + vec2(56.0, 96.0), Align2::LEFT_CENTER, tl!("Automatically save unsaved changes periodically"), Tokens::ui(10.0), Color32::from_rgb(140, 140, 150));

    // Interval.
    ui.painter().text(rect.min + vec2(24.0, 130.0), Align2::LEFT_CENTER, tl!("Save Every"), Tokens::ui(13.0), Color32::from_rgb(160, 160, 170));
    let intervals = [tl!("1 minute"), tl!("2 minutes"), tl!("5 minutes"), tl!("10 minutes"), tl!("15 minutes")];
    let mut iy = rect.min.y + 150.0;
    for interval in intervals.iter() {
        let ir = Rect::from_min_size(rect.min + vec2(24.0, iy), vec2(180.0, 30.0));
        let hovered = ui.interact(ir, egui::Id::unique(("as_interval", interval)), Sense::click()).hovered();
        ui.painter().rect_filled(ir, 4.0, if hovered { Color32::from_rgb(40, 44, 56) } else { Color32::TRANSPARENT });
        ui.painter().text(ir.min + vec2(10.0, 15.0), Align2::LEFT_CENTER, interval, Tokens::ui(12.0), Color32::from_rgb(200, 200, 210));
        iy += 36.0;
    }
    ui.painter().text(rect.min + vec2(24.0, 340.0), Align2::LEFT_CENTER, format!("{} {}", tl!("Next save:"), if app.ui.autosave.pending { tl!("pending…") } else { tl!("scheduled") }), Tokens::ui(10.0), Color32::from_rgb(120, 120, 130));
}

fn show_theme(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    ui.painter().text(rect.min + vec2(24.0, 24.0), Align2::LEFT_CENTER, tl!("Appearance"), Tokens::semibold(20.0), t.text);

    // Theme mode.
    ui.painter().text(rect.min + vec2(24.0, 70.0), Align2::LEFT_CENTER, tl!("Theme Mode"), Tokens::ui(13.0), Color32::from_rgb(160, 160, 170));
    let themes = [tl!("Dark"), tl!("Light"), tl!("System")];
    let mut ty = rect.min.y + 90.0;
    for theme in themes.iter() {
        let tr = Rect::from_min_size(rect.min + vec2(24.0, ty), vec2(180.0, 30.0));
        let hovered = ui.interact(tr, egui::Id::unique(("theme", theme)), Sense::click()).hovered();
        ui.painter().rect_filled(tr, 4.0, if hovered { Color32::from_rgb(40, 44, 56) } else { Color32::TRANSPARENT });
        ui.painter().text(tr.min + vec2(10.0, 15.0), Align2::LEFT_CENTER, theme, Tokens::ui(12.0), Color32::from_rgb(200, 200, 210));
        ty += 36.0;
    }

    // Dark mode toggle.
    ui.painter().text(rect.min + vec2(24.0, 200.0), Align2::LEFT_CENTER, tl!("Dark Mode"), Tokens::ui(13.0), Color32::from_rgb(160, 160, 170));
    let dark_r = Rect::from_min_size(rect.min + vec2(24.0, 220.0), vec2(24.0, 24.0));
    let dark_h = ui.interact(dark_r, egui::Id::new("theme.dark"), Sense::click()).clicked();
    ui.painter().rect_filled(dark_r, 4.0, Color32::from_rgb(60, 60, 70));
    ui.painter().rect_stroke(dark_r, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(100, 100, 110)), egui::StrokeKind::Inside);
    if dark_h {
        app.ui.dark = !app.ui.dark;
    }
    if app.ui.dark {
        ui.painter().rect_filled(dark_r, 4.0, Color32::from_rgb(0, 180, 216));
    }
    ui.painter().text(rect.min + vec2(56.0, 232.0), Align2::LEFT_CENTER, tl!("Dark theme enabled"), Tokens::ui(11.0), Color32::from_rgb(140, 140, 150));

    // UI Scale.
    ui.painter().text(rect.min + vec2(24.0, 270.0), Align2::LEFT_CENTER, tl!("UI Scale"), Tokens::ui(13.0), Color32::from_rgb(160, 160, 170));
    let scale_r = Rect::from_min_size(rect.min + vec2(24.0, 290.0), vec2(200.0, 20.0));
    ui.painter().rect_filled(scale_r, 4.0, Color32::from_rgb(60, 60, 70));
    let scale_pct = 1.0; // 100%
    ui.painter().rect_filled(Rect::from_min_size(scale_r.min, vec2(scale_r.width() * scale_pct, 20.0)), 4.0, Color32::from_rgb(0, 180, 216));
    ui.painter().text(scale_r.center(), Align2::CENTER_CENTER, "100%", Tokens::ui(11.0), Color32::WHITE);
}

fn show_shortcuts(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    ui.painter().text(rect.min + vec2(24.0, 24.0), Align2::LEFT_CENTER, tl!("Keyboard Shortcuts"), Tokens::semibold(20.0), t.text);
    ui.painter().text(rect.min + vec2(24.0, 54.0), Align2::LEFT_CENTER, tl!("View and customize keyboard shortcuts for all commands."), Tokens::ui(12.0), Color32::from_rgb(160, 160, 170));

    // Show some common shortcuts.
    let shortcuts = [
        (tl!("N"), tl!("New Project")),
        (tl!("O"), tl!("Open Project")),
        (tl!("Cmd/Ctrl+S"), tl!("Save")),
        (tl!("Cmd/Ctrl+E"), tl!("Export")),
        (tl!("Space"), tl!("Play/Stop")),
        (tl!("J"), tl!("Shuttle Left")),
        (tl!("K"), tl!("Stop")),
        (tl!("L"), tl!("Shuttle Right")),
        (tl!("V"), tl!("Selection Tool")),
        (tl!("C"), tl!("Razor Tool")),
        (tl!("M"), tl!("Add Marker")),
        (tl!("Cmd/Ctrl+Z"), tl!("Undo")),
        (tl!("Cmd/Ctrl+Shift+Z"), tl!("Redo")),
    ];
    let mut sy = rect.min.y + 90.0;
    for (key, action) in shortcuts.iter() {
        let key_r = Rect::from_min_size(rect.min + vec2(24.0, sy), vec2(100.0, 30.0));
        let act_r = Rect::from_min_size(rect.min + vec2(140.0, sy), vec2(300.0, 30.0));
        ui.painter().rect_filled(key_r, 4.0, Color32::from_rgb(40, 44, 56));
        ui.painter().rect_stroke(key_r, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);
        ui.painter().text(key_r.center(), Align2::CENTER_CENTER, key, Tokens::semibold(11.0), Color32::from_rgb(0, 180, 216));
        ui.painter().text(act_r.min, Align2::LEFT_CENTER, action, Tokens::ui(12.0), Color32::from_rgb(200, 200, 210));
        sy += 36.0;
    }

    let edit_r = Rect::from_min_size(rect.min + vec2(24.0, sy + 20.0), vec2(140.0, 32.0));
    let edit_h = ui.interact(edit_r, egui::Id::new("shortcuts.edit"), Sense::click()).hovered();
    ui.painter().rect_filled(edit_r, 6.0, if edit_h { Color32::from_rgb(60, 120, 255) } else { Color32::from_rgb(40, 100, 220) });
    ui.painter().text(edit_r.center(), Align2::CENTER_CENTER, tl!("Edit Shortcuts"), Tokens::semibold(12.0), Color32::WHITE);
}

fn show_cache(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    ui.painter().text(rect.min + vec2(24.0, 24.0), Align2::LEFT_CENTER, tl!("Cache"), Tokens::semibold(20.0), t.text);

    // Cache directory info.
    ui.painter().text(rect.min + vec2(24.0, 70.0), Align2::LEFT_CENTER, tl!("Cache directory"), Tokens::ui(13.0), Color32::from_rgb(160, 160, 170));
    let cache_dir = tl!("~/.cache/FilmCraft").to_string();
    let dir_r = Rect::from_min_size(rect.min + vec2(24.0, 90.0), vec2(rect.width() - 48.0, 32.0));
    ui.painter().rect_filled(dir_r, 4.0, Color32::from_rgb(24, 28, 36));
    ui.painter().rect_stroke(dir_r, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);
    ui.painter().text(dir_r.min + vec2(10.0, 16.0), Align2::LEFT_CENTER, cache_dir, Tokens::ui(11.0), Color32::from_rgb(200, 200, 210));

    // Projects directory info.
    ui.painter().text(rect.min + vec2(24.0, 140.0), Align2::LEFT_CENTER, tl!("Projects directory"), Tokens::ui(13.0), Color32::from_rgb(160, 160, 170));
    let proj_dir = tl!("~/.local/share/FilmCraft").to_string();
    let proj_r = Rect::from_min_size(rect.min + vec2(24.0, 160.0), vec2(rect.width() - 48.0, 32.0));
    ui.painter().rect_filled(proj_r, 4.0, Color32::from_rgb(24, 28, 36));
    ui.painter().rect_stroke(proj_r, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);
    ui.painter().text(proj_r.min + vec2(10.0, 16.0), Align2::LEFT_CENTER, proj_dir, Tokens::ui(11.0), Color32::from_rgb(200, 200, 210));

    // Clear cache button.
    let clear_r = Rect::from_min_size(rect.min + vec2(24.0, 210.0), vec2(140.0, 32.0));
    let clear_h = ui.interact(clear_r, egui::Id::new("cache.clear"), Sense::click()).hovered();
    let clear_c = ui.interact(clear_r, egui::Id::new("cache.clear"), Sense::click()).clicked();
    ui.painter().rect_filled(clear_r, 6.0, if clear_h { Color32::from_rgb(255, 80, 80) } else { Color32::from_rgb(220, 60, 60) });
    ui.painter().text(clear_r.center(), Align2::CENTER_CENTER, tl!("Clear Cache"), Tokens::semibold(12.0), Color32::WHITE);
    if clear_c {
        let cache_path = std::path::Path::new("~/.cache/FilmCraft");
        if cache_path.exists() {
            let _ = std::fs::remove_dir_all(cache_path);
            let _ = std::fs::create_dir_all(cache_path);
        }
    }

    // API keys section.
    ui.painter().text(rect.min + vec2(24.0, 260.0), Align2::LEFT_CENTER, tl!("Audio Search API Keys"), Tokens::ui(13.0), Color32::from_rgb(160, 160, 170));
    ui.painter().text(rect.min + vec2(24.0, 280.0), Align2::LEFT_CENTER, tl!("Stored locally only, used in M5."), Tokens::ui(10.0), Color32::from_rgb(120, 120, 130));

    let fs_r = Rect::from_min_size(rect.min + vec2(24.0, 300.0), vec2(rect.width() - 48.0, 32.0));
    ui.painter().rect_filled(fs_r, 4.0, Color32::from_rgb(24, 28, 36));
    ui.painter().rect_stroke(fs_r, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);
    ui.painter().text(fs_r.min + vec2(10.0, 16.0), Align2::LEFT_CENTER, tl!("Freesound API key"), Tokens::ui(12.0), Color32::WHITE);
    // Text field placeholder — stored in app state would be ideal; here we show a read-only info.
    ui.painter().text(fs_r.max - vec2(120.0, 16.0), Align2::RIGHT_CENTER, tl!("Not configured"), Tokens::ui(10.0), Color32::from_rgb(140, 140, 150));

    let px_r = Rect::from_min_size(rect.min + vec2(24.0, 340.0), vec2(rect.width() - 48.0, 32.0));
    ui.painter().rect_filled(px_r, 4.0, Color32::from_rgb(24, 28, 36));
    ui.painter().rect_stroke(px_r, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);
    ui.painter().text(px_r.min + vec2(10.0, 16.0), Align2::LEFT_CENTER, tl!("Pixabay API key"), Tokens::ui(12.0), Color32::WHITE);
    ui.painter().text(px_r.max - vec2(120.0, 16.0), Align2::RIGHT_CENTER, tl!("Not configured"), Tokens::ui(10.0), Color32::from_rgb(140, 140, 150));
}

fn show_about(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    ui.painter().text(rect.min + vec2(24.0, 40.0), Align2::LEFT_CENTER, tl!("FilmCraft"), Tokens::semibold(32.0), Color32::WHITE);
    ui.painter().text(rect.min + vec2(24.0, 80.0), Align2::LEFT_CENTER, tl!("Open-source nonlinear video editor"), Tokens::ui(14.0), Color32::from_rgb(160, 160, 170));
    ui.painter().text(rect.min + vec2(24.0, 110.0), Align2::LEFT_CENTER, tl!("Version 0.17.1 (Alpha)"), Tokens::ui(12.0), Color32::from_rgb(120, 120, 130));

    let info = [
        tl!("License: MIT OR Apache-2.0"),
        tl!("Repository: github.com/storytold/filmcraft"),
        tl!("Built with: Rust, egui, wgpu"),
        tl!("Platforms: macOS, Windows, Linux, FreeBSD, WebAssembly"),
    ];
    let mut iy = rect.min.y + 150.0;
    for line in info.iter() {
        ui.painter().text(rect.min + vec2(24.0, iy), Align2::LEFT_CENTER, line, Tokens::ui(12.0), Color32::from_rgb(200, 200, 210));
        iy += 26.0;
    }

    let discord_r = Rect::from_min_size(rect.min + vec2(24.0, iy + 30.0), vec2(160.0, 34.0));
    let discord_h = ui.interact(discord_r, egui::Id::new("about.discord"), Sense::click()).hovered();
    ui.painter().rect_filled(discord_r, 6.0, if discord_h { Color32::from_rgb(80, 100, 255) } else { Color32::from_rgb(60, 80, 220) });
    ui.painter().text(discord_r.center(), Align2::CENTER_CENTER, tl!("Join Discord Community"), Tokens::semibold(12.0), Color32::WHITE);
}

/// Export settings page: ffmpeg binary path for Path A ffmpeg-based encoding.
fn show_export(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    ui.painter().text(rect.min + vec2(24.0, 24.0), Align2::LEFT_CENTER, tl!("Export"), Tokens::semibold(20.0), t.text);
    ui.painter().text(
        rect.min + vec2(24.0, 54.0),
        Align2::LEFT_CENTER,
        tl!("FFmpeg binary path for export encoding (leave empty to auto-detect)"),
        Tokens::ui(12.0),
        Color32::from_rgb(160, 160, 170),
    );

    let ffmpeg_r = Rect::from_min_size(rect.min + vec2(24.0, 80.0), vec2(rect.width() - 48.0, 34.0));
    let ffmpeg_h = ui.interact(ffmpeg_r, egui::Id::new("export.ffmpeg.path"), Sense::click()).hovered();
    ui.painter().rect_filled(ffmpeg_r, 4.0, if ffmpeg_h { Color32::from_rgb(40, 44, 56) } else { Color32::from_rgb(24, 28, 36) });
    ui.painter().rect_stroke(ffmpeg_r, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);
    let display = if app.ui.export_ffmpeg_path.is_empty() {
        tl!("(auto-detect)").to_string()
    } else {
        app.ui.export_ffmpeg_path.clone()
    };
    ui.painter().text(ffmpeg_r.min + vec2(10.0, 17.0), Align2::LEFT_CENTER, &display, Tokens::ui(12.0), Color32::from_rgb(200, 200, 210));

    // Probe ffmpeg and show available encoders.
    let probe = filmcraft_export::probe();
    let encoders_y = 130.0;
    ui.painter().text(rect.min + vec2(24.0, encoders_y), Align2::LEFT_CENTER, tl!("Detected encoders"), Tokens::ui(13.0), Color32::from_rgb(160, 160, 170));
    let encoders = [
        ("H.264", probe.h264),
        ("H.265 / HEVC", probe.hevc),
        ("VP9 / WebM", probe.vp9),
        ("AAC audio", probe.aac),
        ("NVENC", probe.nvenc),
        ("VideoToolbox", probe.videotoolbox),
        ("QSV", probe.qsv),
    ];
    let mut ey = encoders_y + 26.0;
    for (name, ok) in encoders.iter() {
        ui.painter().text(rect.min + vec2(24.0, ey), Align2::LEFT_CENTER, *name, Tokens::ui(11.0), if *ok { Color32::from_rgb(100, 255, 100) } else { Color32::from_rgb(150, 150, 160) });
        ey += 20.0;
    }

    let ffmpeg_path_y = ey + 10.0;
    ui.painter().text(rect.min + vec2(24.0, ffmpeg_path_y), Align2::LEFT_CENTER, tl!("Custom ffmpeg path"), Tokens::ui(13.0), Color32::from_rgb(160, 160, 170));
    let path_r = Rect::from_min_size(rect.min + vec2(24.0, ffmpeg_path_y + 22.0), vec2(rect.width() - 48.0, 34.0));
    let path_h = ui.interact(path_r, egui::Id::new("export.ffmpeg.custom"), Sense::click()).hovered();
    ui.painter().rect_filled(path_r, 4.0, if path_h { Color32::from_rgb(40, 44, 56) } else { Color32::from_rgb(24, 28, 36) });
    ui.painter().rect_stroke(path_r, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);
    ui.painter().text(path_r.min + vec2(10.0, 17.0), Align2::LEFT_CENTER, &app.ui.export_ffmpeg_path, Tokens::ui(12.0), Color32::from_rgb(200, 200, 210));
}
