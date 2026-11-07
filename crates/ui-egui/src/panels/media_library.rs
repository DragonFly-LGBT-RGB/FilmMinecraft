//! Media Library panel (M3): the project's managed media registry.
//!
//! A grid of tiles (video / audio / stills) with search, filters, sorting,
//! background preview generation and "add to timeline" actions.

use std::collections::HashSet;

use egui::{Align2, Color32, Rect, Sense, Stroke, StrokeKind, pos2, vec2};
use filmcraft_project::{AssetKind, MediaAsset, MediaAssetId};
use serde_json::json;

use crate::FilmcraftApp;
use crate::icons::{self, Icon};
use crate::theme::Tokens;

/// Frontend UI state for the Media Library panel.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MediaLibraryUi {
    /// Current search text.
    pub search: String,
    /// `all` | `video` | `audio` | `image`.
    pub filter: String,
    /// `name` | `date` | `duration`.
    pub sort: String,
    /// Tile size: `S` | `M` | `L`.
    pub tile_size: String,
    /// Selected asset ids.
    pub selection: HashSet<MediaAssetId>,
    /// Whether "add all" confirmation is shown.
    pub show_add_all: bool,
}

impl MediaLibraryUi {
    fn tile_size_px(&self) -> f32 {
        match self.tile_size.as_str() {
            "S" => 120.0,
            "L" => 240.0,
            _ => 160.0,
        }
    }
}

/// Number of filmstrip frames per asset thumbnail.
const FILMSTRIP_FRAMES: usize = 8;

/// Draw the Media Library panel.
pub fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    let ctx = ui.ctx().clone();

    // ---- extract all needed data before closures to avoid borrow conflicts
    let filter = app.ui.media_library.filter.clone();
    let search = app.ui.media_library.search.clone();
    let sort = app.ui.media_library.sort.clone();
    let tile_size = app.ui.media_library.tile_size.clone();
    let selection = app.ui.media_library.selection.clone();
    let show_add_all = app.ui.media_library.show_add_all;

    let assets: Vec<MediaAsset> = app
        .session
        .project
        .media
        .iter()
        .filter(|a| match filter.as_str() {
            "video" => a.kind == AssetKind::Video,
            "audio" => a.kind == AssetKind::Audio,
            "image" => a.kind == AssetKind::Image,
            _ => true,
        })
        .filter(|a| {
            if search.is_empty() {
                true
            } else {
                let s = search.to_lowercase();
                a.name.to_lowercase().contains(&s) || a.path.to_lowercase().contains(&s)
            }
        })
        .cloned()
        .collect::<Vec<_>>();

    // sort
    let mut assets = assets;
    assets.sort_by(|a, b| match sort.as_str() {
        "name" => a.name.cmp(&b.name),
        "duration" => a.duration.partial_cmp(&b.duration).unwrap_or(std::cmp::Ordering::Equal),
        _ => b.created_at.cmp(&a.created_at),
    });

    let tile = match tile_size.as_str() {
        "S" => 120.0,
        "L" => 240.0,
        _ => 160.0,
    };
    let tile_h = tile * 9.0 / 16.0;
    let cell = vec2(tile + 16.0, tile_h + 46.0);
    let per_row = ((rect.width() - 8.0) / cell.x).floor().max(1.0) as usize;
    let asset_count = assets.len();
    let now = app.now();
    let generation: Vec<(MediaAssetId, u8)> = app
        .ui
        .media_library_generation
        .iter()
        .map(|(k, v)| (*k, *v))
        .collect();

    // ---- toolbar
    let bar = Rect::from_min_size(rect.min + vec2(6.0, 4.0), vec2(rect.width() - 12.0, 28.0));
    let mut bar_child = ui.new_child(egui::UiBuilder::new().max_rect(bar).id_salt("medialib-bar"));
    bar_child.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 4.0);
        let mut search_text = search.clone();
        let sr = ui.add(
            egui::TextEdit::singleline(&mut search_text)
                .hint_text(crate::i18n::t("Search media…"))
                .desired_width(200.0),
        );
        app.auto.add("medialib.search", sr.rect, "Search media");
        if sr.changed() {
            app.ui.media_library.search = search_text;
        }
        ui.separator();
        for (label, value) in [
            (crate::i18n::t("All"), "all"),
            (crate::i18n::t("Video"), "video"),
            (crate::i18n::t("Audio"), "audio"),
            (crate::i18n::t("Image"), "image"),
        ] {
            let sel = filter == value;
            let b = ui.selectable_label(sel, label);
            app.auto.add(&format!("medialib.filter.{value}"), b.rect, label);
            if b.clicked() {
                app.ui.media_library.filter = value.to_string();
            }
        }
        ui.separator();
        egui::ComboBox::from_id_salt("medialib.sort")
            .selected_text(match sort.as_str() {
                "name" => crate::i18n::t("Name"),
                "date" => crate::i18n::t("Date added"),
                _ => crate::i18n::t("Duration"),
            })
            .show_ui(ui, |ui| {
                for (label, value) in [
                    (crate::i18n::t("Name"), "name"),
                    (crate::i18n::t("Date added"), "date"),
                    (crate::i18n::t("Duration"), "duration"),
                ] {
                    if ui.selectable_label(sort == value, label).clicked() {
                        app.ui.media_library.sort = value.to_string();
                    }
                }
            });
        ui.separator();
        for (label, value) in [("S", "S"), ("M", "M"), ("L", "L")] {
            let sel = tile_size == value;
            let b = ui.selectable_label(sel, label);
            app.auto.add(&format!("medialib.size.{value}"), b.rect, label);
            if b.clicked() {
                app.ui.media_library.tile_size = value.to_string();
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let has_sel = !selection.is_empty();
            if has_sel {
                let b = ui.button(crate::i18n::t("Add selected to timeline"));
                app.auto.add("medialib.addSelected", b.rect, "Add selected to timeline");
                if b.clicked() {
                    let ids: Vec<u64> = selection.iter().map(|i| i.0).collect();
                    let _ = app.session.execute("media.addToTimeline", json!({ "assets": ids }));
                    app.ui.media_library.selection.clear();
                }
            }
            if asset_count > 0 {
                let b = ui.button(crate::i18n::t("Add all to timeline"));
                app.auto.add("medialib.addAll", b.rect, "Add all to timeline");
                if b.clicked() {
                    app.ui.media_library.show_add_all = true;
                }
            }
        });
    });

    // ---- add-all confirmation popup
    if show_add_all {
        let ids: Vec<u64> = assets.iter().map(|a| a.id.0).collect();
        egui::Window::new(crate::i18n::t("Add all to timeline?"))
            .id(egui::Id::unique("medialib-add-all"))
            .collapsible(false)
            .pivot(Align2::CENTER_CENTER)
            .default_pos(crate::widgets::dialog_center(&ctx))
            .show(&ctx, |ui| {
                ui.label(crate::i18n::t("Add N assets to the end of the sequence?").replace("N", &asset_count.to_string()));
                ui.horizontal(|ui| {
                    if ui.button(crate::i18n::t("Cancel")).clicked() {
                        app.ui.media_library.show_add_all = false;
                    }
                    if ui.button(crate::i18n::t("Add All")).clicked() {
                        let _ = app.session.execute(
                            "media.addToTimeline",
                            json!({ "assets": ids }),
                        );
                        app.ui.media_library.show_add_all = false;
                        app.ui.media_library.selection.clear();
                    }
                });
            });
    }

    // ---- body: grid of tiles
    let bar_bottom = rect.min.y + 36.0;
    let body = Rect::from_min_max(
        pos2(rect.min.x, bar_bottom + 8.0),
        pos2(rect.max.x, rect.max.y - 4.0),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body).id_salt("medialib-body"));
    child.set_clip_rect(body.intersect(ui.clip_rect()));

    if assets.is_empty() {
        empty_state(&mut child, &t);
        return;
    }

    let mut gen_count = 0u64;
    egui::ScrollArea::vertical()
        .id_salt("medialib-scroll")
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            for chunk in assets.chunks(per_row) {
                let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), cell.y), Sense::hover());
                for (k, asset) in chunk.iter().enumerate() {
                    let x = row.min.x + 8.0 + k as f32 * cell.x;
                    let tr = Rect::from_min_size(pos2(x, row.min.y + 6.0), vec2(tile, tile_h));
                    let name_r = Rect::from_min_size(
                        pos2(x, row.min.y + tile_h + 6.0),
                        vec2(tile, 18.0),
                    );
                    let selected = selection.contains(&asset.id);
                    let resp = ui.interact(
                        tr.expand(4.0),
                        egui::Id::unique(("medialib_tile", asset.id.0)),
                        Sense::click_and_drag(),
                    );

                    // selection
                    if resp.clicked() {
                        if ui.input(|i| i.modifiers.command) {
                            if selection.contains(&asset.id) {
                                app.ui.media_library.selection.remove(&asset.id);
                            } else {
                                app.ui.media_library.selection.insert(asset.id);
                            }
                        } else {
                            app.ui.media_library.selection.clear();
                            app.ui.media_library.selection.insert(asset.id);
                        }
                    }
                    if resp.double_clicked() {
                        let _ = app.session.execute(
                            "media.addToTimeline",
                            json!({ "assets": [asset.id.0] }),
                        );
                    }

                    // tile background
                    ui.painter().rect_filled(tr, 4.0, Color32::from_gray(14));
                    if selected {
                        ui.painter()
                            .rect_stroke(tr, 4.0, Stroke::new(2.0, t.accent), StrokeKind::Outside);
                    }
                    if asset.missing {
                        ui.painter()
                            .rect_stroke(tr, 4.0, Stroke::new(2.0, Color32::from_rgb(255, 60, 60)), StrokeKind::Outside);
                    }
                    // hover overlay
                    if resp.hovered() {
                        ui.painter().rect_filled(tr, 4.0, Color32::from_black_alpha(60));
                        // add-to-timeline button on hover
                        let btn = Rect::from_center_size(tr.center(), vec2(28.0, 28.0));
                        let btn_resp = ui.interact(btn, egui::Id::unique(("medialib_add", asset.id.0)), Sense::click());
                        if btn_resp.clicked() {
                            let _ = app.session.execute(
                                "media.addToTimeline",
                                json!({ "assets": [asset.id.0] }),
                            );
                            app.ui.media_library.selection.clear();
                        }
                        ui.painter().text(
                            btn.center(),
                            Align2::CENTER_CENTER,
                            "+",
                            egui::FontId::default(),
                            Color32::WHITE,
                        );
                        // overflow menu
                        let mbtn = Rect::from_min_size(tr.right_top() + vec2(-26.0, 4.0), vec2(20.0, 20.0));
                        let mresp = ui.interact(mbtn, egui::Id::unique(("medialib_menu", asset.id.0)), Sense::click());
                        if mresp.clicked() {
                            app.ui.media_library_asset = Some(asset.id);
                        }
                        ui.painter().text(
                            mbtn.center(),
                            Align2::CENTER_CENTER,
                            "⋮",
                            egui::FontId::default(),
                            Color32::from_rgb(180, 180, 190),
                        );
                    }

                    // thumbnail content
                    let drawn = draw_thumbnail(app, ui, &ctx, asset, tr);
                    if !drawn {
                        let icon = match asset.kind {
                            AssetKind::Video => Icon::Film,
                            AssetKind::Audio => Icon::Audio,
                            AssetKind::Image => Icon::Image,
                            AssetKind::Unknown => Icon::Sequence,
                        };
                        icons::paint(
                            ui.painter(),
                            Rect::from_center_size(tr.center(), vec2(34.0, 34.0)),
                            icon,
                            t.text_faint,
                        );
                    }
                    // duration badge
                    if asset.duration > 0.0 {
                        let secs = asset.duration;
                        let mm = (secs / 60.0) as i64;
                        let ss = (secs % 60.0) as i64;
                        let badge = format!("{mm}:{ss:02}");
                        let br = Rect::from_min_size(tr.right_bottom() + vec2(-26.0, -12.0), vec2(24.0, 10.0));
                        ui.painter().rect_filled(br, 2.0, Color32::from_black_alpha(150));
                        ui.painter().text(
                            br.center(),
                            Align2::CENTER_CENTER,
                            &badge,
                            Tokens::ui(8.0),
                            Color32::WHITE,
                        );
                    }
                    // generation spinner
                    if let Some((_, progress)) = generation.iter().find(|(id, _)| *id == asset.id) {
                        gen_count += 1;
                        let br = Rect::from_min_size(tr.left_top() + vec2(4.0, 4.0), vec2(12.0, 12.0));
                        let angle = (now * 4.0) % std::f64::consts::TAU;
                        let center = br.center();
                        let radius = 5.0;
                        let mut points = Vec::new();
                        let steps = 12;
                        for i in 0..=steps {
                            let a = angle as f32 + (i as f32 / steps as f32) * std::f32::consts::PI;
                            points.push(pos2(center.x + radius * a.cos(), center.y + radius * a.sin()));
                        }
                        for w in points.windows(2) {
                            ui.painter().line_segment([w[0], w[1]], Stroke::new(2.0, Color32::from_rgb(0, 180, 216)));
                        }
                        ui.painter().text(
                            pos2(br.max.x + 2.0, br.center().y),
                            Align2::LEFT_CENTER,
                            &format!("{progress}%"),
                            Tokens::ui(8.0),
                            t.text_dim,
                        );
                    }

                    // name
                    ui.painter().with_clip_rect(name_r.intersect(ui.clip_rect())).text(
                        name_r.min,
                        Align2::LEFT_TOP,
                        &asset.name,
                        Tokens::ui(10.5),
                        if selected { t.text } else { t.text_dim },
                    );
                    // kind + size
                    let kind_label = match asset.kind {
                        AssetKind::Video => crate::i18n::t("Video"),
                        AssetKind::Audio => crate::i18n::t("Audio"),
                        AssetKind::Image => crate::i18n::t("Image"),
                        AssetKind::Unknown => crate::i18n::t("Media"),
                    };
                    let size_kb = asset.size_bytes / 1024;
                    ui.painter().with_clip_rect(name_r.intersect(ui.clip_rect())).text(
                        pos2(name_r.min.x, name_r.min.y + 11.0),
                        Align2::LEFT_TOP,
                        &format!("{kind_label} · {size_kb} KB"),
                        Tokens::ui(8.5),
                        t.text_faint,
                    );
                    app.auto.add(&format!("medialib.tile.{}", asset.id.0), tr.expand(4.0), &asset.name);
                }
            }
            if gen_count > 0 {
                ui.painter().text(
                    pos2(body.min.x + 6.0, body.max.y - 12.0),
                    Align2::LEFT_CENTER,
                    &crate::i18n::t("Generating previews N").replace("N", &gen_count.to_string()),
                    Tokens::ui(10.0),
                    t.text_dim,
                );
            }
        });

    // ---- overflow menu for the right-clicked asset
    media_library_menu(app, &ctx);
}

/// Empty state: drop zone with format hints.
fn empty_state(ui: &mut egui::Ui, t: &Tokens) {
    let r = ui.available_rect_before_wrap();
    let center = r.center();
    ui.painter().text(
        pos2(center.x, center.y - 20.0),
        Align2::CENTER_CENTER,
        crate::i18n::t("Media Library"),
        Tokens::ui(20.0),
        t.text,
    );
    ui.painter().text(
        pos2(center.x, center.y + 6.0),
        Align2::CENTER_CENTER,
        crate::i18n::t("Import media to build your project library. Files are copied into the project's media/ folder."),
        Tokens::ui(11.0),
        t.text_dim,
    );
    ui.painter().text(
        pos2(center.x, center.y + 22.0),
        Align2::CENTER_CENTER,
        crate::i18n::t("mp4 · mov · mp3 · wav · jpg · png · …"),
        Tokens::ui(9.0),
        t.text_faint,
    );
    let drop = Rect::from_center_size(center, vec2(200.0, 40.0));
    let _resp = ui.interact(drop, egui::Id::unique("medialib_drop"), Sense::click());
    ui.painter().rect_stroke(drop, 4.0, Stroke::new(1.0, t.separator), StrokeKind::Inside);
    ui.painter().text(
        drop.center(),
        Align2::CENTER_CENTER,
        crate::i18n::t("Import…"),
        Tokens::ui(11.0),
        t.text,
    );
}

/// Draw a thumbnail for an asset (from cache or frame server).
fn draw_thumbnail(_app: &FilmcraftApp, _ui: &egui::Ui, _ctx: &egui::Context, _asset: &MediaAsset, _r: Rect) -> bool {
    // Thumbnail rendering from cache/textures is wired up when the media pipeline is ready.
    // For now, fall back to type icons.
    false
}

/// Overflow menu for a right-clicked / hover-menu asset.
fn media_library_menu(app: &mut FilmcraftApp, ctx: &egui::Context) {
    let Some(asset_id) = app.ui.media_library_asset else { return };
    let Some(asset) = app.session.project.media.get(asset_id) else {
        app.ui.media_library_asset = None;
        return;
    };
    let asset_path = asset.path.clone();
    let pos = ctx.pointer_hover_pos().unwrap_or_default();
    egui::Area::new(egui::Id::unique("medialib-menu-area"))
        .order(egui::Order::Tooltip)
        .fixed_pos(pos + vec2(8.0, 8.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                if ui.button(crate::i18n::t("Add to timeline")).clicked() {
                    let _ = app.session.execute(
                        "media.addToTimeline",
                        json!({ "assets": [asset_id.0] }),
                    );
                    app.ui.media_library_asset = None;
                }
                if ui.button(crate::i18n::t("Remove from library")).clicked() {
                    let _ = app.session.execute(
                        "media.remove",
                        json!({ "assets": [asset_id.0], "deleteFiles": false }),
                    );
                    app.ui.media_library_asset = None;
                }
                if ui.button(crate::i18n::t("Reveal in folder")).clicked() {
                    let _ = crate::panels::menu_dialogs::open_path(app, ctx, &asset_path, false);
                    app.ui.media_library_asset = None;
                }
                ui.separator();
                if ui.button(crate::i18n::t("Cancel")).clicked() {
                    app.ui.media_library_asset = None;
                }
            });
        });
}

/// Panel ≡ menu.
pub fn panel_menu(_app: &mut FilmcraftApp, _ui: &mut egui::Ui) -> bool {
    false
}
