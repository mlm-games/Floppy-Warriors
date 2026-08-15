use super::art::WarriorArt;
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

const TILE_SURFACE_ROW: u32 = 4;
const TILE_FILL_ROWS: [u32; 2] = [5, 6];
const TILE_FILL_DARK_ROW: u32 = 7;

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
            // Darkened as it is too bright.
            color: Color::srgb(0.28, 0.28, 0.28),
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
            color: Color::srgb(0.40, 0.22, 0.12),
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

    let strip_px = 4.0 * art.tile_px as f32;
    let strip_rect = |row: u32| {
        let p = art.tile_px as f32;
        Rect::from_corners(
            Vec2::new(0.0, row as f32 * p),
            Vec2::new(strip_px, (row + 1) as f32 * p),
        )
    };
    let surface_rect = strip_rect(TILE_SURFACE_ROW);
    let fill_light_rect = strip_rect(TILE_FILL_ROWS[0]);
    let fill_dark_rect = strip_rect(TILE_FILL_DARK_ROW);

    let mut c = 0i32;
    while c < cols {
        let n = (cols - c).min(4);
        let x = start_x + (c as f32 + (n as f32 - 1.0) * 0.5) * tw;
        let size = Vec2::new(n as f32 * tw, tw);

        // Surface row: top of strip flush with GROUND_TOP
        let surface_y = top - tw * 0.5;
        commands.spawn((
            tag.clone(),
            Sprite {
                image: art.tileset.clone(),
                rect: Some(surface_rect),
                custom_size: Some(size),
                color: Color::srgb(0.5, 0.5, 0.5),
                ..default()
            },
            Transform::from_xyz(x, surface_y, -1.0),
        ));

        // Fill under surface; alternate light cap rows, dark strip at the bottom.
        for r in 1..=fill_rows {
            let rect = if r == fill_rows {
                fill_dark_rect
            } else if r % 2 == 0 {
                strip_rect(TILE_FILL_ROWS[1])
            } else {
                fill_light_rect
            };
            let y = top - tw * 0.5 - r as f32 * tw;
            commands.spawn((
                tag.clone(),
                Sprite {
                    image: art.tileset.clone(),
                    rect: Some(rect),
                    custom_size: Some(size),
                    color: Color::srgb(0.5, 0.5, 0.5),
                    ..default()
                },
                Transform::from_xyz(x, y, -1.1),
            ));
        }
        c += n;
    }
}
