//! Project manager data model: list of local projects, persistence, and project creation.
//! Projects are stored as JSON files in the FilmCraft data directory.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Information about a saved project, shown in the project manager grid.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// Unique project id (folder name).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Absolute path to the project folder.
    pub path: PathBuf,
    /// Aspect ratio preset used when creating.
    pub aspect: String,
    /// Width × height.
    pub resolution: String,
    /// FPS.
    pub fps: u32,
    /// Last modification time (unix seconds).
    pub modified_at: u64,
    /// Total duration of the timeline in seconds (0 if empty).
    pub duration_secs: f64,
    /// Number of clips on the timeline.
    pub clip_count: usize,
    /// Path to a thumbnail PNG, if one was generated.
    pub thumbnail: Option<PathBuf>,
}

/// Sort order for the project grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ProjectSort {
    #[default]
    ModifiedDesc,
    NameAsc,
    DurationAsc,
}

/// Aspect ratio presets for new projects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CanvasPreset {
    /// 16:9 — YouTube, standard video.
    #[default]
    W16_9,
    /// 9:16 — TikTok, Reels, Shorts.
    H9_16,
    /// 1:1 — Square (Instagram post).
    S1_1,
    /// 4:5 — Instagram Reel/portrait.
    P4_5,
    /// 21:9 — Cinematic ultrawide.
    W21_9,
    /// Custom resolution entered by the user.
    Custom,
}

impl CanvasPreset {
    pub fn label(self) -> &'static str {
        match self {
            CanvasPreset::W16_9 => "16:9 YouTube",
            CanvasPreset::H9_16 => "9:16 TikTok/Reels",
            CanvasPreset::S1_1 => "1:1 Square",
            CanvasPreset::P4_5 => "4:5 Portrait",
            CanvasPreset::W21_9 => "21:9 Cinema",
            CanvasPreset::Custom => "Custom",
        }
    }

    pub fn resolution(self) -> (u32, u32) {
        match self {
            CanvasPreset::W16_9 => (1920, 1080),
            CanvasPreset::H9_16 => (1080, 1920),
            CanvasPreset::S1_1 => (1080, 1080),
            CanvasPreset::P4_5 => (1080, 1350),
            CanvasPreset::W21_9 => (2560, 1080),
            CanvasPreset::Custom => (1920, 1080),
        }
    }

    pub fn aspect_label(self) -> &'static str {
        match self {
            CanvasPreset::W16_9 => "16:9",
            CanvasPreset::H9_16 => "9:16",
            CanvasPreset::S1_1 => "1:1",
            CanvasPreset::P4_5 => "4:5",
            CanvasPreset::W21_9 => "21:9",
            CanvasPreset::Custom => "Custom",
        }
    }
}

/// Default FPS options.
pub const FPS_OPTIONS: [u32; 6] = [24, 25, 30, 50, 60, 120];

/// Crash recovery candidate found on startup.
#[derive(Clone, Debug)]
pub struct CrashCandidate {
    pub id: String,
    pub project_name: String,
    pub modified_at: u64,
}

/// Project manager: scans the data directory for saved projects and persists the list.
#[derive(Clone, Debug, Default)]
pub struct ProjectManager {
    pub projects: Vec<ProjectInfo>,
    pub search: String,
    pub sort: ProjectSort,
    pub data_dir: Option<PathBuf>,
}

impl ProjectManager {
    /// Scan `data_dir` for project folders and load their `project.json` metadata.
    pub fn scan(data_dir: &Path) -> Self {
        let mut projects = Vec::new();
        if let Ok(entries) = std::fs::read_dir(data_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let meta_path = path.join("project.json");
                    if meta_path.is_file() {
                        if let Ok(text) = std::fs::read_to_string(&meta_path) {
                            if let Ok(info) = serde_json::from_str::<ProjectInfo>(&text) {
                                projects.push(info);
                            }
                        }
                    }
                }
            }
        }
        let sort = ProjectSort::default();
        let mut pm = Self {
            projects,
            search: String::new(),
            sort,
            data_dir: Some(data_dir.to_path_buf()),
        };
        pm.apply_sort();
        pm
    }

    /// Filter by search text and apply sort order.
    pub fn apply_sort(&mut self) {
        let term = self.search.to_lowercase();
        self.projects.retain(|p| {
            term.is_empty()
                || p.name.to_lowercase().contains(&term)
                || p.id.to_lowercase().contains(&term)
        });
        match self.sort {
            ProjectSort::ModifiedDesc => {
                self.projects.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
            }
            ProjectSort::NameAsc => {
                self.projects.sort_by(|a, b| a.name.cmp(&b.name));
            }
            ProjectSort::DurationAsc => {
                self.projects
                    .sort_by(|a, b| a.duration_secs.partial_cmp(&b.duration_secs).unwrap_or(std::cmp::Ordering::Equal));
            }
        }
    }

    /// Save a new project's metadata file and return its info.
    pub fn create_project(
        &mut self,
        name: &str,
        preset: CanvasPreset,
        fps: u32,
        custom_res: Option<(u32, u32)>,
    ) -> Option<ProjectInfo> {
        let data_dir = self.data_dir.as_ref()?;
        let id = format!("{}-{}", name, SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs());
        let safe_id = id.replace(' ', "_").replace('/', "_");
        let project_dir = data_dir.join(&safe_id);
        let _ = std::fs::create_dir_all(&project_dir);
        let _ = std::fs::create_dir_all(project_dir.join("media"));
        let _ = std::fs::create_dir_all(project_dir.join("cache"));
        let (w, h) = match preset {
            CanvasPreset::Custom => custom_res.unwrap_or((1920, 1080)),
            p => p.resolution(),
        };
        let info = ProjectInfo {
            id: safe_id.clone(),
            name: name.to_string(),
            path: project_dir.clone(),
            aspect: preset.aspect_label().to_string(),
            resolution: format!("{}×{}", w, h),
            fps,
            modified_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            duration_secs: 0.0,
            clip_count: 0,
            thumbnail: None,
        };
        // Write project.json metadata.
        if let Ok(json) = serde_json::to_string_pretty(&info) {
            let _ = std::fs::write(project_dir.join("project.json"), json);
        }
        self.projects.push(info.clone());
        self.apply_sort();
        Some(info)
    }

    /// Delete a project folder and its metadata.
    pub fn delete_project(&mut self, id: &str) -> bool {
        if let Some(data_dir) = &self.data_dir {
            let project_dir = data_dir.join(id);
            if project_dir.is_dir() {
                let _ = std::fs::remove_dir_all(project_dir);
            }
        }
        let before = self.projects.len();
        self.projects.retain(|p| p.id != id);
        self.projects.len() < before
    }

    /// Update a project's metadata (e.g. after editing).
    pub fn touch_project(&mut self, id: &str, duration_secs: f64, clip_count: usize) {
        if let Some(p) = self.projects.iter_mut().find(|p| p.id == id) {
            p.modified_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            p.duration_secs = duration_secs;
            p.clip_count = clip_count;
            if let Ok(json) = serde_json::to_string_pretty(p) {
                if let Some(data_dir) = &self.data_dir {
                    let _ = std::fs::write(data_dir.join(&p.id).join("project.json"), json);
                }
            }
            self.apply_sort();
        }
    }

    /// Find a project by id.
    pub fn find(&self, id: &str) -> Option<&ProjectInfo> {
        self.projects.iter().find(|p| p.id == id)
    }
}

/// Filtered project list (search + sort applied).
#[derive(Clone, Debug, Default)]
pub struct HomeState {
    /// All scanned projects.
    pub manager: ProjectManager,
    /// Whether the "Create new project" dialog is open.
    pub create_dialog_open: bool,
    /// Name being typed in the create dialog.
    pub new_project_name: String,
    /// Selected preset in the create dialog.
    pub new_project_preset: CanvasPreset,
    /// Selected FPS in the create dialog.
    pub new_project_fps: u32,
    /// Custom width/height when preset is Custom.
    pub new_project_custom_w: u32,
    pub new_project_custom_h: u32,
    /// Crash recovery candidate waiting for user decision.
    pub crash_recovery: Option<CrashCandidate>,
    /// Whether we're showing the empty state (no projects at all).
    pub show_empty: bool,
    /// Project id awaiting a context menu action (delete / duplicate / rename / show in folder).
    pub pending_ctx_menu: Option<String>,
    /// Project id awaiting rename dialog.
    pub pending_rename: Option<String>,
    /// Rename text buffer.
    pub pending_rename_buffer: Option<String>,
    /// Project id awaiting delete confirmation.
    pub pending_delete: Option<String>,
}

impl HomeState {
    pub fn new(data_dir: &Path) -> Self {
        let manager = ProjectManager::scan(data_dir);
        let show_empty = manager.projects.is_empty();
        Self {
            manager,
            create_dialog_open: false,
            new_project_name: String::new(),
            new_project_preset: CanvasPreset::H9_16,
            new_project_fps: 30,
            new_project_custom_w: 1080,
            new_project_custom_h: 1920,
            crash_recovery: None,
            show_empty,
            pending_ctx_menu: None,
            pending_rename: None,
            pending_rename_buffer: None,
            pending_delete: None,
        }
    }

    /// Projects matching the current search and sort.
    pub fn filtered(&self) -> &[ProjectInfo] {
        &self.manager.projects
    }
}


