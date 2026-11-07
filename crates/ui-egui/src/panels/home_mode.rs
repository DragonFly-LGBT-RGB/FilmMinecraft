//! Home mode — CapCut-style project manager.
//! Shown at startup instead of the demo project. Lists local projects, lets you
//! create new ones, import media, open recent projects, and restore after a crash.

use std::path::Path;

use egui::{Align2, Color32, Rect, Sense, vec2, pos2};
use serde_json::json;

use crate::FilmcraftApp;
use crate::icons::Icon;
use crate::menus;
use crate::panels::preferences::SettingsPage;
use crate::panels::projects::{
    CanvasPreset, CrashCandidate, FPS_OPTIONS, HomeState, ProjectInfo, ProjectManager,
    ProjectSort,
};
use crate::theme::Tokens;

/// Render the project manager home screen.
pub fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let ctx = ui.ctx().clone();

    // Take home_state out of app to avoid borrow conflicts.
    let mut home_state = app.ui.home_state.take().unwrap_or_else(|| {
        HomeState::new(Path::new("."))
    });

    // Background.
    ui.painter().rect_filled(rect, 0.0, app.tokens.app_bg);

    // Extract pending dialog actions before any immutable borrow of home_state.
    let pending_delete = home_state.pending_delete.take();
    let pending_rename = home_state.pending_rename.take();

    // ================= Crash recovery on startup =================
    if home_state.crash_recovery.is_none()
        && app.ui.settings_page.is_none()
        && !app.session.recovery_candidates().is_empty()
    {
        let cand = &app.session.recovery_candidates()[0];
        home_state.crash_recovery = Some(CrashCandidate {
            id: cand.id.clone(),
            project_name: cand.meta.project_name.clone(),
            modified_at: cand.meta.saved_unix as u64,
        });
    }

    // ================= Crash recovery dialog =================
    if let Some(candidate) = home_state.crash_recovery.take() {
        show_crash_recovery_dialog(app, &ctx, rect, &mut home_state, candidate);
        app.ui.home_state = Some(home_state);
        return;
    }

    // ================= Header =================
    let header_h = 56.0;
    let header_rect = Rect::from_min_size(rect.min, vec2(rect.width(), header_h));
    ui.painter().rect_filled(header_rect, 0.0, app.tokens.app_bg);

    ui.painter().text(
        header_rect.min + vec2(24.0, 0.0),
        Align2::LEFT_CENTER,
        tl!("FilmCraft"),
        Tokens::semibold(18.0),
        Color32::WHITE,
    );

    let btn_w = 120.0;
    let btn_h = 34.0;

    // Create new button.
    let create_r = Rect::from_min_size(
        header_rect.max - vec2(24.0 + btn_w + 16.0 + btn_w + 44.0, 0.0) + vec2(0.0, (header_h - btn_h) / 2.0),
        vec2(btn_w, btn_h),
    );
    let create_c = ui.interact(create_r, egui::Id::new("home.create_new"), Sense::click()).clicked();
    ui.painter().rect_filled(create_r, 8.0, Color32::from_rgb(40, 100, 220));
    ui.painter().text(create_r.center(), Align2::CENTER_CENTER, tl!("Create new"), Tokens::semibold(13.0), Color32::WHITE);
    if create_c {
        home_state.create_dialog_open = true;
    }

    // Import media button.
    let import_r = Rect::from_min_size(
        header_rect.max - vec2(24.0 + btn_w + 44.0, 0.0) + vec2(0.0, (header_h - btn_h) / 2.0),
        vec2(btn_w, btn_h),
    );
    let import_c = ui.interact(import_r, egui::Id::new("home.import_media"), Sense::click()).clicked();
    ui.painter().rect_filled(import_r, 8.0, Color32::from_rgb(45, 50, 60));
    ui.painter().rect_stroke(import_r, 8.0, egui::Stroke::new(1.0, Color32::from_rgb(70, 80, 90)), egui::StrokeKind::Inside);
    ui.painter().text(import_r.center(), Align2::CENTER_CENTER, tl!("Import media"), Tokens::semibold(13.0), Color32::from_rgb(200, 200, 210));
    if import_c {
        let _ = menus::invoke(app, &ctx, "file.import", json!({}));
    }

    // Settings button (gear).
    let settings_r = Rect::from_min_size(header_rect.max - vec2(80.0, 0.0) + vec2(0.0, (header_h - 32.0) / 2.0), vec2(32.0, 32.0));
    let settings_c = ui.interact(settings_r, egui::Id::new("home.settings"), Sense::click()).clicked();
    ui.painter().circle_filled(settings_r.center(), 16.0, Color32::from_rgb(40, 100, 220));
    crate::icons::paint(ui.painter(), Rect::from_min_size(settings_r.center() - vec2(10.0, 10.0), vec2(20.0, 20.0)), Icon::Gear, Color32::WHITE);
    if settings_c {
        app.ui.settings_page = Some(SettingsPage::General);
    }

    // Profile avatar.
    let profile_r = Rect::from_min_size(header_rect.max - vec2(44.0, 0.0) + vec2(0.0, (header_h - 32.0) / 2.0), vec2(32.0, 32.0));
    ui.painter().circle_filled(profile_r.center(), 16.0, Color32::from_rgb(100, 100, 110));
    ui.painter().text(profile_r.center(), Align2::CENTER_CENTER, "U", Tokens::semibold(14.0), Color32::WHITE);

    // ================= Continue section =================
    let continue_y = header_h + 24.0;
    let continue_rect = Rect::from_min_size(rect.min + vec2(24.0, continue_y), vec2(rect.width() - 48.0, 100.0));
    let has_projects = !home_state.manager.projects.is_empty();

    if has_projects {
        if let Some(recent) = home_state.manager.projects.iter().max_by_key(|p| p.modified_at) {
            let continue_clicked = ui.interact(continue_rect, egui::Id::new("home.continue"), Sense::click()).double_clicked();
            ui.painter().rect_filled(continue_rect, 12.0, Color32::from_rgb(32, 36, 46));
            if ui.interact(continue_rect, egui::Id::new("home.continue_hover"), Sense::hover()).hovered() {
                ui.painter().rect_stroke(continue_rect, 12.0, egui::Stroke::new(1.0, Color32::from_rgb(0, 180, 216)), egui::StrokeKind::Inside);
            }

            let thumb = Rect::from_min_size(continue_rect.min + vec2(16.0, 16.0), vec2(68.0, 68.0));
            ui.painter().rect_filled(thumb, 8.0, Color32::from_rgb(60, 120, 200));
            crate::icons::paint(ui.painter(), Rect::from_min_size(thumb.center() - vec2(16.0, 16.0), vec2(32.0, 32.0)), Icon::Film, Color32::WHITE);

            let dur = Rect::from_min_size(thumb.max - vec2(34.0, 14.0), vec2(30.0, 14.0));
            ui.painter().rect_filled(dur, 7.0, Color32::BLACK.linear_multiply(0.6));
            ui.painter().text(dur.center(), Align2::CENTER_CENTER, format_duration(recent.duration_secs), Tokens::ui(9.0), Color32::WHITE);

            ui.painter().text(
                continue_rect.min + vec2(100.0, 30.0),
                Align2::LEFT_CENTER,
                &recent.name,
                Tokens::semibold(16.0),
                Color32::WHITE,
            );
            ui.painter().text(
                continue_rect.min + vec2(100.0, 52.0),
                Align2::LEFT_CENTER,
                format!("{} • {} fps • {}", recent.resolution, recent.fps, recent.aspect),
                Tokens::ui(11.0),
                Color32::from_rgb(160, 160, 170),
            );
            ui.painter().text(
                continue_rect.min + vec2(100.0, 72.0),
                Align2::LEFT_CENTER,
                format!("{} {}", tl!("Modified"), format_date(recent.modified_at)),
                Tokens::ui(10.0),
                Color32::from_rgb(120, 120, 130),
            );

            ui.painter().text(
                continue_rect.max - vec2(80.0, 50.0),
                Align2::RIGHT_CENTER,
                tl!("Continue"),
                Tokens::semibold(14.0),
                Color32::from_rgb(0, 180, 216),
            );

            if continue_clicked {
                open_project(app, &ctx, &recent.id);
                app.ui.mode = crate::state::Mode::Edit;
            }
        }
    } else {
        ui.painter().rect_filled(continue_rect, 12.0, Color32::from_rgb(32, 36, 46));
        ui.painter().text(
            continue_rect.center() - vec2(0.0, 10.0),
            Align2::CENTER_CENTER,
            tl!("No projects yet"),
            Tokens::semibold(18.0),
            Color32::WHITE,
        );
        ui.painter().text(
            continue_rect.center() + vec2(0.0, 18.0),
            Align2::CENTER_CENTER,
            tl!("Create your first project or import media to get started."),
            Tokens::ui(13.0),
            Color32::from_rgb(160, 160, 170),
        );
    }

    // ================= Search + Sort =================
    let bar_y = continue_y + 116.0;
    let search_rect = Rect::from_min_size(rect.min + vec2(24.0, bar_y), vec2(300.0, 32.0));
    ui.painter().rect_filled(search_rect, 6.0, Color32::from_rgb(24, 28, 36));
    ui.painter().rect_stroke(search_rect, 6.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);
    ui.painter().text(search_rect.min + vec2(10.0, 16.0), Align2::LEFT_CENTER, "🔍", Tokens::ui(14.0), Color32::from_rgb(120, 120, 130));

    let search_events: Vec<egui::Event> = ctx.input(|i| i.events.iter().cloned().collect());
    for ev in search_events.iter() {
        if let egui::Event::Text(c) = ev {
            home_state.manager.search.push_str(c);
        }
        if let egui::Event::Key { key: egui::Key::Backspace, pressed: true, .. } = ev {
            home_state.manager.search.pop();
        }
    }
    ui.painter().text(
        search_rect.min + vec2(30.0, 16.0),
        Align2::LEFT_CENTER,
        if home_state.manager.search.is_empty() { tl!("Search projects…") } else { &home_state.manager.search },
        Tokens::ui(13.0),
        if home_state.manager.search.is_empty() { Color32::from_rgb(120, 120, 130) } else { Color32::WHITE },
    );

    // Sort dropdown.
    let sort_rect = Rect::from_min_size(rect.max - vec2(140.0 + 24.0, bar_y), vec2(140.0, 32.0));
    let sort_c = ui.interact(sort_rect, egui::Id::new("home.sort"), Sense::click()).clicked();
    ui.painter().rect_filled(sort_rect, 6.0, Color32::from_rgb(24, 28, 36));
    ui.painter().rect_stroke(sort_rect, 6.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Inside);
    let sort_label = match home_state.manager.sort {
        ProjectSort::ModifiedDesc => tl!("Date modified"),
        ProjectSort::NameAsc => tl!("Name"),
        ProjectSort::DurationAsc => tl!("Duration"),
    };
    ui.painter().text(sort_rect.min + vec2(10.0, 16.0), Align2::LEFT_CENTER, sort_label, Tokens::ui(13.0), Color32::WHITE);
    ui.painter().text(sort_rect.max - vec2(20.0, 16.0), Align2::RIGHT_CENTER, "▾", Tokens::ui(12.0), Color32::from_rgb(120, 120, 130));
    if sort_c {
        home_state.manager.sort = match home_state.manager.sort {
            ProjectSort::ModifiedDesc => ProjectSort::NameAsc,
            ProjectSort::NameAsc => ProjectSort::DurationAsc,
            ProjectSort::DurationAsc => ProjectSort::ModifiedDesc,
        };
        home_state.manager.apply_sort();
    }

    // ================= Templates row =================
    let tmpl_y = bar_y + 48.0;
    ui.painter().text(
        pos2(rect.min.x + 24.0, tmpl_y),
        Align2::LEFT_CENTER,
        tl!("Templates"),
        Tokens::semibold(14.0),
        Color32::from_rgb(160, 160, 170),
    );

    let templates = [
        (tl!("Intro"), tl!("Quick title animation"), Icon::Film, Color32::from_rgb(255, 80, 80)),
        (tl!("Slideshow"), tl!("Photo montage with music"), Icon::Image, Color32::from_rgb(80, 180, 255)),
        (tl!("Title card"), tl!("Text overlay template"), Icon::Chat, Color32::from_rgb(140, 100, 255)),
    ];
    let tmpl_w = 140.0;
    let tmpl_h = 90.0;
    let tmpl_gap = 16.0;
    let mut tmpl_x = rect.min.x + 24.0;
    for (name, desc, icon, color) in templates.iter() {
        let tr = Rect::from_min_size(pos2(tmpl_x, tmpl_y + 24.0), vec2(tmpl_w, tmpl_h));
        let clicked = ui.interact(tr, egui::Id::new(("tmpl", *name)), Sense::click()).clicked();
        ui.painter().rect_filled(tr, 8.0, *color);
        crate::icons::paint(ui.painter(), Rect::from_min_size(tr.min + vec2(14.0, 14.0), vec2(32.0, 32.0)), *icon, Color32::WHITE);
        ui.painter().text(tr.min + vec2(14.0, 58.0), Align2::LEFT_CENTER, name, Tokens::semibold(12.0), Color32::WHITE);
        ui.painter().text(tr.min + vec2(14.0, 74.0), Align2::LEFT_CENTER, desc, Tokens::ui(9.0), Color32::from_rgb(220, 220, 230));
        if clicked {
            home_state.create_dialog_open = true;
        }
        tmpl_x += tmpl_w + tmpl_gap;
    }

    // ================= Project grid =================
    let grid_y = tmpl_y + 24.0 + tmpl_h + 24.0;
    ui.painter().text(
        pos2(rect.min.x + 24.0, grid_y),
        Align2::LEFT_CENTER,
        if has_projects { tl!("Your projects") } else { tl!("Projects") },
        Tokens::semibold(16.0),
        Color32::WHITE,
    );

    let grid_rect = Rect::from_min_size(rect.min + vec2(24.0, grid_y + 24.0), vec2(rect.width() - 48.0, rect.height() - grid_y - 48.0 - 40.0));
    ui.painter().rect_filled(grid_rect, 8.0, Color32::from_rgb(24, 28, 36));

    let filtered = home_state.filtered();
    let mut delete_target: Option<String> = None;
    let mut rename_target: Option<String> = None;
    if filtered.is_empty() && !home_state.manager.search.is_empty() {
        ui.painter().text(grid_rect.center(), Align2::CENTER_CENTER, tl!("No matching projects"), Tokens::ui(14.0), Color32::from_rgb(120, 120, 130));
    } else if filtered.is_empty() && has_projects {
        ui.painter().text(grid_rect.center(), Align2::CENTER_CENTER, tl!("No projects match your search"), Tokens::ui(14.0), Color32::from_rgb(120, 120, 130));
    } else {
        let card_w = 260.0;
        let card_h = 160.0;
        let gap = 20.0;
        let cols = ((grid_rect.width() + gap) / (card_w + gap)).floor() as usize;

        for (i, project) in filtered.iter().enumerate() {
            let row = i / cols;
            let col = i % cols;
            let x = grid_rect.min.x + 12.0 + col as f32 * (card_w + gap);
            let y = grid_rect.min.y + 12.0 + row as f32 * (card_h + gap);
            let r = Rect::from_min_size(pos2(x, y), vec2(card_w, card_h));
            let card_resp = ui.interact(r, egui::Id::new(("proj", &project.id)), Sense::click());
            let hovered = card_resp.hovered();
            let double_clicked = card_resp.double_clicked();

            ui.painter().rect_filled(r, 10.0, Color32::from_rgb(32, 36, 46));
            if hovered {
                ui.painter().rect_stroke(r, 10.0, egui::Stroke::new(1.0, Color32::from_rgb(0, 180, 216)), egui::StrokeKind::Inside);
            }

            let thumb = Rect::from_min_size(r.min + vec2(12.0, 12.0), vec2(card_w - 24.0, 96.0));
            ui.painter().rect_filled(thumb, 6.0, Color32::from_rgb(60, 120, 200));
            crate::icons::paint(ui.painter(), Rect::from_min_size(thumb.center() - vec2(20.0, 20.0), vec2(40.0, 40.0)), Icon::Film, Color32::WHITE);

            let badge = Rect::from_min_size(r.min + vec2(12.0, r.height() - 26.0), vec2(44.0, 16.0));
            ui.painter().rect_filled(badge, 4.0, Color32::BLACK.linear_multiply(0.5));
            ui.painter().text(badge.center(), Align2::CENTER_CENTER, &project.aspect, Tokens::ui(8.0), Color32::WHITE);

            let dur = Rect::from_min_size(thumb.max - vec2(36.0, 16.0), vec2(32.0, 14.0));
            ui.painter().rect_filled(dur, 7.0, Color32::BLACK.linear_multiply(0.6));
            ui.painter().text(dur.center(), Align2::CENTER_CENTER, format_duration(project.duration_secs), Tokens::ui(8.0), Color32::WHITE);

            ui.painter().text(r.min + vec2(12.0, 118.0), Align2::LEFT_CENTER, &project.name, Tokens::semibold(13.0), Color32::WHITE);
            ui.painter().text(r.min + vec2(12.0, 134.0), Align2::LEFT_CENTER, format!("{} clips • {}", project.clip_count, project.resolution), Tokens::ui(10.0), Color32::from_rgb(140, 140, 150));

            if double_clicked {
                open_project(app, &ctx, &project.id);
                app.ui.mode = crate::state::Mode::Edit;
            }

            // Inline action buttons on hover (right side of card).
            if hovered {
                let btn_w = 22.0;
                let btn_h = 22.0;
                let gap = 4.0;
                let mut bx = r.max.x - btn_w - 12.0;

                // Delete button.
                let del_r = Rect::from_min_size(r.min + vec2(bx, r.min.y + 10.0), vec2(btn_w, btn_h));
                let del_c = ui.interact(del_r, egui::Id::new(("del", &project.id)), Sense::click()).clicked();
                ui.painter().rect_filled(del_r, 4.0, Color32::from_rgb(220, 60, 60));
                ui.painter().text(del_r.center(), Align2::CENTER_CENTER, "×", Tokens::semibold(12.0), Color32::WHITE);
                if del_c {
                    delete_target = Some(project.id.clone());
                }
                bx -= btn_w + gap;

                // Duplicate button.
                let dup_r = Rect::from_min_size(r.min + vec2(bx, r.min.y + 10.0), vec2(btn_w, btn_h));
                let dup_c = ui.interact(dup_r, egui::Id::new(("dup", &project.id)), Sense::click()).clicked();
                ui.painter().rect_filled(dup_r, 4.0, Color32::from_rgb(100, 100, 110));
                ui.painter().text(dup_r.center(), Align2::CENTER_CENTER, "⧉", Tokens::semibold(10.0), Color32::WHITE);
                if dup_c {
                    duplicate_project(app, &project.id);
                }
                bx -= btn_w + gap;

                // Rename button.
                let ren_r = Rect::from_min_size(r.min + vec2(bx, r.min.y + 10.0), vec2(btn_w, btn_h));
                let ren_c = ui.interact(ren_r, egui::Id::new(("ren", &project.id)), Sense::click()).clicked();
                ui.painter().rect_filled(ren_r, 4.0, Color32::from_rgb(100, 100, 110));
                ui.painter().text(ren_r.center(), Align2::CENTER_CENTER, "✎", Tokens::semibold(10.0), Color32::WHITE);
                if ren_c {
                    rename_target = Some(project.id.clone());
                }
            }
        }
    }

    // Move pending actions into home_state after the immutable borrow ends.
    home_state.pending_delete = delete_target;
    home_state.pending_rename = rename_target;

    // ================= Bottom bar =================
    let bottom_y = rect.max.y - 24.0;
    ui.painter().text(
        pos2(rect.min.x + 24.0, bottom_y),
        Align2::LEFT_CENTER,
        tl!("FilmCraft • Open-source nonlinear video editor"),
        Tokens::ui(11.0),
        Color32::from_rgb(100, 100, 110),
    );
    ui.painter().text(
        pos2(rect.max.x - 200.0, bottom_y),
        Align2::RIGHT_CENTER,
        tl!("N  New project   O  Open   Esc  Back"),
        Tokens::ui(10.0),
        Color32::from_rgb(100, 100, 110),
    );

    // ================= Delete confirmation dialog =================
    if let Some(id) = pending_delete {
        show_delete_confirm_dialog(&ctx, rect, &id, &mut home_state);
    }

    // ================= Rename dialog =================
    if let Some(id) = pending_rename {
        show_rename_dialog(&ctx, rect, &id, &mut home_state);
    }

    // ================= Create project dialog =================
    if home_state.create_dialog_open {
        show_create_project_dialog(app, &ctx, rect, &mut home_state);
    }

    // ================= Settings / preferences panel =================
    if app.ui.settings_page.is_some() {
        let w = 900.0;
        let h = 600.0;
        egui::Window::new(tl!("Settings"))
            .title_bar(false)
            .resizable(false)
            .fixed_size(egui::vec2(w, h))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(&ctx, |ui| {
                ui.set_min_size(egui::vec2(w, h));
                crate::panels::preferences::show(app, ui, ui.available_rect_before_wrap());
                if app.ui.settings_page.is_none() {
                    ui.close();
                }
            });
    }

    app.ui.home_state = Some(home_state);
}

/// Open a project by id — delegates to the engine's file.open command.
fn open_project(app: &mut FilmcraftApp, ctx: &egui::Context, id: &str) {
    let info = app.ui.home_state.as_ref().and_then(|h| h.manager.find(id));
    if let Some(info) = info {
        let path = info.path.to_string_lossy().to_string();
        let _ = menus::invoke(app, ctx, "file.open", json!({"path": path}));
    }
}

/// Duplicate a project by copying its folder (including media/).
fn duplicate_project(app: &mut FilmcraftApp, id: &str) {
    if let Some(info) = app.ui.home_state.as_ref().and_then(|h| h.manager.find(id)) {
        let src = info.path.clone();
        let new_name = format!("{} {}", info.name, tl!("Copy"));
        if let Some(new_info) = app.ui.home_state.as_mut().and_then(|h| h.manager.create_project(&new_name, CanvasPreset::W16_9, 30, None)) {
            let _ = copy_dir_all(&src, &new_info.path);
        }
    }
}

/// Copy a directory recursively.
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            std::fs::copy(&entry.path(), &dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// Delete confirmation dialog.
fn show_delete_confirm_dialog(
    ctx: &egui::Context,
    _rect: Rect,
    id: &str,
    state: &mut HomeState,
) {
    let w = 340.0;
    let h = 180.0;
    let project_name = state.manager.find(id).map(|p| p.name.clone()).unwrap_or_default();

    egui::Window::new(tl!("Delete project"))
        .title_bar(false)
        .resizable(false)
        .fixed_size(egui::vec2(w, h))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_min_size(egui::vec2(w, h));
            ui.vertical_centered(|ui| {
                ui.add_space(16.0);
                ui.label(tl!("Delete project \"{name}\"?"));
                ui.add_space(8.0);
                ui.label(tl!("This cannot be undone. Media files will also be removed."));
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    if ui.button(tl!("Cancel")).clicked() {
                        ui.close();
                    }
                    if ui.button(tl!("Delete")).clicked() {
                        state.manager.delete_project(id);
                        ui.close();
                    }
                });
                ui.add_space(16.0);
            });
        });
}

/// Rename dialog — inline text input for project name.
fn show_rename_dialog(
    ctx: &egui::Context,
    _rect: Rect,
    id: &str,
    state: &mut HomeState,
) {
    let w = 340.0;
    let h = 180.0;

    if state.pending_rename_buffer.is_none() {
        let current = state.manager.find(id).map(|p| p.name.clone()).unwrap_or_default();
        state.pending_rename_buffer = Some(current);
    }

    egui::Window::new(tl!("Rename project"))
        .title_bar(false)
        .resizable(false)
        .fixed_size(egui::vec2(w, h))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_min_size(egui::vec2(w, h));
            ui.vertical_centered(|ui| {
                ui.add_space(16.0);
                ui.label(tl!("New name:"));
                ui.add_space(8.0);
                let buffer = state.pending_rename_buffer.as_mut().unwrap();
                ui.text_edit_singleline(buffer);
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    if ui.button(tl!("Cancel")).clicked() {
                        ui.close();
                    }
                    if ui.button(tl!("OK")).clicked() {
                        let new_name = buffer.clone();
                        if !new_name.trim().is_empty() {
                            if let Some(p) = state.manager.projects.iter_mut().find(|pp| pp.id == id) {
                                p.name = new_name.clone();
                                if let Some(data_dir) = &state.manager.data_dir {
                                    let meta_path = data_dir.join(id).join("project.json");
                                    if let Ok(json) = serde_json::to_string_pretty(p) {
                                        let _ = std::fs::write(&meta_path, json);
                                    }
                                }
                            }
                        }
                        ui.close();
                    }
                });
                ui.add_space(16.0);
            });
        });
}

/// Format duration seconds as MM:SS or HH:MM:SS.
fn format_duration(secs: f64) -> String {
    let total = secs as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{:02}:{:02}", m, s)
    }
}

/// Format unix timestamp as a relative or absolute date string.
fn format_date(unix: u64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ok = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let diff = ok.saturating_sub(unix);
    if diff < 60 {
        tl!("Just now").to_string()
    } else if diff < 3600 {
        tlf!("{} min ago", n = diff / 60)
    } else if diff < 86400 {
        tlf!("{} h ago", n = diff / 3600)
    } else {
        tlf!("{} days ago", n = diff / 86400)
    }
}

/// Crash recovery dialog.
fn show_crash_recovery_dialog(
    app: &mut FilmcraftApp,
    ctx: &egui::Context,
    _rect: Rect,
    state: &mut HomeState,
    candidate: CrashCandidate,
) {
    let w = 420.0;
    let h = 240.0;
    egui::Window::new(tl!("Crash recovery"))
        .title_bar(false)
        .resizable(false)
        .fixed_size(egui::vec2(w, h))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_min_size(egui::vec2(w, h));
            ui.vertical_centered(|ui| {
                ui.add_space(16.0);
                ui.label(tl!("FilmCraft closed unexpectedly while working on:"));
                ui.add_space(8.0);
                ui.label(egui::RichText::new(&candidate.project_name).strong().size(18.0));
                ui.add_space(16.0);
                ui.label(tl!("Recover the project and continue editing, or start over:"));
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    if ui.button(tl!("Recover")).clicked() {
                        state.crash_recovery = None;
                        open_project(app, ctx, &candidate.id);
                        app.ui.mode = crate::state::Mode::Edit;
                        ui.close();
                    }
                    if ui.button(tl!("Start over")).clicked() {
                        state.crash_recovery = None;
                        ui.close();
                    }
                    if ui.button(tl!("Delete recovery")).clicked() {
                        state.crash_recovery = None;
                        ui.close();
                    }
                });
                ui.add_space(16.0);
            });
        });
}

/// Create project dialog — name, aspect ratio, FPS.
fn show_create_project_dialog(
    app: &mut FilmcraftApp,
    ctx: &egui::Context,
    _rect: Rect,
    state: &mut HomeState,
) {
    let w = 440.0;
    let h = 520.0;
    egui::Window::new(tl!("Create new project"))
        .title_bar(false)
        .resizable(false)
        .fixed_size(egui::vec2(w, h))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_min_size(egui::vec2(w, h));
            ui.vertical(|ui| {
                ui.add_space(12.0);
                ui.label(egui::RichText::new(tl!("Create new project")).strong().size(20.0));
                ui.add_space(16.0);

                // Project name.
                ui.label(tl!("Project name"));
                ui.text_edit_singleline(&mut state.new_project_name);
                ui.add_space(8.0);

                // Canvas preset.
                ui.label(tl!("Canvas preset"));
                egui::ComboBox::from_id_salt("canvas_preset")
                    .selected_text(state.new_project_preset.label())
                    .show_ui(ui, |ui| {
                        for preset in [
                            CanvasPreset::W16_9,
                            CanvasPreset::H9_16,
                            CanvasPreset::S1_1,
                            CanvasPreset::P4_5,
                            CanvasPreset::W21_9,
                            CanvasPreset::Custom,
                        ] {
                            if ui.selectable_label(state.new_project_preset == preset, preset.label()).clicked() {
                                state.new_project_preset = preset;
                            }
                        }
                    });
                ui.add_space(8.0);

                // FPS.
                ui.label(tl!("Frame rate"));
                egui::ComboBox::from_id_salt("fps")
                    .selected_text(format!("{} fps", state.new_project_fps))
                    .show_ui(ui, |ui| {
                        for &fps in FPS_OPTIONS.iter() {
                            if ui.selectable_label(state.new_project_fps == fps, format!("{} fps", fps)).clicked() {
                                state.new_project_fps = fps;
                            }
                        }
                    });
                ui.add_space(8.0);

                // Custom resolution (only visible for Custom preset).
                if state.new_project_preset == CanvasPreset::Custom {
                    ui.label(tl!("Resolution"));
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut state.new_project_custom_w).range(100..=7680).suffix(" px"));
                        ui.label("×");
                        ui.add(egui::DragValue::new(&mut state.new_project_custom_h).range(100..=4320).suffix(" px"));
                    });
                    ui.add_space(8.0);
                }

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    if ui.button(tl!("Cancel")).clicked() {
                        state.create_dialog_open = false;
                        ui.close();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let can_create = !state.new_project_name.trim().is_empty();
                        let create_btn = ui.add_enabled(can_create, egui::Button::new(tl!("Create")).fill(Color32::from_rgb(0, 180, 216)));
                        if create_btn.clicked() {
                            let preset = state.new_project_preset;
                            let fps = state.new_project_fps;
                            let custom_res = if preset == CanvasPreset::Custom {
                                Some((state.new_project_custom_w, state.new_project_custom_h))
                            } else {
                                None
                            };
                            if let Some(info) = state.manager.create_project(
                                &state.new_project_name,
                                preset,
                                fps,
                                custom_res,
                            ) {
                                open_project(app, ctx, &info.id);
                                app.ui.mode = crate::state::Mode::Edit;
                                state.create_dialog_open = false;
                                state.new_project_name.clear();
                                ui.close();
                            }
                        }
                    });
                });
                ui.add_space(12.0);
            });
        });
}
