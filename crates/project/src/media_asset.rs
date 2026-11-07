//! Media library assets: the project's registry of imported media, stored in the project's
//! `media/` folder. Each `MediaAsset` is the project's managed copy of a source file, with
//! generated thumbnails, filmstrips and waveforms cached alongside it.
//!
//! `Project.media` holds all assets of a project; `TrackItem.media_id` references the asset a
//! clip shows. Assets are project-scoped, never global — moving the project folder moves the
//! media with it.

use std::collections::BTreeMap;

use filmcraft_time::{FrameRate, Tick};
use serde::{Deserialize, Serialize};

id_type!(MediaAssetId);

/// The kind of an asset, derived from its file extension / container.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetKind {
    #[default]
    Unknown,
    Video,
    Audio,
    Image,
}

/// One managed media asset in a project's media library.
///
/// Files are copied into the project's `media/` folder on import; paths here are relative to
/// that folder. Generated previews (thumbnail, filmstrip, waveform, proxy) live in
/// `cache/thumbs`, `cache/filmstrips`, `cache/waveforms`, `cache/proxy` next to the project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaAsset {
    pub id: MediaAssetId,
    /// File name inside `media/`, e.g. `media/Interview.mp4`.
    pub name: String,
    /// Relative path inside `media/`.
    pub path: String,
    pub kind: AssetKind,
    /// Duration in seconds (video/audio); stills report the default still duration.
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub size_bytes: u64,
    pub has_audio: bool,
    pub codec: Option<String>,
    /// `cache/thumbs/<id>.jpg` — a single frame at ~10% of duration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_path: Option<String>,
    /// `cache/filmstrips/<id>.jpg` — 8–16 uniform frames as a sprite sheet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filmstrip_path: Option<String>,
    /// `cache/waveforms/<id>.json` — `{ peaks: number[] }` normalised 0..1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waveform_path: Option<String>,
    /// `cache/proxy/<id>.mp4` — 720p proxy for sources > 1080p.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_path: Option<String>,
    pub created_at: u64,
    /// `true` when the managed file in `media/` is missing from disk.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub missing: bool,
    /// SHA-1 of file size + first 1 MiB + last 1 MiB, for de-duplication on re-import.
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "hex_u64::serialize_option", deserialize_with = "hex_u64::deserialize_option")]
    pub fingerprint: Option<u64>,
}

impl Default for MediaAsset {
    fn default() -> Self {
        Self {
            id: MediaAssetId(0),
            name: String::new(),
            path: String::new(),
            kind: AssetKind::Unknown,
            duration: 0.0,
            width: 0,
            height: 0,
            fps: 0.0,
            size_bytes: 0,
            has_audio: false,
            codec: None,
            thumbnail_path: None,
            filmstrip_path: None,
            waveform_path: None,
            proxy_path: None,
            created_at: 0,
            missing: false,
            fingerprint: None,
        }
    }
}

mod hex_u64 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{v:016x}"))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        let s = String::deserialize(d)?;
        u64::from_str_radix(&s, 16).map_err(serde::de::Error::custom)
    }

    pub fn serialize_option<S: Serializer>(v: &Option<u64>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(n) => s.serialize_str(&format!("{n:016x}")),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize_option<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
        let s = Option::<String>::deserialize(d)?;
        match s {
            Some(s) => u64::from_str_radix(&s, 16).map(Some).map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

/// The project's media library: all managed assets of one project.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaLibrary {
    pub assets: BTreeMap<MediaAssetId, MediaAsset>,
    pub next_id: u64,
}

impl MediaLibrary {
    pub fn new() -> Self {
        Self {
            assets: BTreeMap::new(),
            next_id: 1,
        }
    }

    pub fn alloc_id(&mut self) -> MediaAssetId {
        let id = MediaAssetId(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn insert(&mut self, asset: MediaAsset) -> MediaAssetId {
        let id = asset.id;
        self.assets.insert(id, asset);
        id
    }

    pub fn get(&self, id: MediaAssetId) -> Option<&MediaAsset> {
        self.assets.get(&id)
    }

    pub fn get_mut(&mut self, id: MediaAssetId) -> Option<&mut MediaAsset> {
        self.assets.get_mut(&id)
    }

    pub fn remove(&mut self, id: MediaAssetId) -> Option<MediaAsset> {
        self.assets.remove(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &MediaAsset> {
        self.assets.values()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    /// Find an asset by its managed path (e.g. `media/Interview.mp4`).
    pub fn by_path(&self, path: &str) -> Option<&MediaAsset> {
        self.assets.values().find(|a| a.path == path)
    }

    /// Find an asset by fingerprint (de-duplication on re-import).
    pub fn by_fingerprint(&self, fingerprint: u64) -> Option<&MediaAsset> {
        self
            .assets
            .values()
            .find(|a| a.fingerprint.is_some_and(|f| f == fingerprint))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_ids_are_monotonic() {
        let mut lib = MediaLibrary::new();
        let a = lib.alloc_id();
        let b = lib.alloc_id();
        assert!(b.0 > a.0);
    }

    #[test]
    fn by_path_finds_asset() {
        let mut lib = MediaLibrary::new();
        let id = lib.insert(MediaAsset {
            id: lib.alloc_id(),
            name: "clip.mp4".into(),
            path: "media/clip.mp4".into(),
            ..Default::default()
        });
        assert_eq!(lib.by_path("media/clip.mp4").map(|a| a.id), Some(id));
        assert_eq!(lib.by_path("media/other.mp4"), None);
    }

    #[test]
    fn by_fingerprint_deduplicates() {
        let mut lib = MediaLibrary::new();
        let id = lib.insert(MediaAsset {
            id: lib.alloc_id(),
            name: "clip.mp4".into(),
            path: "media/clip.mp4".into(),
            fingerprint: Some(0xdeadbeef),
            ..Default::default()
        });
        assert_eq!(
            lib.by_fingerprint(0xdeadbeef).map(|a| a.id),
            Some(id)
        );
        assert_eq!(lib.by_fingerprint(0xcafe), None);
    }
}
