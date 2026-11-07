//! Media Library commands (M3): the project's managed media registry.
//!
//! Commands:
//! - `media.import` — copy files into the project's `media/` folder, de-duplicate by fingerprint,
//!   create `MediaAsset` + `ProjectItem` entries, return the new asset ids.
//! - `media.addToTimeline` — append one or more assets to a sequence as track items.
//! - `media.remove` — remove an asset (with warnings when it is in use).
//! - `media.list` — list the project's media assets with optional filter / search.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;

use filmcraft_media::{is_importable, open_bytes};
use filmcraft_project::{
    AssetKind, MediaAsset, MediaAssetId, MediaClip, MediaIdentity, MediaLibrary, MediaRef,
    TrackItem,
};
use filmcraft_time::{FrameRate, Tick, TimeRange, TICKS_PER_SECOND};
use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

/// Max file size (bytes) before offering "link to original" instead of copy.
const LARGE_FILE_THRESHOLD: u64 = 200 * 1024 * 1024;

/// `media.import {paths:[str], copy?:bool, bin?:binId}`: import files into the project's media
/// library. Files are copied into `media/`; when a file exceeds `LARGE_FILE_THRESHOLD` and
/// `copy` is false, the original path is referenced instead (a "linked" asset).
pub fn import(s: &mut Session, p: &Value) -> Result<Value> {
    let paths: Vec<String> = match p.get("paths").and_then(Value::as_array) {
        Some(a) => a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        None => str_p(p, "path").map(|x| vec![x.to_string()]).unwrap_or_default(),
    };
    if paths.is_empty() {
        return Err(bad("media.import", "need `paths`"));
    }
    let copy = p.get("copy").and_then(Value::as_bool).unwrap_or(true);
    let bin = u64_p(p, "bin").map(filmcraft_project::BinId);

    let project_dir = s
        .path
        .as_deref()
        .and_then(|p| std::path::Path::new(p).parent())
        .map(|d| d.to_string_lossy().to_string())
        .unwrap_or_default();
    let media_dir = if project_dir.is_empty() {
        "media".to_string()
    } else {
        format!("{project_dir}/media")
    };

    // ensure media/ exists
    let _ = std::fs::create_dir_all(std::path::Path::new(&media_dir));

    let mut ids: Vec<u64> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut created: Vec<Value> = Vec::new();

    for path in &paths {
        if !is_importable(std::path::Path::new(path)) {
            errors.push(format!("{path}: unsupported format"));
            continue;
        }

        let size = s
            .services
            .file_size(path)
            .map_err(|e| EngineError::Other(format!("{path}: {e}")))?;

        // Read enough of the file for a fingerprint (size + first/last 1 MiB).
        let fingerprint = fingerprint_hash(path, size);

        // De-duplication: same fingerprint = same asset already in the library.
        if let Some(fp) = fingerprint
            && let Some(existing) = s.project.media.by_fingerprint(fp)
        {
            ids.push(existing.id.0);
            created.push(json!({
                "id": existing.id.0,
                "name": existing.name,
                "path": existing.path,
                "deduplicated": true,
            }));
            continue;
        }

        // Read the file for media probing.
        let bytes = s
            .services
            .read_file(path)
            .map_err(|e| EngineError::Other(format!("{path}: {e}")))?;
        let name = std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();
        let src = open_bytes(&name, bytes.into(), &[])
            .map_err(|e| EngineError::Other(format!("{path}: {e}")))?;
        let info = src.info().clone();

        // Decide: copy into media/ or link to the original.
        let (should_copy, managed_path) = if copy && size < LARGE_FILE_THRESHOLD {
            let dest = unique_media_path(&media_dir, &name);
            let file_bytes = s
                .services
                .read_file(path)
                .map_err(|e| EngineError::Other(format!("{path}: {e}")))?;
            s.services
                .write_file(&dest, &file_bytes)
                .map_err(|e| EngineError::Other(format!("{dest}: {e}")))?;
            (true, dest)
        } else {
            (false, path.clone())
        };

        let kind = asset_kind(path);
        let duration_secs = info.duration.seconds();
        let fps = info.frame_rate().num as f64 / info.frame_rate().den as f64;

        let asset = MediaAsset {
            id: MediaAssetId(0),
            name: name.clone(),
            path: managed_path.clone(),
            kind,
            duration: duration_secs,
            width: info.video.as_ref().map_or(0, |v| v.width),
            height: info.video.as_ref().map_or(0, |v| v.height),
            fps,
            size_bytes: size,
            has_audio: info.has_audio(),
            codec: info
                .video
                .as_ref()
                .map(|v| v.codec.clone())
                .or_else(|| info.audio().map(|a| a.codec.clone())),
            thumbnail_path: None,
            filmstrip_path: None,
            waveform_path: None,
            proxy_path: None,
            created_at: now_seconds(),
            missing: false,
            fingerprint,
        };

        let asset_name = asset.name.clone();
        let asset_path = asset.path.clone();
        let asset_kind_label = format!("{:?}", asset.kind).to_lowercase();
        let asset_duration = asset.duration;

        let id = s.edit(&format!("Import {name}"), |pr, _| {
            let asset_id = pr.media.alloc_id();
            let mut asset = asset;
            asset.id = asset_id;
            pr.media.insert(asset);

            let item_id = pr.add_item(
                &name,
                filmcraft_project::Label::Iris,
                filmcraft_project::ItemKind::Media(Box::new(MediaClip {
                    media: MediaRef::File { path: managed_path.clone() },
                    info,
                    interpret: Default::default(),
                    mark_in: None,
                    mark_out: None,
                    markers: vec![],
                    offline: false,
                    proxy: None,
                    identity: fingerprint.map(|f| MediaIdentity { size, fingerprint: f }),
                })),
                bin,
            );
            Ok((asset_id, item_id))
        })?;

        let (asset_id, item_id) = id;
        ids.push(asset_id.0);
        created.push(json!({
            "id": asset_id.0,
            "item": item_id.0,
            "name": asset_name,
            "path": asset_path,
            "kind": asset_kind_label,
            "duration": asset_duration,
            "deduplicated": false,
        }));
    }

    if ids.is_empty() && !errors.is_empty() {
        return Err(EngineError::Other(errors.join("; ")));
    }

    Ok(json!({
        "assets": ids,
        "items": created,
        "errors": errors,
    }))
}

/// `media.addToTimeline {assets:[id], track?:str, start?:ticks}`: append assets to a sequence.
pub fn add_to_timeline(s: &mut Session, p: &Value) -> Result<Value> {
    let asset_ids: Vec<u64> = match p.get("assets").and_then(Value::as_array) {
        Some(a) => a.iter().filter_map(Value::as_u64).collect(),
        None => return Err(bad("media.addToTimeline", "need `assets`")),
    };
    if asset_ids.is_empty() {
        return Err(bad("media.addToTimeline", "`assets` is empty"));
    }
    let active_seq = s.active_sequence().ok_or_else(|| bad("media.addToTimeline", "no sequence open"))?;
    let track_kind = match p.get("track").and_then(Value::as_str) {
        Some("A1" | "A2" | "A3" | "A4") => filmcraft_project::TrackKind::Audio,
        _ => filmcraft_project::TrackKind::Video,
    };
    let start = p.get("start").and_then(Value::as_i64).map(Tick).unwrap_or(active_seq.duration());
    let rate = active_seq.settings.frame_rate;

    // find the ProjectItem id for the active sequence
    let seq_item_id = s
        .state
        .active_sequence
        .ok_or_else(|| bad("media.addToTimeline", "no active sequence item"))?;

    let mut added: Vec<u64> = Vec::new();
    let mut cursor = start;
    for asset_id in asset_ids {
        let aid = MediaAssetId(asset_id);
        let Some(asset) = s.project.media.get(aid) else {
            continue;
        };
        // find the ProjectItem for this asset's managed path
        let Some(item_id) = s
            .project
            .items
            .iter()
            .find_map(|(id, it)| match &it.kind {
                filmcraft_project::ItemKind::Media(m) if matches!(&m.media, MediaRef::File { path } if *path == asset.path) => Some(*id),
                _ => None,
            })
        else {
            continue;
        };

        let dur = if asset.duration > 0.0 {
            Tick::from_seconds_f64(asset.duration)
        } else {
            rate.frame_duration()
        };
        let dur = dur.max(rate.frame_duration());

        s.edit("Add to Timeline", |pr, _| {
            let Some(clip) = pr.make_track_item(item_id, track_kind, cursor, TimeRange::new(Tick::ZERO, dur), rate) else {
                return Err(EngineError::Other("cannot make track item".into()));
            };
            let clip = TrackItem {
                media_id: Some(aid),
                ..clip
            };
            let track = match track_kind {
                filmcraft_project::TrackKind::Video => {
                    pr.sequence_mut(seq_item_id).and_then(|q| q.video_tracks.first_mut())
                }
                filmcraft_project::TrackKind::Audio => {
                    pr.sequence_mut(seq_item_id).and_then(|q| q.audio_tracks.first_mut())
                }
            };
            let Some(track) = track else { return Err(EngineError::Other("no track".into())) };
            track.items.push(clip);
            Ok(())
        })?;

        added.push(aid.0);
        cursor = cursor + dur;
    }

    Ok(json!({
        "clips": added,
        "end": cursor.0,
    }))
}

/// `media.remove {assets:[id], deleteFiles?:bool, replaceWith?:id}`: remove assets from the
/// library. When an asset is used by clips, either replace those clips' source or fail.
pub fn remove(s: &mut Session, p: &Value) -> Result<Value> {
    let asset_ids: Vec<u64> = match p.get("assets").and_then(Value::as_array) {
        Some(a) => a.iter().filter_map(Value::as_u64).collect(),
        None => return Err(bad("media.remove", "need `assets`")),
    };
    let delete_files = p.get("deleteFiles").and_then(Value::as_bool).unwrap_or(false);
    let replace_with = u64_p(p, "replaceWith");

    // find which clips use each asset
    let mut used_by: Vec<(MediaAssetId, Vec<filmcraft_project::ClipId>)> = Vec::new();
    for &aid in &asset_ids {
        let id = MediaAssetId(aid);
        let clips: Vec<filmcraft_project::ClipId> = s
            .project
            .items
            .values()
            .filter_map(|it| it.as_sequence())
            .flat_map(|q| {
                q.all_tracks()
                    .flat_map(|t| t.items.iter())
                    .filter(|c| c.media_id == Some(id))
                    .map(|c| c.id)
                    .collect::<Vec<_>>()
            })
            .collect();
        if !clips.is_empty() {
            used_by.push((id, clips));
        }
    }

    if !used_by.is_empty() && replace_with.is_none() {
        let detail: Vec<Value> = used_by
            .iter()
            .map(|(aid, clips)| {
                json!({ "asset": aid.0, "clips": clips.iter().map(|c| c.0).collect::<Vec<_>>() })
            })
            .collect();
        return Err(bad(
            "media.remove",
            format!(
                "asset(s) in use by {} clip(s); pass `replaceWith` to repoint them",
                detail.len()
            ),
        ));
    }

    // repoint clips to replacement asset
    if let Some(replace_id) = replace_with {
        let replace = MediaAssetId(replace_id);
        for (_aid, clips) in &used_by {
            for clip_id in clips {
                s.edit("Replace Media", |pr, _| {
                    for it in pr.items.values_mut() {
                        if let Some(q) = it.as_sequence_mut() {
                            for t in q.all_tracks_mut() {
                                for c in t.items.iter_mut() {
                                    if c.id == *clip_id {
                                        c.media_id = Some(replace);
                                    }
                                }
                            }
                        }
                    }
                    Ok(())
                })?;
            }
        }
    }

    let mut removed: Vec<u64> = Vec::new();
    for &aid in &asset_ids {
        let id = MediaAssetId(aid);
        if let Some(asset) = s.project.media.get(id) {
            if delete_files {
                let _ = std::fs::remove_file(&asset.path);
            }
        }
        s.edit("Remove Media", |pr, _| {
            pr.media.remove(id);
            Ok(())
        })?;
        removed.push(aid);
    }

    Ok(json!({ "removed": removed }))
}

/// `media.list {filter:"video|audio|image|all", search?:str}`: list media assets.
pub fn list(s: &mut Session, p: &Value) -> Result<Value> {
    let filter = str_p(p, "filter").unwrap_or("all");
    let search = str_p(p, "search").unwrap_or("");
    let search_lc = search.to_lowercase();

    let assets: Vec<Value> = s
        .project
        .media
        .iter()
        .filter(|a| {
            let kind_ok = match filter {
                "video" => a.kind == AssetKind::Video,
                "audio" => a.kind == AssetKind::Audio,
                "image" => a.kind == AssetKind::Image,
                _ => true,
            };
            let search_ok = search.is_empty()
                || a.name.to_lowercase().contains(&search_lc)
                || a.path.to_lowercase().contains(&search_lc);
            kind_ok && search_ok
        })
        .map(|a| {
            json!({
                "id": a.id.0,
                "name": a.name,
                "path": a.path,
                "kind": format!("{:?}", a.kind).to_lowercase(),
                "duration": a.duration,
                "width": a.width,
                "height": a.height,
                "fps": a.fps,
                "sizeBytes": a.size_bytes,
                "hasAudio": a.has_audio,
                "codec": a.codec,
                "thumbnailPath": a.thumbnail_path,
                "filmstripPath": a.filmstrip_path,
                "waveformPath": a.waveform_path,
                "proxyPath": a.proxy_path,
                "createdAt": a.created_at,
                "missing": a.missing,
            })
        })
        .collect();

    Ok(json!({ "assets": assets }))
}

fn asset_kind(path: &str) -> AssetKind {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "mp4" | "mov" | "webm" | "mkv" | "avi" | "mxf" | "mts" | "m2ts" | "ts" | "mpg" | "mpeg" => AssetKind::Video,
        "mp3" | "wav" | "m4a" | "aac" | "flac" | "ogg" | "opus" | "aif" | "aiff" | "bwf" => AssetKind::Audio,
        "jpg" | "jpeg" | "png" | "webp" | "gif" | "tif" | "tiff" | "bmp" | "tga" | "dpx" | "exr" | "hdr" | "dng" | "psd" => AssetKind::Image,
        _ => AssetKind::Unknown,
    }
}

/// Compute a cheap fingerprint: hash of size + first 1 MiB + last 1 MiB.
fn fingerprint_hash(path: &str, size: u64) -> Option<u64> {
    let mut hasher = DefaultHasher::new();
    size.hash(&mut hasher);
    if let Ok(mut f) = std::fs::File::open(std::path::Path::new(path)) {
        let mut buf = [0u8; 1_048_576];
        let n = f.read(&mut buf).unwrap_or(0);
        buf[..n].hash(&mut hasher);
        if size as usize > n {
            let skip = (size as usize - n).saturating_sub(1_048_576);
            let _ = f.seek(SeekFrom::Start(skip as u64));
            let mut buf2 = [0u8; 1_048_576];
            let n2 = f.read(&mut buf2).unwrap_or(0);
            buf2[..n2].hash(&mut hasher);
        }
    }
    Some(hasher.finish())
}

fn unique_media_path(media_dir: &str, name: &str) -> String {
    let stem = std::path::Path::new(name)
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    let ext = std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let mut candidate = format!("{media_dir}/{name}");
    let mut n = 1;
    while std::path::Path::new(&candidate).exists() {
        candidate = format!("{media_dir}/{stem} ({n}).{ext}");
        n += 1;
    }
    candidate
}

fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// All Media Library commands, for `command_specs()` to extend.
pub fn commands() -> Vec<crate::commands::CommandSpec> {
    use crate::commands::{always, CommandSpec};
    let c = |id: &'static str, label: &'static str, params: &'static str, run: fn(&mut Session, &Value) -> Result<Value>| CommandSpec {
        id,
        label,
        menu: &[],
        shortcut: None,
        params,
        enabled: always,
        run,
        journal: true,
    };
    let q = |id: &'static str, label: &'static str, params: &'static str, run: fn(&mut Session, &Value) -> Result<Value>| CommandSpec {
        id,
        label,
        menu: &[],
        shortcut: None,
        params,
        enabled: always,
        run,
        journal: false,
    };
    vec![
        c(
            "media.import",
            "Import to Media Library…",
            r#"{"paths":[str],"copy":bool?,"bin":binId?}"#,
            |s, p| import(s, p),
        ),
        c(
            "media.addToTimeline",
            "Add to Timeline",
            r#"{"assets":[id],"track":str?,"start":ticks?}"#,
            |s, p| add_to_timeline(s, p),
        ),
        c(
            "media.remove",
            "Remove from Media Library",
            r#"{"assets":[id],"deleteFiles":bool?,"replaceWith":id?}"#,
            |s, p| remove(s, p),
        ),
        q(
            "media.list",
            "List Media Assets",
            r#"{"filter":"video|audio|image|all"?,"search":str?}"#,
            |s, p| list(s, p),
        ),
    ]
}

fn bad(cmd: &str, msg: impl Into<String>) -> EngineError {
    EngineError::BadParams { cmd: cmd.to_string(), msg: msg.into() }
}

// These helper functions are also defined in commands.rs; re-declare locally to avoid import issues.
fn str_p<'a>(p: &'a Value, key: &str) -> Option<&'a str> {
    p.get(key).and_then(Value::as_str)
}

fn u64_p(p: &Value, key: &str) -> Option<u64> {
    p.get(key).and_then(Value::as_u64)
}
