use bevy::prelude::*;
use bevy::window::PrimaryWindow;
#[cfg(feature = "physics")]
use bevy_rapier2d::prelude::*;

use super::audio_fx::{self, CombatSfx};
use super::components::*;
use super::round_manager::{RoundManager, RunPhase};

pub fn player_aim_and_bow(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    phase: Res<RoundManager>,
    asset_server: Res<AssetServer>,
    sfx: Res<CombatSfx>,
    mut commands: Commands,
    mut players: Query<
        (Entity, &WarriorRoot, &mut BowState, Option<&mut Airdodge>),
        With<PlayerTag>,
    >,
    torso_tf: Query<&GlobalTransform>,
    mut bow_tf: Query<&mut Transform>,
    #[cfg(feature = "physics")] mut torso_physics: Query<(&mut ExternalImpulse, &mut Velocity)>,
    #[cfg(not(feature = "physics"))] mut root_tf: Query<&mut Transform>,
) {
    if phase.phase != RunPhase::Combat {
        return;
    }

    let Ok((camera, camera_tf)) = camera_q.single() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };

    for (entity, warrior, mut bow, airdodge) in &mut players {
        if warrior.is_dead {
            continue;
        }

        let Ok(torso_global) = torso_tf.get(warrior.torso) else {
            continue;
        };
        let origin = torso_global.translation().truncate();

        let mouse_target = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world_2d(camera_tf, cursor).ok());

        let stick = Vec2::new(
            keys.pressed(KeyCode::ArrowRight) as i32 as f32
                - keys.pressed(KeyCode::ArrowLeft) as i32 as f32,
            keys.pressed(KeyCode::ArrowUp) as i32 as f32
                - keys.pressed(KeyCode::ArrowDown) as i32 as f32,
        );

        let target = if stick.length_squared() > 0.04 {
            origin + stick.normalize() * 120.0
        } else {
            mouse_target.unwrap_or(origin + Vec2::X * 100.0)
        };

        let angle = (target - origin).y.atan2((target - origin).x);

        let torso_angle = {
            let right = (torso_global.compute_transform().rotation * Vec3::X).truncate();
            right.y.atan2(right.x)
        };

        let local_angle = angle - torso_angle;

        if let Ok(mut local_bow_tf) = bow_tf.get_mut(warrior.bow_pivot) {
            local_bow_tf.rotation = Quat::from_rotation_z(local_angle);
        }

        if mouse.just_pressed(MouseButton::Left) || keys.just_pressed(KeyCode::Space) {
            bow.drawing = true;
            bow.draw_power = 0.0;
            audio_fx::play_sfx(&mut commands, &asset_server, &sfx.bow_draw, 0.3, 0.05);
        }

        if bow.drawing {
            let speed = 160.0 * warrior.draw_speed_mult.max(0.1);
            bow.draw_power = (bow.draw_power + speed * time.delta_secs()).min(100.0);
        }

        if (mouse.just_released(MouseButton::Left) || keys.just_released(KeyCode::Space))
            && bow.drawing
        {
            let spawn_origin = origin + Vec2::from_angle(angle) * 36.0;

            super::warrior::fire_from_bow_angled(
                &mut commands,
                &asset_server,
                &sfx,
                entity,
                warrior,
                &bow,
                spawn_origin,
                angle,
            );
            bow.drawing = false;
            bow.draw_power = 0.0;
        }

        if let Some(mut dodge) = airdodge {
            dodge.remaining = (dodge.remaining - time.delta_secs()).max(0.0);

            let dodge_pressed =
                keys.just_pressed(KeyCode::ShiftLeft) || keys.just_pressed(KeyCode::ShiftRight);

            if dodge_pressed && dodge.remaining <= 0.0 {
                dodge.remaining = dodge.cooldown;

                // Recovery window: hover is suspended so the launch burst is
                // not immediately cancelled by vertical motor correction.
                commands.entity(entity).insert(Recovery { remaining: 0.35 });

                #[cfg(feature = "physics")]
                {
                    if let Ok((mut impulse, mut velocity)) = torso_physics.get_mut(warrior.torso) {
                        impulse.impulse += Vec2::Y * dodge.impulse;
                        velocity.linear.y = velocity.linear.y.max(dodge.impulse * 0.9);
                    }
                }

                #[cfg(not(feature = "physics"))]
                {
                    if let Ok(mut tf) = root_tf.get_mut(entity) {
                        tf.translation.y += 18.0;
                    }
                }
            }
        }
    }
}
