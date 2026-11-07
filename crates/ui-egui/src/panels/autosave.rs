//! Auto-save state and periodic project persistence.

use std::time::{SystemTime, UNIX_EPOCH};

/// Auto-save state: whether enabled, interval in seconds, and last save timestamp.
#[derive(Clone, Debug, Default)]
pub struct AutosaveState {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub last_save: Option<u64>,
    pub pending: bool,
}

impl AutosaveState {
    pub fn now_seconds() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
    }

    pub fn should_save(&self) -> bool {
        if !self.enabled {
            return false;
        }
        let now = Self::now_seconds();
        match self.last_save {
            None => true,
            Some(last) => now.saturating_sub(last) >= self.interval_seconds,
        }
    }

    pub fn mark_saved(&mut self) {
        self.last_save = Some(Self::now_seconds());
        self.pending = false;
    }

    pub fn mark_pending(&mut self) {
        self.pending = true;
    }
}
