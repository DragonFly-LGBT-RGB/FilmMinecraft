//! Audio Library panel (M5): three tabs — Library (built-in pack), Поиск в интернете
//! (Freesound / Pixabay), Мои звуки (downloaded / recorded / extracted).
//!
//! Every audio source becomes an ordinary `MediaAsset` and timeline clip via
//! `audioLibrary.search` / `audioLibrary.download` / `audioLibrary.addToTimeline`.

use egui::{Align2, Color32, Rect, Sense, pos2, vec2};
use serde_json::{Value, json};

use crate::FilmcraftApp;
use crate::theme::Tokens;

/// Active sub-tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
enum AudioTab {
    #[default]
    Library,
    Online,
    MySounds,
}

/// One preview result row (online search).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OnlineResult {
    provider_id: String,
    external_id: String,
    title: String,
    author: String,
    license: String,
    duration: f64,
    preview_url: Option<String>,
    download_url: Option<String>,
    tags: Vec<String>,
    cached: bool,
}

/// One built-in catalog item view (deserialized from engine `audioLibrary.catalog`).
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuiltinItemView {
    id: String,
    kind: String,
    title: String,
    category: String,
    path: String,
    duration: f64,
    tags: Vec<String>,
}

/// Panel state.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AudioLibraryState {
    tab: AudioTab,
    lib_query: String,
    lib_category: String,
    online_query: String,
    online_kind: String,
    online_license: String,
    online_providers: Vec<String>,
    online_results: Vec<OnlineResult>,
    online_loading: bool,
    online_error: Option<String>,
    my_query: String,
    preview_provider: String,
    preview_external: String,
    preview_playing: bool,
}

impl AudioLibraryState {
    fn tab_label(&self, t: AudioTab) -> &'static str {
        match t {
            AudioTab::Library => "Библиотека",
            AudioTab::Online => "Поиск в интернете",
            AudioTab::MySounds => "Мои звуки",
        }
    }
}

/// A command to run after layout.
type Actions = Vec<(String, Value)>;

pub fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    let painter = ui.painter().clone();
    painter.rect_filled(rect, 0.0, t.panel_bg);

    let pad = 12.0;
    let tab_h = 32.0;
    let tab_w = (rect.width() - 2.0 * pad) / 3.0;

    // ---- Tab bar ----
    // NOTE: the `&mut app.ui.audio_library` borrow lives only inside this block, so the
    // sub-tab functions below can take `app` mutably again.
    let tab = {
        let st = &mut app.ui.audio_library;
        let mut new_tab = None;
        for (i, tab) in [AudioTab::Library, AudioTab::Online, AudioTab::MySounds]
            .into_iter()
            .enumerate()
        {
            let x = rect.min.x + pad + i as f32 * tab_w;
            let r = Rect::from_min_size(pos2(x, rect.min.y + pad), vec2(tab_w - 2.0, tab_h));
            let active = st.tab == tab;
            let hovered = ui
                .interact(r, egui::Id::unique(("audio_tab", i as u32)), Sense::click())
                .hovered();
            let clicked = ui
                .interact(r, egui::Id::unique(("audio_tab_click", i as u32)), Sense::click())
                .clicked();
            painter.rect_filled(
                r,
                6.0,
                if active {
                    Color32::from_rgb(40, 44, 56)
                } else if hovered {
                    Color32::from_rgb(30, 34, 44)
                } else {
                    t.panel_bg
                },
            );
            if active {
                painter.rect_stroke(r, 6.0, egui::Stroke::new(2.0, t.accent), egui::StrokeKind::Inside);
            }
            let label = st.tab_label(tab);
            painter.text(
                r.center(),
                Align2::CENTER_CENTER,
                label,
                Tokens::semibold(13.0),
                if active { Color32::WHITE } else { t.text_dim },
            );
            app.auto.add(&format!("audioLibrary.tab.{}", i), r, label);
            if clicked {
                new_tab = Some(tab);
            }
        }
        if let Some(tab) = new_tab {
            st.tab = tab;
        }
        st.tab
    };

    let body = Rect::from_min_max(pos2(rect.min.x, rect.min.y + pad + tab_h + 10.0), rect.max);
    let mut actions: Actions = Vec::new();

    match tab {
        AudioTab::Library => library_tab(app, ui, body, &mut actions),
        AudioTab::Online => online_tab(app, ui, body, &mut actions),
        AudioTab::MySounds => my_sounds_tab(app, ui, body, &mut actions),
    }

    // Run pending actions.
    for (cmd, params) in actions {
        let _ = crate::menus::invoke(app, ui.ctx(), &cmd, params);
    }
}

/// ----------------------------------------------------------------------------------------------
/// Library tab — built-in pack
/// ----------------------------------------------------------------------------------------------

fn library_tab(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect, actions: &mut Actions) {
    let t = app.tokens;
    let pad = 10.0;
    let painter = ui.painter().clone();

    // Clone read values before any st borrow (avoid conflict with invoke below).
    let mut lib_query = app.ui.audio_library.lib_query.clone();
    let lib_category = app.ui.audio_library.lib_category.clone();
    let preview_provider = app.ui.audio_library.preview_provider.clone();
    let preview_external = app.ui.audio_library.preview_external.clone();
    let preview_playing = app.ui.audio_library.preview_playing;

    // Search bar.
    let search_r = Rect::from_min_size(rect.min + vec2(pad, pad), vec2(rect.width() - 2.0 * pad, 28.0));
    painter.rect_filled(search_r, 4.0, Color32::from_rgb(24, 28, 36));
    painter.rect_stroke(
        search_r,
        4.0,
        egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)),
        egui::StrokeKind::Inside,
    );
    painter.text(
        search_r.min + vec2(10.0, 14.0),
        Align2::LEFT_CENTER,
        &lib_query,
        Tokens::ui(12.0),
        Color32::from_rgb(200, 200, 210),
    );
    app.auto.add("audioLibrary.lib.search", search_r, "Search built-in sounds");
    let input_r = Rect::from_min_size(search_r.min + vec2(8.0, 2.0), vec2(search_r.width() - 16.0, 24.0));
    ui.put(input_r, egui::TextEdit::singleline(&mut lib_query));

    // Category chips.
    let fy = pad + 38.0;
    let cat_r = Rect::from_min_size(rect.min + vec2(pad, fy), vec2(rect.width() - 2.0 * pad, 26.0));
    let categories = ["Все", "Переходы", "Удары", "Райзеры", "Интерфейс", "Техника"];
    let mut cx = 0.0;
    let mut new_category: Option<String> = None;
    for cat in categories {
        let g = painter.layout_no_wrap(cat.to_string(), Tokens::ui(11.0), t.text);
        let chip_r = Rect::from_min_size(cat_r.min + vec2(cx, 2.0), vec2(g.size().x + 16.0, 22.0));
        let active = lib_category == cat;
        let hovered = ui
            .interact(chip_r, egui::Id::unique(("audio_cat", cat)), Sense::click())
            .hovered();
        let clicked = ui
            .interact(chip_r, egui::Id::unique(("audio_cat_click", cat)), Sense::click())
            .clicked();
        painter.rect_filled(
            chip_r,
            11.0,
            if active {
                t.accent
            } else if hovered {
                Color32::from_rgb(60, 60, 70)
            } else {
                Color32::from_rgb(40, 44, 56)
            },
        );
        painter.text(
            chip_r.center(),
            Align2::CENTER_CENTER,
            cat,
            Tokens::ui(11.0),
            if active { Color32::WHITE } else { t.text_dim },
        );
        app.auto.add(&format!("audioLibrary.cat.{cat}"), chip_r, cat);
        if clicked {
            new_category = Some(cat.to_string());
        }
        cx += g.size().x + 20.0;
    }

    // Invoke — no borrow of `app.ui` alive here, so this is fine.
    let catalog_res = crate::menus::invoke(app, ui.ctx(), "audioLibrary.catalog", json!({}));
    let items: Vec<BuiltinItemView> = catalog_res
        .ok()
        .and_then(|v| v.get("items").and_then(Value::as_array).cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|v| serde_json::from_value::<BuiltinItemView>(v.clone()).ok())
        .collect();

    // Write the edited UI values back into the state.
    {
        let st = &mut app.ui.audio_library;
        st.lib_query = lib_query;
        if let Some(cat) = new_category {
            st.lib_category = cat;
        }
    }

    let query_lower = app.ui.audio_library.lib_query.trim().to_lowercase();
    let category = app.ui.audio_library.lib_category.trim().to_string();

    let filtered: Vec<_> = items
        .into_iter()
        .filter(|it| {
            let mq = query_lower.is_empty()
                || it.title.to_lowercase().contains(&query_lower)
                || it.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower));
            let mc = category == "Все" || category.is_empty() || it.category == category;
            mq && mc
        })
        .collect();

    let top = fy + 32.0;
    let card_w = 170.0;
    let card_h = 92.0;
    let gap = 12.0;
    let cols = ((rect.width() - 2.0 * pad + gap) / (card_w + gap)).floor().max(1.0) as usize;
    let mut row = 0usize;
    let mut col = 0usize;

    for it in &filtered {
        let x = rect.min.x + pad + col as f32 * (card_w + gap);
        let y = top + row as f32 * (card_h + gap);
        let card_r = Rect::from_min_size(pos2(x, y), vec2(card_w, card_h));
        let hovered = ui
            .interact(card_r, egui::Id::unique(("audio_lib_item", it.id.as_str())), Sense::click())
            .hovered();
        let _clicked = ui
            .interact(
                card_r,
                egui::Id::unique(("audio_lib_item_click", it.id.as_str())),
                Sense::click(),
            )
            .clicked();
        painter.rect_filled(
            card_r,
            8.0,
            if hovered {
                Color32::from_rgb(36, 40, 50)
            } else {
                Color32::from_rgb(24, 28, 36)
            },
        );
        painter.rect_stroke(
            card_r,
            8.0,
            egui::Stroke::new(1.0, Color32::from_rgb(40, 44, 56)),
            egui::StrokeKind::Inside,
        );

        let kind_col = if it.kind == "sfx" {
            Color32::from_rgb(100, 200, 255)
        } else {
            Color32::from_rgb(255, 160, 100)
        };
        painter.text(card_r.min + vec2(10.0, 8.0), Align2::LEFT_CENTER, &it.kind, Tokens::ui(9.0), kind_col);
        painter.text(
            card_r.min + vec2(10.0, 26.0),
            Align2::LEFT_CENTER,
            &it.title,
            Tokens::semibold(12.0),
            Color32::WHITE,
        );
        painter.text(
            card_r.min + vec2(10.0, 42.0),
            Align2::LEFT_CENTER,
            &it.category,
            Tokens::ui(10.0),
            Color32::from_rgb(150, 150, 160),
        );
        painter.text(
            card_r.min + vec2(10.0, 56.0),
            Align2::LEFT_CENTER,
            &format!("{:.1} с  ·  {}", it.duration, it.tags.join(", ")),
            Tokens::ui(9.5),
            Color32::from_rgb(140, 140, 150),
        );

        // Play button.
        let play_r = Rect::from_min_size(card_r.max - vec2(30.0, 30.0), vec2(28.0, 28.0));
        let playing = preview_playing && preview_provider == "builtin" && preview_external == it.id;
        let play_hovered = ui
            .interact(play_r, egui::Id::unique(("audio_lib_play", it.id.as_str())), Sense::click())
            .hovered();
        let play_clicked = ui
            .interact(
                play_r,
                egui::Id::unique(("audio_lib_play_click", it.id.as_str())),
                Sense::click(),
            )
            .clicked();
        let play_col = if playing {
            Color32::from_rgb(255, 60, 60)
        } else {
            t.accent
        };
        painter.circle_filled(
            play_r.center(),
            10.0,
            if play_hovered { play_col.linear_multiply(1.2) } else { play_col },
        );
        let tri = [
            pos2(play_r.center().x - 4.0, play_r.center().y - 5.0),
            pos2(play_r.center().x - 4.0, play_r.center().y + 5.0),
            pos2(play_r.center().x + 5.0, play_r.center().y),
        ];
        painter.add(egui::Shape::convex_polygon(
            tri.to_vec(),
            Color32::WHITE,
            egui::Stroke::new(1.0, Color32::WHITE),
        ));
        app.auto.add(
            &format!("audioLibrary.lib.play.{}", it.id),
            play_r,
            &format!("Play {}", it.title),
        );
        // Only `app.ui.audio_library` is touched here (disjoint from `app.auto`), so this is fine.
        if play_clicked {
            let st = &mut app.ui.audio_library;
            if playing {
                st.preview_playing = false;
            } else {
                st.preview_playing = true;
                st.preview_provider = "builtin".into();
                st.preview_external = it.id.clone();
            }
        }

        // Add to timeline.
        let add_r = Rect::from_min_size(card_r.min + vec2(card_w - 70.0, card_h - 34.0), vec2(60.0, 24.0));
        let add_hovered = ui
            .interact(add_r, egui::Id::unique(("audio_lib_add", it.id.as_str())), Sense::click())
            .hovered();
        let add_clicked = ui
            .interact(
                add_r,
                egui::Id::unique(("audio_lib_add_click", it.id.as_str())),
                Sense::click(),
            )
            .clicked();
        painter.rect_filled(
            add_r,
            4.0,
            if add_hovered {
                Color32::from_rgb(60, 160, 100)
            } else {
                Color32::from_rgb(40, 120, 80)
            },
        );
        painter.text(
            add_r.center(),
            Align2::CENTER_CENTER,
            "+ на таймлайн",
            Tokens::ui(9.0),
            Color32::WHITE,
        );
        app.auto.add(
            &format!("audioLibrary.lib.add.{}", it.id),
            add_r,
            &format!("Add {} to timeline", it.title),
        );
        if add_clicked {
            let path = format!("assets/audio/builtin/{}", it.path);
            actions.push(("audioLibrary.addToTimeline".into(), json!({ "path": path })));
        }

        col += 1;
        if col >= cols {
            col = 0;
            row += 1;
        }
    }

    if filtered.is_empty() {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "Звуков не найдено",
            Tokens::ui(13.0),
            t.text_dim,
        );
    }
}

/// ----------------------------------------------------------------------------------------------
/// Online tab — Freesound / Pixabay search
/// ----------------------------------------------------------------------------------------------

fn online_tab(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect, actions: &mut Actions) {
    let t = app.tokens;
    let pad = 10.0;
    let painter = ui.painter().clone();

    let msg_y = pad + 38.0 + 28.0 + 34.0;
    let list_top = msg_y + 26.0;
    let list_rect = Rect::from_min_max(pos2(rect.min.x, list_top), rect.max);

    // ---- Header block: everything that needs `&mut app.ui.audio_library` lives here ----
    // The borrow must end before `menus::invoke(app, ..)`, otherwise the borrow checker
    // rejects the whole-`app` reborrow (E0499).
    let mut search_params: Option<Value> = None;
    {
        let st = &mut app.ui.audio_library;

        // Search bar.
        let search_r = Rect::from_min_size(rect.min + vec2(pad, pad), vec2(rect.width() - 2.0 * pad, 28.0));
        painter.rect_filled(search_r, 4.0, Color32::from_rgb(24, 28, 36));
        painter.rect_stroke(
            search_r,
            4.0,
            egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)),
            egui::StrokeKind::Inside,
        );
        painter.text(
            search_r.min + vec2(10.0, 14.0),
            Align2::LEFT_CENTER,
            &st.online_query,
            Tokens::ui(12.0),
            Color32::from_rgb(200, 200, 210),
        );
        app.auto.add("audioLibrary.online.search", search_r, "Search online audio");
        let input_r = Rect::from_min_size(search_r.min + vec2(8.0, 2.0), vec2(search_r.width() - 16.0, 24.0));
        ui.put(input_r, egui::TextEdit::singleline(&mut st.online_query));

        // Filters.
        let fy = pad + 38.0;
        let kind_label = match st.online_kind.as_str() {
            "music" => "Музыка",
            "sfx" => "Звуки",
            _ => "Всё",
        };
        let lic_label = match st.online_license.as_str() {
            "cc0" => "CC0",
            "cc-by" => "CC BY",
            _ => "Любая",
        };
        painter.text(
            pos2(rect.min.x + pad, fy),
            Align2::LEFT_CENTER,
            &format!("Тип: {}   Лицензия: {}", kind_label, lic_label),
            Tokens::ui(11.0),
            t.text_dim,
        );

        // Provider toggles.
        let py = fy + 28.0;
        let providers = [("freesound", "Freesound"), ("pixabay", "Pixabay")];
        let mut px = pad;
        for (pid, pname) in providers {
            let g = painter.layout_no_wrap(pname.to_string(), Tokens::ui(11.0), t.text);
            let toggle_r = Rect::from_min_size(pos2(rect.min.x + px, py), vec2(g.size().x + 30.0, 22.0));
            let enabled = st.online_providers.iter().any(|p| p == pid);
            let hovered = ui
                .interact(toggle_r, egui::Id::unique(("audio_prov", pid)), Sense::click())
                .hovered();
            let clicked = ui
                .interact(toggle_r, egui::Id::unique(("audio_prov_click", pid)), Sense::click())
                .clicked();
            painter.rect_filled(
                toggle_r,
                11.0,
                if enabled {
                    t.accent
                } else if hovered {
                    Color32::from_rgb(60, 60, 70)
                } else {
                    Color32::from_rgb(40, 44, 56)
                },
            );
            painter.text(
                toggle_r.center(),
                Align2::CENTER_CENTER,
                pname,
                Tokens::ui(11.0),
                if enabled { Color32::WHITE } else { t.text_dim },
            );
            app.auto.add(&format!("audioLibrary.provider.{pid}"), toggle_r, pname);
            if clicked {
                if enabled {
                    st.online_providers.retain(|p| p != pid);
                } else {
                    st.online_providers.push(pid.to_string());
                }
            }
            px += g.size().x + 34.0;
        }

        // Find button.
        let find_r = Rect::from_min_size(rect.max - vec2(pad + 100.0, pad), vec2(100.0, 28.0));
        let find_hovered = ui
            .interact(find_r, egui::Id::unique("audioLibrary.online.find"), Sense::click())
            .hovered();
        let find_clicked = ui
            .interact(find_r, egui::Id::unique("audioLibrary.online.find_click"), Sense::click())
            .clicked();
        painter.rect_filled(
            find_r,
            6.0,
            if find_hovered {
                Color32::from_rgb(60, 160, 255)
            } else {
                Color32::from_rgb(40, 120, 220)
            },
        );
        painter.text(
            find_r.center(),
            Align2::CENTER_CENTER,
            "Найти",
            Tokens::semibold(12.0),
            Color32::WHITE,
        );
        app.auto.add("audioLibrary.online.find", find_r, "Find audio");
        if find_clicked {
            st.online_loading = true;
            st.online_error = None;
            st.online_results.clear();
            do_online_search(st);
        }

        // Error / loading messages.
        if let Some(e) = &st.online_error {
            painter.text(
                pos2(rect.min.x + pad, msg_y),
                Align2::LEFT_CENTER,
                e,
                Tokens::ui(11.0),
                Color32::from_rgb(255, 100, 100),
            );
        } else if st.online_loading {
            painter.text(
                pos2(rect.min.x + pad, msg_y),
                Align2::LEFT_CENTER,
                "Идёт поиск…",
                Tokens::ui(11.0),
                t.text_dim,
            );
        }

        // Snapshot the search request while we still own the state.
        if st.online_loading {
            search_params = Some(json!({
                "text": st.online_query.clone(),
                "kind": st.online_kind.clone(),
                "license": st.online_license.clone(),
                "providers": st.online_providers.clone(),
            }));
        }
    }

    if search_params.is_none()
        && app.ui.audio_library.online_results.is_empty()
        && app.ui.audio_library.online_error.is_none()
    {
        painter.text(
            list_rect.center(),
            Align2::CENTER_CENTER,
            "Введите запрос и нажмите Найти",
            Tokens::ui(13.0),
            t.text_dim,
        );
        return;
    }

    // ---- Network step: no borrow of `app.ui` is alive here ----
    if let Some(params) = search_params {
        let res = crate::menus::invoke(app, ui.ctx(), "audioLibrary.search", params);
        let st = &mut app.ui.audio_library;
        st.online_loading = false;
        match res {
            Ok(Value::Object(map)) => {
                st.online_results = map
                    .get("results")
                    .and_then(Value::as_array)
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| serde_json::from_value::<OnlineResult>(v.clone()).ok())
                            .collect()
                    })
                    .unwrap_or_default();
            }
            Ok(_) => st.online_error = Some("Некорректный ответ сервера".into()),
            Err(_) => st.online_error = Some("Не удалось выполнить поиск".into()),
        }
    }

    // ---- Results ----
    // Local snapshot: `app.ui.audio_library` must stay free for the per-row writes below.
    let results: Vec<OnlineResult> = app.ui.audio_library.online_results.clone();
    let preview_playing = app.ui.audio_library.preview_playing;
    let preview_provider = app.ui.audio_library.preview_provider.clone();
    let preview_external = app.ui.audio_library.preview_external.clone();

    let item_h = 56.0;
    let gap = 6.0;
    let mut y = list_rect.min.y;
    for r in &results {
        let item_r = Rect::from_min_size(pos2(list_rect.min.x, y), vec2(list_rect.width(), item_h));
        let hovered = ui
            .interact(
                item_r,
                egui::Id::unique(("audio_online_result", r.external_id.as_str())),
                Sense::click(),
            )
            .hovered();
        let _clicked = ui
            .interact(
                item_r,
                egui::Id::unique(("audio_online_result_click", r.external_id.as_str())),
                Sense::click(),
            )
            .clicked();
        painter.rect_filled(
            item_r,
            4.0,
            if hovered {
                Color32::from_rgb(36, 40, 50)
            } else {
                Color32::from_rgb(24, 28, 36)
            },
        );

        let badge_col = if r.provider_id == "freesound" {
            Color32::from_rgb(100, 200, 255)
        } else {
            Color32::from_rgb(255, 160, 100)
        };
        painter.text(
            item_r.min + vec2(10.0, 8.0),
            Align2::LEFT_CENTER,
            &r.provider_id,
            Tokens::ui(9.0),
            badge_col,
        );
        painter.text(
            item_r.min + vec2(10.0, 24.0),
            Align2::LEFT_CENTER,
            &r.title,
            Tokens::semibold(12.0),
            Color32::WHITE,
        );
        painter.text(
            item_r.min + vec2(10.0, 40.0),
            Align2::LEFT_CENTER,
            &format!("{} · {:.0} с · {}", r.author, r.duration, r.license),
            Tokens::ui(10.0),
            Color32::from_rgb(150, 150, 160),
        );

        // Play / pause.
        let play_r = Rect::from_min_size(pos2(item_r.max.x - 250.0, item_r.center().y - 13.0), vec2(26.0, 26.0));
        let playing = preview_playing
            && preview_provider == r.provider_id
            && preview_external == r.external_id;
        let play_clicked = ui
            .interact(
                play_r,
                egui::Id::unique(("audio_online_play", r.external_id.as_str())),
                Sense::click(),
            )
            .clicked();
        let play_col = if playing {
            Color32::from_rgb(255, 60, 60)
        } else {
            t.accent
        };
        painter.circle_filled(play_r.center(), 9.0, play_col);
        let tri = [
            pos2(play_r.center().x - 3.0, play_r.center().y - 4.0),
            pos2(play_r.center().x - 3.0, play_r.center().y + 4.0),
            pos2(play_r.center().x + 4.0, play_r.center().y),
        ];
        painter.add(egui::Shape::convex_polygon(
            tri.to_vec(),
            Color32::WHITE,
            egui::Stroke::new(1.0, Color32::WHITE),
        ));
        app.auto.add(
            &format!("audioLibrary.online.play.{}", r.external_id),
            play_r,
            &format!("Play {}", r.title),
        );
        if play_clicked {
            let st = &mut app.ui.audio_library;
            if playing {
                st.preview_playing = false;
            } else {
                st.preview_playing = true;
                st.preview_provider = r.provider_id.clone();
                st.preview_external = r.external_id.clone();
            }
        }

        // Download button.
        let dl_r = Rect::from_min_size(pos2(item_r.max.x - 178.0, item_r.center().y - 12.0), vec2(64.0, 24.0));
        let dl_hovered = ui
            .interact(
                dl_r,
                egui::Id::unique(("audio_online_dl", r.external_id.as_str())),
                Sense::click(),
            )
            .hovered();
        let dl_clicked = ui
            .interact(
                dl_r,
                egui::Id::unique(("audio_online_dl_click", r.external_id.as_str())),
                Sense::click(),
            )
            .clicked();
        painter.rect_filled(
            dl_r,
            4.0,
            if dl_hovered {
                Color32::from_rgb(60, 160, 100)
            } else {
                Color32::from_rgb(40, 120, 80)
            },
        );
        painter.text(
            dl_r.center(),
            Align2::CENTER_CENTER,
            if r.cached { "В проекте" } else { "Скачать" },
            Tokens::ui(9.0),
            Color32::WHITE,
        );
        app.auto.add(
            &format!("audioLibrary.online.download.{}", r.external_id),
            dl_r,
            &format!("Download {}", r.title),
        );
        if dl_clicked {
            actions.push((
                "audioLibrary.download".into(),
                json!({
                    "providerId": r.provider_id,
                    "externalId": r.external_id,
                    "title": r.title,
                    "downloadUrl": r.download_url,
                    "previewUrl": r.preview_url,
                }),
            ));
        }

        // Add to timeline button.
        let add_r = Rect::from_min_size(pos2(item_r.max.x - 108.0, item_r.center().y - 12.0), vec2(64.0, 24.0));
        let add_hovered = ui
            .interact(
                add_r,
                egui::Id::unique(("audio_online_add", r.external_id.as_str())),
                Sense::click(),
            )
            .hovered();
        let add_clicked = ui
            .interact(
                add_r,
                egui::Id::unique(("audio_online_add_click", r.external_id.as_str())),
                Sense::click(),
            )
            .clicked();
        painter.rect_filled(
            add_r,
            4.0,
            if add_hovered {
                Color32::from_rgb(100, 200, 255)
            } else {
                Color32::from_rgb(80, 160, 220)
            },
        );
        painter.text(
            add_r.center(),
            Align2::CENTER_CENTER,
            "+ таймлайн",
            Tokens::ui(9.0),
            Color32::WHITE,
        );
        app.auto.add(
            &format!("audioLibrary.online.add.{}", r.external_id),
            add_r,
            &format!("Add {} to timeline", r.title),
        );
        if add_clicked {
            actions.push((
                "audioLibrary.download".into(),
                json!({
                    "providerId": r.provider_id,
                    "externalId": r.external_id,
                    "title": r.title,
                    "downloadUrl": r.download_url,
                    "previewUrl": r.preview_url,
                }),
            ));
            actions.push((
                "audioLibrary.addToTimeline".into(),
                json!({ "path": format!("media/audio/{}_{}.wav", r.title, r.external_id) }),
            ));
        }

        y += item_h + gap;
    }
}

/// ----------------------------------------------------------------------------------------------
/// My Sounds tab — local files
/// ----------------------------------------------------------------------------------------------

fn my_sounds_tab(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect, _actions: &mut Actions) {
    let t = app.tokens;
    let pad = 10.0;
    let painter = ui.painter().clone();

    // Search.
    {
        let st = &mut app.ui.audio_library;
        let search_r = Rect::from_min_size(rect.min + vec2(pad, pad), vec2(rect.width() - 2.0 * pad, 28.0));
        painter.rect_filled(search_r, 4.0, Color32::from_rgb(24, 28, 36));
        painter.rect_stroke(
            search_r,
            4.0,
            egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)),
            egui::StrokeKind::Inside,
        );
        painter.text(
            search_r.min + vec2(10.0, 14.0),
            Align2::LEFT_CENTER,
            &st.my_query,
            Tokens::ui(12.0),
            Color32::from_rgb(200, 200, 210),
        );
        app.auto.add("audioLibrary.my.search", search_r, "Search my sounds");
        let input_r = Rect::from_min_size(search_r.min + vec2(8.0, 2.0), vec2(search_r.width() - 16.0, 24.0));
        ui.put(input_r, egui::TextEdit::singleline(&mut st.my_query));
    }

    // List files from media/audio/ via services.
    let dir = "media/audio";
    let files: Vec<String> = if let Some(Ok(entries)) = app.session.services.list_dir(dir) {
        entries
            .into_iter()
            .filter(|n| n.ends_with(".wav") || n.ends_with(".mp3") || n.ends_with(".ogg"))
            .map(|n| format!("{dir}/{n}"))
            .collect()
    } else {
        Vec::new()
    };
    let query_lower = app.ui.audio_library.my_query.trim().to_lowercase();
    let filtered: Vec<_> = files
        .into_iter()
        .filter(|f| query_lower.is_empty() || f.to_lowercase().contains(&query_lower))
        .collect();

    let top = pad + 38.0;
    let item_h = 44.0;
    let gap = 4.0;
    let mut y = rect.min.y + top;
    for f in &filtered {
        let item_r = Rect::from_min_size(pos2(rect.min.x, y), vec2(rect.width() - 2.0 * pad, item_h));
        let hovered = ui
            .interact(item_r, egui::Id::unique(("audio_my_item", f.as_str())), Sense::click())
            .hovered();
        let _clicked = ui
            .interact(
                item_r,
                egui::Id::unique(("audio_my_item_click", f.as_str())),
                Sense::click(),
            )
            .clicked();
        painter.rect_filled(
            item_r,
            4.0,
            if hovered {
                Color32::from_rgb(36, 40, 50)
            } else {
                Color32::from_rgb(24, 28, 36)
            },
        );

        let name = std::path::Path::new(f)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| f.clone());
        painter.text(
            item_r.min + vec2(10.0, item_h / 2.0),
            Align2::LEFT_CENTER,
            &name,
            Tokens::ui(12.0),
            Color32::WHITE,
        );
        app.auto.add(&format!("audioLibrary.my.item.{}", f), item_r, f);

        // Add button.
        let add_r = Rect::from_min_size(pos2(item_r.max.x - 90.0, item_r.center().y - 12.0), vec2(64.0, 24.0));
        let add_hovered = ui
            .interact(add_r, egui::Id::unique(("audio_my_add", f.as_str())), Sense::click())
            .hovered();
        let add_clicked = ui
            .interact(
                add_r,
                egui::Id::unique(("audio_my_add_click", f.as_str())),
                Sense::click(),
            )
            .clicked();
        painter.rect_filled(
            add_r,
            4.0,
            if add_hovered {
                Color32::from_rgb(60, 160, 100)
            } else {
                Color32::from_rgb(40, 120, 80)
            },
        );
        painter.text(
            add_r.center(),
            Align2::CENTER_CENTER,
            "+ таймлайн",
            Tokens::ui(9.0),
            Color32::WHITE,
        );
        app.auto.add(
            &format!("audioLibrary.my.add.{}", f),
            add_r,
            &format!("Add {} to timeline", name),
        );
        if add_clicked {
            let _ = crate::menus::invoke(app, ui.ctx(), "audioLibrary.addToTimeline", json!({ "path": f }));
        }

        y += item_h + gap;
    }

    if filtered.is_empty() {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "Нет локальных звуков",
            Tokens::ui(13.0),
            t.text_dim,
        );
    }
}

/// ----------------------------------------------------------------------------------------------
/// Helpers
/// ----------------------------------------------------------------------------------------------

fn do_online_search(st: &mut AudioLibraryState) {
    let _ = st;
}
