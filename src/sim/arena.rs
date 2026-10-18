use bevy_ecs::prelude::*;
use glam::Vec2;

use super::comps::*;
use super::rounds::RunState;

pub const GROUND_TOP: f32 = -160.0;
pub const GROUND_HALF_WIDTH: f32 = 1200.0;
pub const GROUND_HALF_HEIGHT: f32 = 100.0;
pub const STAND_Y: f32 = GROUND_TOP - 58.5;
pub const TILE_WORLD: f32 = 32.0;
pub const BG_SIZE: [f32; 2] = [2400.0, 1440.0];
pub const WORLD_VIEW: [f32; 2] = [1200.0, 720.0];
pub const CAMERA_HOME: Vec2 = Vec2::new(50.0, -328.0);
pub const TILESET_COLS: u32 = 32;
pub const TILESET_ROWS: u32 = 16;
pub const TILE_SURFACE_ROW: u32 = 4;
pub const TILE_COLUMN: u32 = 4;
pub const TILE_FILL_ROWS: [u32; 2] = [5, 6];
pub const TILE_FILL_DARK_ROW: u32 = 7;

#[derive(Component)]
pub struct GroundTag;

/// Visible world extent, republished by the view each frame as the camera
/// fit scales [`WORLD_VIEW`] to the window. Gameplay placements scale with
/// it, so they stay on screen at any density or window size.
#[derive(Resource, Clone, Copy, Debug)]
pub struct ViewExtent {
    pub half_width: f32,
    pub half_height: f32,
}

impl Default for ViewExtent {
    fn default() -> Self {
        Self {
            half_width: BASE_HALF_WIDTH,
            half_height: BASE_HALF_HEIGHT,
        }
    }
}

pub const BASE_HALF_WIDTH: f32 = WORLD_VIEW[0] * 0.5;
pub const BASE_HALF_HEIGHT: f32 = WORLD_VIEW[1] * 0.5;

pub fn spawn_ground(world: &mut World) {
    world.spawn((
        GroundTag,
        repame_rapier2d::RapierBody2d {
            kind: repame_rapier2d::BodyKind::Fixed,
            spawn_pos: Vec2::new(0.0, GROUND_TOP + GROUND_HALF_HEIGHT),
            half_extents: Vec2::new(GROUND_HALF_WIDTH, GROUND_HALF_HEIGHT),
            memberships: 0x0004,
            filter: 0xFFFF_FFFF,
            ..Default::default()
        },
    ));
}

pub fn cleanup_far_entities(world: &mut World) {
    const MAX_X: f32 = 3000.0;
    const MIN_X: f32 = -3000.0;
    const MAX_Y: f32 = 2000.0;
    const MIN_Y: f32 = -3000.0;
    let arrows: Vec<(Entity, Vec2)> = world
        .query::<(Entity, &Arrow)>()
        .iter(world)
        .map(|(e, a)| (e, a.pos))
        .collect();
    for (e, p) in arrows {
        if p.x < MIN_X || p.x > MAX_X || p.y < MIN_Y || p.y > MAX_Y {
            world.despawn(e);
        }
    }
    let warriors: Vec<(Entity, Entity)> = world
        .query::<(Entity, &WarriorRoot)>()
        .iter(world)
        .map(|(entity, warrior)| (entity, warrior.torso))
        .collect();
    for (entity, torso) in warriors {
        let Some(position) = world.get::<repame_rapier2d::BodySnapshot2d>(torso) else {
            continue;
        };
        let position = position.pos;
        if position.x < MIN_X || position.x > MAX_X || position.y < MIN_Y || position.y > MAX_Y {
            let enemy_removed = {
                let mut rm = world.resource_mut::<RunState>();
                let player_removed = rm.player == Some(entity);
                let enemy_removed = rm.active_enemy == Some(entity);
                if player_removed {
                    rm.player = None;
                    rm.phase = super::rounds::RunPhase::GameOver;
                }
                if enemy_removed {
                    rm.active_enemy = None;
                    rm.enemies_remaining = rm.enemies_remaining.saturating_sub(1);
                }
                enemy_removed
            };
            super::warrior::despawn_warrior(world, entity);
            if enemy_removed && world.resource::<RunState>().enemies_remaining == 0 {
                super::rounds::enter_reward(world);
            }
        }
    }
}
