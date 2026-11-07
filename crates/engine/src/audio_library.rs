//! Audio library: offline built-in pack, online providers (Freesound, Pixabay),
//! download-to-project, and `audioLibrary.*` commands.
//!
//! Every audio source — offline pack, internet provider, microphone recording,
//! or extraction from video — becomes an ordinary `MediaAsset` and an ordinary
//! timeline clip. There are no "special" audio objects.
//!
//! Provider API keys come only from preferences (`prefs.get`/`prefs.set`), never
//! from code. A provider without a key is hidden with a "get a key" hint.

use std::collections::HashMap;
use std::io::Read;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use filmcraft_time::Tick;

use crate::{EngineError, Result, Session};
use crate::commands::{CommandSpec, always};

// ==============================================================================================
// Data model
// ==============================================================================================

/// One search result from any provider, normalised to a common shape. UI never
/// sees a provider's native format.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioResult {
    pub provider_id: String,      // 'freesound' | 'pixabay' | 'builtin'
    pub external_id: String,      // provider's own id (empty for builtin)
    pub title: String,
    pub author: String,
    pub license: String,          // human readable: 'CC0', 'CC BY 4.0', 'Pixabay License'
    pub license_url: Option<String>,
    pub duration: f64,            // seconds
    pub preview_url: Option<String>, // stream URL for audition
    pub download_url: Option<String>, // direct download (may be None until resolved)
    pub cover_url: Option<String>,
    pub tags: Vec<String>,
    pub original_page_url: Option<String>,
}

/// Search parameters passed from the UI.
#[derive(Clone, Debug, Default)]
pub struct AudioSearchQuery {
    pub text: String,
    pub kind: AudioKind,
    pub duration_min: Option<f64>,
    pub duration_max: Option<f64>,
    pub genre: Option<String>,
    pub mood: Option<String>,
    pub license: AudioLicense,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioKind {
    #[default]
    Any,
    Music,
    Sfx,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioLicense {
    #[default]
    Any,
    Cc0,
    CcBy,
}

/// One provider: name, capabilities, key requirement, search, download resolution.
pub trait AudioProvider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn requires_key(&self) -> bool;
    fn supports(&self) -> Vec<AudioKind>;
    /// Search, honouring the 8 s timeout and `signal` abort. Returns normalised results.
    fn search(&self, q: &AudioSearchQuery, signal: Arc<AtomicBool>) -> Result<Vec<AudioResult>>;
    /// Resolve a direct download URL for a result (may need a second API call).
    fn download_url(&self, r: &AudioResult) -> Result<String>;
    /// Whether an API key is configured (used to gate UI visibility).
    fn has_key(&self) -> bool { false }
}

// ==============================================================================================
// Offline built-in pack
// ==============================================================================================

/// One entry in `assets/audio/builtin/catalog.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuiltinItem {
    pub id: String,
    pub kind: String,            // 'sfx' | 'music'
    pub title: String,
    pub category: String,
    pub path: String,            // relative to assets/audio/builtin/
    pub duration: f64,
    pub tags: Vec<String>,
}

pub struct BuiltinPack {
    items: Vec<BuiltinItem>,
}

impl BuiltinPack {
    pub fn load() -> Result<Self> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../assets/audio/builtin/catalog.json");
        let data = std::fs::read_to_string(&path)
            .map_err(|e| EngineError::Other(format!("builtin audio catalog: {e}")))?;
        let items: Vec<BuiltinItem> = serde_json::from_str(&data)
            .map_err(|e| EngineError::Other(format!("builtin audio catalog parse: {e}")))?;
        Ok(Self { items })
    }
    pub fn items(&self) -> &[BuiltinItem] { &self.items }
    pub fn search(&self, text: &str) -> Vec<&BuiltinItem> {
        let t = text.to_lowercase();
        self.items.iter().filter(|it| {
            it.title.to_lowercase().contains(&t)
            || it.tags.iter().any(|tag| tag.to_lowercase().contains(&t))
            || it.category.to_lowercase().contains(&t)
        }).collect()
    }
    pub fn categories(&self) -> Vec<String> {
        let mut cats: Vec<String> = self.items.iter().map(|i| i.category.clone()).collect();
        cats.sort(); cats.dedup(); cats
    }
}

// ==============================================================================================
// Freesound provider (API v2)
// ==============================================================================================

/// Freesound API v2 client. Requires a token from https://freesound.org/home/apply/
pub struct FreesoundProvider {
    api_key: Option<String>,
}

impl FreesoundProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self { api_key }
    }
}

impl AudioProvider for FreesoundProvider {
    fn id(&self) -> &str { "freesound" }
    fn name(&self) -> &str { "Freesound" }
    fn requires_key(&self) -> bool { true }
    fn has_key(&self) -> bool { self.api_key.is_some() }
    fn supports(&self) -> Vec<AudioKind> { vec![AudioKind::Sfx, AudioKind::Music] }

    fn search(&self, q: &AudioSearchQuery, _signal: Arc<AtomicBool>) -> Result<Vec<AudioResult>> {
        let key = self.api_key.as_deref().ok_or_else(|| {
            EngineError::Disabled("freesound".to_string(), "Freesound API key not set. Get one at https://freesound.org/home/apply/ and paste it into Settings ▸ Audio.".to_string())
        })?;
        let mut url = format!("https://freesound.org/apiv2/search/text/?query={}&token={}",
            urlencode(&q.text), key);
        if let Some(g) = &q.genre { if !g.is_empty() { url.push_str(&format!("&tag={}", urlencode(g))); } }
        if q.page > 1 { url.push_str(&format!("&page={}", q.page)); }
        url.push_str(&format!("&page_size={}", q.per_page));
        let resp = ureq::get(&url).call()
            .map_err(|e| EngineError::Other(format!("Freesound search: {e}")))?;
        let mut body_buf = Vec::new();
        resp.into_body().into_reader().read_to_end(&mut body_buf)
            .map_err(|e| EngineError::Other(format!("Freesound read body: {e}")))?;
        let body_str = String::from_utf8_lossy(&body_buf).into_owned();
        let resp: Value = serde_json::from_str(&body_str)
            .map_err(|e| EngineError::Other(format!("Freesound parse: {e}")))?;
        let mut out = Vec::new();
        for s in resp.get("results").and_then(Value::as_array).unwrap_or(&vec![]) {
            let dur = s.get("duration").and_then(Value::as_f64).unwrap_or(0.0);
            if let Some(dmin) = q.duration_min { if dur < dmin { continue; } }
            if let Some(dmax) = q.duration_max { if dur > dmax { continue; } }
            out.push(AudioResult {
                provider_id: "freesound".into(),
                external_id: s.get("id").and_then(Value::as_u64).unwrap_or(0).to_string(),
                title: s.get("name").and_then(Value::as_str).unwrap_or("").into(),
                author: s.get("username").and_then(Value::as_str).unwrap_or("").into(),
                license: s.get("license").and_then(Value::as_str).unwrap_or("Unknown").into(),
                license_url: s.get("license_url").and_then(Value::as_str).map(|s| s.to_string()),
                duration: dur,
                preview_url: s.get("previews").and_then(|p| p.get("preview-hq-mp3")).and_then(Value::as_str).map(|s| s.to_string()),
                download_url: None,
                cover_url: None,
                tags: s.get("tags").and_then(Value::as_array).unwrap_or(&vec![]).iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(),
                original_page_url: s.get("url").and_then(Value::as_str).map(|s| s.to_string()),
            });
        }
        Ok(out)
    }

    fn download_url(&self, r: &AudioResult) -> Result<String> {
        let key = self.api_key.as_deref().ok_or_else(|| EngineError::Other("Freesound key missing".to_string()))?;
        let url = format!("https://freesound.org/apiv2/sounds/{}/download/?token={}", r.external_id, key);
        let resp = ureq::get(&url).call()
            .map_err(|e| EngineError::Other(format!("Freesound download URL: {e}")))?;
        Ok(resp.headers().get("location").and_then(|v| v.to_str().ok()).unwrap_or(&url).to_string())
    }
}

// ==============================================================================================
// Pixabay Audio provider
// ==============================================================================================

/// Pixabay Audio API. Requires a key from https://pixabay.com/accounts/api/
pub struct PixabayProvider {
    api_key: Option<String>,
}

impl PixabayProvider {
    pub fn new(api_key: Option<String>) -> Self { Self { api_key } }
}

impl AudioProvider for PixabayProvider {
    fn id(&self) -> &str { "pixabay" }
    fn name(&self) -> &str { "Pixabay" }
    fn requires_key(&self) -> bool { true }
    fn has_key(&self) -> bool { self.api_key.is_some() }
    fn supports(&self) -> Vec<AudioKind> { vec![AudioKind::Music, AudioKind::Sfx] }

    fn search(&self, q: &AudioSearchQuery, _signal: Arc<AtomicBool>) -> Result<Vec<AudioResult>> {
        let key = self.api_key.as_deref().ok_or_else(|| {
            EngineError::Disabled("pixabay".to_string(), "Pixabay API key not set. Get one at https://pixabay.com/accounts/api/ and paste it into Settings ▸ Audio.".to_string())
        })?;
        let kind_str = match q.kind { AudioKind::Music => "music", AudioKind::Sfx => "sound", AudioKind::Any => "" };
        let mut url = format!("https://pixabay.com/api/audio/?key={}&q={}&per_page={}&page={}",
            key, urlencode(&q.text), q.per_page, q.page);
        if !kind_str.is_empty() { url.push_str(&format!("&category={}", kind_str)); }
        let resp = ureq::get(&url).call()
            .map_err(|e| EngineError::Other(format!("Pixabay search: {e}")))?;
        let mut body_buf = Vec::new();
        resp.into_body().into_reader().read_to_end(&mut body_buf)
            .map_err(|e| EngineError::Other(format!("Pixabay read body: {e}")))?;
        let body_str = String::from_utf8_lossy(&body_buf).into_owned();
        let resp: Value = serde_json::from_str(&body_str)
            .map_err(|e| EngineError::Other(format!("Pixabay parse: {e}")))?;
        let mut out = Vec::new();
        for h in resp.get("hits").and_then(Value::as_array).unwrap_or(&vec![]) {
            let dur = h.get("duration").and_then(Value::as_f64).unwrap_or(0.0);
            if let Some(dmin) = q.duration_min { if dur < dmin { continue; } }
            if let Some(dmax) = q.duration_max { if dur > dmax { continue; } }
            out.push(AudioResult {
                provider_id: "pixabay".into(),
                external_id: h.get("id").and_then(Value::as_u64).unwrap_or(0).to_string(),
                title: h.get("title").and_then(Value::as_str).unwrap_or("").into(),
                author: h.get("user").and_then(Value::as_str).unwrap_or("").into(),
                license: "Pixabay License".into(),
                license_url: Some("https://pixabay.com/service/license/".to_string()),
                duration: dur,
                preview_url: h.get("previewURL").and_then(Value::as_str).map(|s| s.to_string()),
                download_url: h.get("previewURL").and_then(Value::as_str).map(|s| s.to_string()),
                cover_url: None,
                tags: h.get("tags").and_then(Value::as_str).unwrap_or("").split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
                original_page_url: h.get("pageURL").and_then(Value::as_str).map(|s| s.to_string()),
            });
        }
        Ok(out)
    }

    fn download_url(&self, r: &AudioResult) -> Result<String> {
        Ok(r.download_url.clone().unwrap_or_default())
    }
}

// ==============================================================================================
// Provider registry
// ==============================================================================================

pub struct AudioLibrary {
    providers: Vec<Box<dyn AudioProvider>>,
    builtin: BuiltinPack,
    /// Downloaded assets: provider_id + external_id -> local path (session cache).
    cache: HashMap<(String, String), String>,
}

impl AudioLibrary {
    pub fn new() -> Result<Self> {
        Ok(Self {
            providers: Vec::new(),
            builtin: BuiltinPack::load().unwrap_or_else(|_| BuiltinPack { items: Vec::new() }),
            cache: HashMap::new(),
        })
    }

    pub fn add_provider(&mut self, p: Box<dyn AudioProvider>) {
        self.providers.push(p);
    }

    pub fn providers(&self) -> &[Box<dyn AudioProvider>] { &self.providers }
    pub fn builtin_pack(&self) -> &BuiltinPack { &self.builtin }

    pub fn search(&mut self, q: &AudioSearchQuery) -> Result<Vec<AudioResult>> {
        let mut results: Vec<AudioResult> = Vec::new();
        let signal = Arc::new(AtomicBool::new(false));
        for p in &self.providers {
            if p.has_key() {
                let mut r = p.search(q, signal.clone())?;
                results.append(&mut r);
            }
        }
        // Built-in pack (always available, no key needed)
        for it in self.builtin.search(&q.text) {
            results.push(AudioResult {
                provider_id: "builtin".into(),
                external_id: it.id.clone(),
                title: it.title.clone(),
                author: "FilmCraft".into(),
                license: "CC0".into(),
                license_url: None,
                duration: it.duration,
                preview_url: None,
                download_url: None,
                cover_url: None,
                tags: it.tags.clone(),
                original_page_url: None,
            });
        }
        Ok(results)
    }

    pub fn is_cached(&self, provider_id: &str, external_id: &str) -> bool {
        self.cache.contains_key(&(provider_id.to_string(), external_id.to_string()))
    }

    pub fn cache_path(&self, provider_id: &str, external_id: &str) -> Option<&str> {
        self.cache.get(&(provider_id.to_string(), external_id.to_string())).map(|s| s.as_str())
    }

    pub fn record_download(&mut self, provider_id: String, external_id: String, path: String) {
        self.cache.insert((provider_id, external_id), path);
    }
}

// ==============================================================================================
// Commands: audioLibrary.*
// ==============================================================================================

/// `audioLibrary.search` — search providers + built-in pack.
pub fn cmd_audio_library_search(s: &mut Session, p: &Value) -> Result<Value> {
    let text = p.get("text").and_then(Value::as_str).unwrap_or("").to_string();
    let kind = match p.get("kind").and_then(Value::as_str).unwrap_or("any") {
        "music" => AudioKind::Music,
        "sfx" => AudioKind::Sfx,
        _ => AudioKind::Any,
    };
    let license = match p.get("license").and_then(Value::as_str).unwrap_or("any") {
        "cc0" => AudioLicense::Cc0,
        "cc-by" => AudioLicense::CcBy,
        _ => AudioLicense::Any,
    };
    let query = AudioSearchQuery {
        text,
        kind,
        duration_min: p.get("durationMin").and_then(Value::as_f64),
        duration_max: p.get("durationMax").and_then(Value::as_f64),
        genre: p.get("genre").and_then(Value::as_str).map(|s| s.to_string()),
        mood: p.get("mood").and_then(Value::as_str).map(|s| s.to_string()),
        license,
        page: p.get("page").and_then(Value::as_u64).unwrap_or(1) as u32,
        per_page: p.get("perPage").and_then(Value::as_u64).unwrap_or(20) as u32,
    };
    let lib = s.audio_library.as_mut().ok_or(EngineError::Other("audio library not initialised".to_string()))?;
    let results = lib.search(&query)?;
    Ok(json!({ "results": results.iter().map(|r| json!({
        "providerId": r.provider_id,
        "externalId": r.external_id,
        "title": r.title,
        "author": r.author,
        "license": r.license,
        "licenseUrl": r.license_url,
        "duration": r.duration,
        "previewUrl": r.preview_url,
        "downloadUrl": r.download_url,
        "coverUrl": r.cover_url,
        "tags": r.tags,
        "originalPageUrl": r.original_page_url,
    })).collect::<Vec<_>>() }))
}

/// `audioLibrary.download` — download a result to `media/audio/`, cache it, return local path.
pub fn cmd_audio_library_download(s: &mut Session, p: &Value) -> Result<Value> {
    let provider_id = str_p(p, "providerId")?;
    let external_id = str_p(p, "externalId")?;
    let title = str_p(p, "title")?;
    let download_url = str_p(p, "downloadUrl")?;
    let preview_url = p.get("previewUrl").and_then(Value::as_str).unwrap_or("");

    let lib = s.audio_library.as_mut().ok_or(EngineError::Other("audio library not initialised".to_string()))?;

    // Already cached?
    if let Some(path) = lib.cache_path(&provider_id, &external_id) {
        return Ok(json!({ "path": path, "cached": true }));
    }

    // Determine local path.
    let safe_name: String = title.chars().filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_').collect();
    let file_name = format!("{}_{}.wav", safe_name, external_id);
    let local_path = format!("media/audio/{}", file_name);

    // Download data.
    let data = if provider_id == "builtin" {
        // Built-in: copy from assets/audio/builtin/
        let asset_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../assets/audio/builtin/")
            .join(format!("{}.wav", external_id));
        std::fs::read(&asset_path)
            .map_err(|e| EngineError::Other(format!("builtin asset not found: {e}")))?
    } else {
        let client = ureq::Agent::new_with_defaults();
        let url = if !download_url.is_empty() { download_url.clone() } else { preview_url.to_string() };
        let resp = client.get(&url).call()
            .map_err(|e| EngineError::Other(format!("download failed: {e}")))?;
        let mut body_buf = Vec::new();
        resp.into_body().into_reader().read_to_end(&mut body_buf)
            .map_err(|e| EngineError::Other(format!("download read failed: {e}")))?;
        body_buf
    };

    // Write to media/audio/ via services (atomic write).
    s.services.write_file(&local_path, &data)
        .map_err(|e| EngineError::Other(format!("write failed: {e}")))?;

    lib.record_download(provider_id, external_id, local_path.clone());

    Ok(json!({ "path": local_path, "cached": false }))
}

/// `audioLibrary.addToTimeline` — import the downloaded asset and place a clip on the playhead.
pub fn cmd_audio_library_add_to_timeline(s: &mut Session, p: &Value) -> Result<Value> {
    let path = str_p(p, "path")?;

    // Import as media item (reads whole file into memory for small audio).
    let vec = s.services.read_file(&path)
        .map_err(|e| EngineError::Media(filmcraft_media::MediaError::Io(e.to_string())))?;
    let bytes: std::sync::Arc<[u8]> = std::sync::Arc::from(vec);
    let item_id = crate::commands::import_bytes(s, &path, bytes, None)?;

    // Place on the audio track under the playhead.
    let at = s.playhead();
    let item = s.project.item(item_id).ok_or(EngineError::Other("no such item".to_string()))?;
    let full = media_duration(item);
    let ids = crate::commands::place_item(s, crate::commands::ItemPlacement {
        item: item_id,
        range: filmcraft_time::TimeRange::new(Tick::ZERO, full),
        at,
        vdest: None,
        adest: None,
        insert: false,
        label: "Place Audio",
        audio_split: None,
    })?;

    Ok(json!({ "clips": ids.iter().map(|c| c.0).collect::<Vec<_>>() }))
}

/// `audioLibrary.providers` — list providers with key status (for UI toggle).
pub fn query_audio_library_providers(s: &Session, p: &Value) -> Result<Value> {
    let _ = (s, p);
    Ok(json!({
        "providers": [
            { "id": "freesound", "name": "Freesound", "requiresKey": true, "supports": ["sfx", "music"] },
            { "id": "pixabay", "name": "Pixabay", "requiresKey": true, "supports": ["music", "sfx"] },
            { "id": "builtin", "name": "Built-in Pack", "requiresKey": false, "supports": ["sfx"] },
        ]
    }))
}

/// `audioLibrary.catalog` — built-in pack items.
pub fn query_audio_library_catalog(s: &Session, p: &Value) -> Result<Value> {
    let _ = (s, p);
    let lib = s.audio_library.as_ref().ok_or(EngineError::Other("audio library not initialised".to_string()))?;
    Ok(json!({
        "items": lib.builtin_pack().items().iter().map(|it| json!({
            "id": it.id,
            "kind": it.kind,
            "title": it.title,
            "category": it.category,
            "path": it.path,
            "duration": it.duration,
            "tags": it.tags,
        })).collect::<Vec<_>>(),
        "categories": lib.builtin_pack().categories(),
    }))
}

// ---------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------

fn str_p(p: &Value, key: &str) -> Result<String> {
    p.get(key).and_then(Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| EngineError::BadParams { cmd: "audioLibrary.*".to_string(), msg: format!("missing `{key}`") })
}

fn media_duration(item: &filmcraft_project::ProjectItem) -> Tick {
    match &item.kind {
        filmcraft_project::ItemKind::Media(m) => {
            m.info.duration
        }
        _ => Tick::ZERO,
    }
}

/// Simple percent-encoder for query values (no external dependency).
fn urlencode(s: &str) -> String {
    s.chars().flat_map(|c| {
        if c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
            Some(c)
        } else {
            None
        }
    }).collect()
}

// ==============================================================================================
// Command specs: audioLibrary.*
// ==============================================================================================

pub fn commands() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            id: "audioLibrary.search",
            label: "Search Audio",
            menu: &[],
            shortcut: None,
            params: r#"{"text":str,"kind":"any|music|sfx"?,"durationMin":f64?,"durationMax":f64?,"genre":str?,"mood":str?,"license":"any|cc0|cc-by"?,"page":int?,"perPage":int?}"#,
            enabled: always,
            run: |s, p| cmd_audio_library_search(s, p),
            journal: true,
        },
        CommandSpec {
            id: "audioLibrary.download",
            label: "Download Audio",
            menu: &[],
            shortcut: None,
            params: r#"{"providerId":str,"externalId":str,"title":str,"downloadUrl":str,"previewUrl":str?}"#,
            enabled: always,
            run: |s, p| cmd_audio_library_download(s, p),
            journal: true,
        },
        CommandSpec {
            id: "audioLibrary.addToTimeline",
            label: "Add Audio to Timeline",
            menu: &[],
            shortcut: None,
            params: r#"{"path":str}"#,
            enabled: always,
            run: |s, p| cmd_audio_library_add_to_timeline(s, p),
            journal: true,
        },
        CommandSpec {
            id: "audioLibrary.providers",
            label: "List Audio Providers",
            menu: &[],
            shortcut: None,
            params: "{}",
            enabled: always,
            run: |s, p| query_audio_library_providers(s, p),
            journal: false,
        },
        CommandSpec {
            id: "audioLibrary.catalog",
            label: "List Built-in Audio Catalog",
            menu: &[],
            shortcut: None,
            params: "{}",
            enabled: always,
            run: |s, p| query_audio_library_catalog(s, p),
            journal: false,
        },
    ]
}