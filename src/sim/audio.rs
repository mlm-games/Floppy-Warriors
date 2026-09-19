use std::sync::{Arc, Mutex};

use bevy_ecs::prelude::*;
use glam::Vec2;

#[derive(Clone, Debug)]
pub struct AudioCue {
    pub name: &'static str,
    pub volume: f32,
    pub variance: f32,
}

#[derive(Resource, Clone, Default)]
pub struct CueQueue(pub Arc<Mutex<Vec<AudioCue>>>);

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
