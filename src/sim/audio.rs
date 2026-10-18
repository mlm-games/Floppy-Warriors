use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use bevy_ecs::prelude::*;
use glam::Vec2;
use repame_audio::{AudioChannel, CueDef, Variation};

#[derive(Clone, Debug)]
pub struct AudioCue {
    pub name: &'static str,
    pub volume: f32,
    pub variance: f32,
}

#[derive(Resource, Clone, Default)]
pub struct CueQueue(pub Arc<Mutex<Vec<AudioCue>>>);

const SOUND_EXTS: [&str; 4] = ["ogg", "wav", "mp3", "flac"];

fn sounds_dir() -> Option<PathBuf> {
    #[cfg(target_os = "android")]
    if let Some(dir) = game_utils::storage::android_runtime_dir() {
        return Some(dir.join("assets/audio/sounds"));
    }
    [".", "..", "../.."]
        .iter()
        .map(|root| Path::new(root).join("assets/audio/sounds"))
        .find(|dir| dir.is_dir())
}

/// Host-side audio: a live device when one opens, `Audio::noop()` otherwise.
/// Cues resolve to `assets/audio/sounds/<name>.<ext>` on first play, so a
/// build without sound files stays silent instead of failing to boot.
pub struct AudioHost {
    audio: repame_audio::Audio,
    sounds: Option<PathBuf>,
    loaded: HashSet<String>,
    missing: HashSet<String>,
    warned_missing: bool,
}

impl Default for AudioHost {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioHost {
    pub fn new() -> Self {
        let audio = repame_audio::Audio::try_init().unwrap_or_else(|error| {
            log::warn!("floppy-warriors: audio device failed ({error}); running silent");
            repame_audio::Audio::noop()
        });
        let sounds = sounds_dir();
        if sounds.is_none() {
            log::warn!("floppy-warriors: no assets/audio/sounds directory; cues silent");
        }
        Self {
            audio,
            sounds,
            loaded: HashSet::new(),
            missing: HashSet::new(),
            warned_missing: false,
        }
    }

    pub fn is_live(&self) -> bool {
        self.audio.is_live()
    }

    pub fn set_volume(&mut self, channel: AudioChannel, gain: f32) {
        self.audio.set_channel(channel, gain);
    }

    pub fn update(&mut self, dt_secs: f32) {
        self.audio.update(dt_secs);
        self.audio.take_finished();
    }

    pub fn play(&mut self, cue: &AudioCue) {
        if !self.ensure_cue(cue) {
            return;
        }
        self.audio.play(cue.name);
    }

    fn ensure_cue(&mut self, cue: &AudioCue) -> bool {
        if self.loaded.contains(cue.name) {
            return true;
        }
        if self.missing.contains(cue.name) {
            return false;
        }
        let Some(sounds) = self.sounds.clone() else {
            return false;
        };
        let mut files: Vec<Vec<u8>> = Vec::new();
        for ext in SOUND_EXTS {
            let path = sounds.join(format!("{}.{ext}", cue.name));
            match std::fs::read(&path) {
                Ok(bytes) => files.push(bytes),
                Err(_) => continue,
            }
        }
        if files.is_empty() {
            self.missing.insert(cue.name.to_string());
            if !self.warned_missing {
                self.warned_missing = true;
                log::warn!(
                    "floppy-warriors: no sound file for cue '{}' under {}",
                    cue.name,
                    sounds.display()
                );
            }
            return false;
        }
        let refs: Vec<&[u8]> = files.iter().map(|bytes| bytes.as_slice()).collect();
        let def = CueDef {
            bus: AudioChannel::Sfx,
            gain: cue.volume.clamp(0.0, 4.0),
            cooldown_ms: 0,
            max_voices: 8,
            pitch_wobble: cue.variance,
            variation: Variation::Random,
        };
        match self.audio.load_cue(cue.name, def, &refs) {
            Ok(()) => {
                self.loaded.insert(cue.name.to_string());
                true
            }
            Err(error) => {
                log::warn!(
                    "floppy-warriors: cue '{}' failed to load: {error}",
                    cue.name
                );
                self.missing.insert(cue.name.to_string());
                false
            }
        }
    }
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct AudioLevels {
    pub master: f32,
    pub sfx: f32,
    pub music: f32,
}

impl Default for AudioLevels {
    fn default() -> Self {
        Self {
            master: 1.0,
            sfx: 1.0,
            music: 0.8,
        }
    }
}

pub fn drain_cues(world: &mut World) -> Vec<AudioCue> {
    if let Some(queue) = world.get_resource::<CueQueue>() {
        std::mem::take(&mut *queue.0.lock().unwrap())
    } else {
        Vec::new()
    }
}

pub fn push_cue(world: &mut World, cue: AudioCue) {
    if let Some(queue) = world.get_resource::<CueQueue>() {
        queue.0.lock().unwrap().push(cue);
    }
}

pub fn sfx_bus(levels: &AudioLevels) -> f32 {
    (levels.master * levels.sfx).clamp(0.0, 1.0)
}

pub fn pos_of(world: &World, entity: Entity) -> Vec2 {
    world
        .get::<repame_rapier2d::BodySnapshot2d>(entity)
        .map(|s| s.pos)
        .unwrap_or(Vec2::ZERO)
}
