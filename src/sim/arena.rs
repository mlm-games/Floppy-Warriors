use bevy_ecs::prelude::*;
use glam::Vec2;

use super::comps::*;

pub const GROUND_TOP: f32 = -160.0;
pub const GROUND_HALF_WIDTH: f32 = 1200.0;
pub const GROUND_HALF_HEIGHT: f32 = 100.0;
pub const STAND_Y: f32 = -102.0;
pub const TILE_WORLD: f32 = 32.0;
pub const BG_SIZE: [f32; 2] = [2400.0, 1440.0];
pub const TILESET_COLS: u32 = 32;
pub const TILESET_ROWS: u32 = 16;
pub const TILE_SURFACE_ROW: u32 = 4;
pub const TILE_FILL_ROWS: [u32; 2] = [5, 6];
pub const TILE_FILL_DARK_ROW: u32 = 7;

#[derive(Component)]
pub struct GroundTag;

pub fn spawn_ground(world: &mut World) {
    world.spawn((
        GroundTag,
        repame_rapier2d::RapierBody2d {
            kind: repame_rapier2d::BodyKind::Fixed,
            spawn_pos: Vec2::new(0.0, GROUND_TOP - GROUND_HALF_HEIGHT),
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
    let fading: Vec<(Entity, Vec2)> = world
        .query::<(Entity, &super::state::DyingFade, &Arrow)>()
        .iter(world)
        .map(|(e, _, a)| (e, a.pos))
        .collect();
    for (e, p) in fading {
        if p.x < MIN_X || p.x > MAX_X || p.y < MIN_Y || p.y > MAX_Y {
            world.despawn(e);
        }
    }
}
