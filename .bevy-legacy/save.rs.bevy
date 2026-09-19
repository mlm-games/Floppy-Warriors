use bevy::prelude::*;
use game_utils::save::Versioned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const SAVE_VERSION: u32 = 2;

#[derive(Resource, Clone, Serialize, Deserialize)]
pub struct SaveData {
    #[serde(default)]
    pub version: u32,
    /// Kept for template HUD compatibility (maps to best score).
    pub high_score: u32,
    pub settings: SettingsData,
    #[serde(default)]
    pub bones: u32,
    #[serde(default)]
    pub meta_levels: HashMap<String, u32>,
    #[serde(default)]
    pub best_round: u32,
    #[serde(default)]
    pub total_runs: u32,
    #[serde(default)]
    pub total_kills: u32,
    #[serde(default)]
    pub total_victories: u32,
    #[serde(default)]
    pub last_played_unix: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SettingsData {
    pub master_volume: f32,
    pub sfx_volume: f32,
    pub music_volume: f32,
    pub language: String,
}

impl Default for SettingsData {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            sfx_volume: 1.0,
            music_volume: 0.8,
            language: "en".into(),
        }
    }
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            version: SAVE_VERSION,
            high_score: 0,
            settings: SettingsData::default(),
            bones: 0,
            meta_levels: HashMap::new(),
            best_round: 0,
            total_runs: 0,
            total_kills: 0,
            total_victories: 0,
            last_played_unix: 0,
        }
    }
}

impl Versioned for SaveData {
    fn version(&self) -> u32 {
        self.version
    }

    fn set_version(&mut self, version: u32) {
        self.version = version;
    }
}

impl SaveData {
    pub fn meta_level(&self, id: &str) -> u32 {
        *self.meta_levels.get(id).unwrap_or(&0)
    }

    /// Normalize legacy/corrupt saves after load. Future migrations key off
    /// `self.version` and bump `SAVE_VERSION` here before releasing.
    pub fn migrate(&mut self) {
        if self.version < SAVE_VERSION {
            bevy::log::info!("migrating save {} -> {}", self.version, SAVE_VERSION);
            self.version = SAVE_VERSION;
        }

        self.settings.master_volume = self.settings.master_volume.clamp(0.0, 1.0);
        self.settings.sfx_volume = self.settings.sfx_volume.clamp(0.0, 1.0);
        self.settings.music_volume = self.settings.music_volume.clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_clamps_volume_settings() {
        let mut save = SaveData {
            settings: SettingsData {
                master_volume: 2.5,
                sfx_volume: -1.0,
                music_volume: 0.5,
                language: "en".into(),
            },
            ..Default::default()
        };
        save.migrate();
        assert_eq!(save.settings.master_volume, 1.0);
        assert_eq!(save.settings.sfx_volume, 0.0);
        assert_eq!(save.settings.music_volume, 0.5);
        assert_eq!(save.version, SAVE_VERSION);
    }

    #[test]
    fn migrate_keeps_current_version() {
        let save = SaveData::default();
        assert_eq!(save.version, SAVE_VERSION);
    }
}
