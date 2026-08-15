use super::art::{WarriorArt, tileset_rect};
use super::components::*;
use bevy::prelude::*;
#[cfg(feature = "physics")]
use bevy_rapier2d::prelude::*;

/// Shared arena metrics — title demo + in-game must use these.
pub const GROUND_TOP: f32 = -160.0;
pub const GROUND_HALF_WIDTH: f32 = 1200.0;
pub const GROUND_HALF_HEIGHT: f32 = 100.0;
pub const STAND_Y: f32 = -102.0;
pub const TILE_WORLD: f32 = 32.0;
pub const BG_SIZE: Vec2 = Vec2::new(2400.0, 1440.0);

/// Surface tile (dirt + rounded stone) and fill tile (plain dirt).
const TILE_SURFACE: (u32, u32) = (0, 1);
const TILE_FILL: (u32, u32) = (1, 1);

/// In-game entry point.
pub fn spawn_arena(mut commands: Commands, art: Res<WarriorArt>) {
    spawn_arena_tagged(&mut commands, &art, GameCleanup);
}

/// Shared builder. `tag` is `GameCleanup` or `TitleDemo`.
pub fn spawn_arena_tagged(commands: &mut Commands, art: &WarriorArt, tag: impl Component + Clone) {
    commands.spawn((
        tag.clone(),
        Sprite {
            image: art.bg.clone(),
            color: Color::WHITE,
            custom_size: Some(BG_SIZE),
            ..default()
        },
        Transform::from_xyz(0.0, 40.0, -50.0), // slight lift so horizon sits above ground
    ));

    let ground = Ground {
        top: GROUND_TOP,
        half_width: GROUND_HALF_WIDTH,
        half_height: GROUND_HALF_HEIGHT,
    };

    let mut ground_e = commands.spawn((
        tag.clone(),
        ground,
        // Opaque underlay (reads even if tiles fail to sample).
        Sprite {
            color: Color::srgb(0.72, 0.40, 0.20),
            custom_size: Some(ground.size()),
            ..default()
        },
        Transform::from_xyz(0.0, ground.center_y(), -2.0),
    ));

    #[cfg(feature = "physics")]
    {
        ground_e.insert((
            RigidBody::Fixed,
            Collider::cuboid(ground.half_width, ground.half_height),
            // GROUP_3 matches limb masks in warrior.rs
            CollisionGroups::new(Group::GROUP_3, Group::ALL),
            Friction::coefficient(0.9),
            Restitution::coefficient(0.05),
        ));
    }

    spawn_ground_tiles(commands, art, tag, GROUND_TOP, GROUND_HALF_WIDTH * 2.0);
}

fn spawn_ground_tiles(
    commands: &mut Commands,
    art: &WarriorArt,
    tag: impl Component + Clone,
    top: f32,
    width: f32,
) {
    let tw = TILE_WORLD;
    let cols = (width / tw).ceil() as i32;
    let start_x = -width * 0.5 + tw * 0.5;

    // How many rows of fill below the surface (visual only; collider is thicker).
    let fill_rows = 4;

    let surface_rect = tileset_rect(art.tile_px, TILE_SURFACE.0, TILE_SURFACE.1);
    let fill_rect = tileset_rect(art.tile_px, TILE_FILL.0, TILE_FILL.1);

    for c in 0..cols {
        let x = start_x + c as f32 * tw;

        // Surface row: top of tile flush with GROUND_TOP
        let surface_y = top - tw * 0.5;
        commands.spawn((
            tag.clone(),
            Sprite {
                image: art.tileset.clone(),
                rect: Some(surface_rect),
                custom_size: Some(Vec2::splat(tw)),
                color: Color::WHITE,
                ..default()
            },
            Transform::from_xyz(x, surface_y, -1.0),
        ));

        // Fill under surface
        for r in 1..=fill_rows {
            let y = top - tw * 0.5 - r as f32 * tw;
            commands.spawn((
                tag.clone(),
                Sprite {
                    image: art.tileset.clone(),
                    rect: Some(fill_rect),
                    custom_size: Some(Vec2::splat(tw)),
                    color: Color::WHITE,
                    ..default()
                },
                Transform::from_xyz(x, y, -1.1),
            ));
        }
    }
}
