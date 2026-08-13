use super::components::*;
use bevy::prelude::*;
#[cfg(feature = "physics")]
use bevy_rapier2d::prelude::*;

pub fn spawn_arena(mut commands: Commands) {
    // Backdrop
    commands.spawn((
        GameCleanup,
        Sprite {
            color: Color::srgb(0.45, 0.66, 0.79),
            custom_size: Some(Vec2::new(2000.0, 1200.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -50.0),
    ));
    // Ground
    let mut ground = commands.spawn((
        GameCleanup,
        Ground,
        Sprite {
            color: Color::srgb(0.25, 0.28, 0.22),
            custom_size: Some(Vec2::new(2400.0, 80.0)),
            ..default()
        },
        Transform::from_xyz(0.0, -200.0, -1.0),
    ));
    #[cfg(feature = "physics")]
    {
        ground.insert((
            RigidBody::Fixed,
            Collider::cuboid(1200.0, 40.0),
            CollisionGroups::new(Group::GROUP_3, Group::ALL),
        ));
    }
}
