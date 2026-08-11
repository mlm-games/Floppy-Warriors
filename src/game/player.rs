use bevy::prelude::*;
use bevy::window::PrimaryWindow;
#[cfg(feature = "physics")]
use bevy_rapier2d::prelude::*;
use super::components::*;
use super::round_manager::{RunPhase, RoundManager};

pub fn player_aim_and_bow(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    phase: Res<RoundManager>,
    mut commands: Commands,
    mut players: Query<(
        Entity,
        &mut WarriorRoot,
        &mut BowState,
        Option<&mut Airdodge>,
    ), With<PlayerTag>>,
    torso_tf: Query<&GlobalTransform>,
    mut bow_tf: Query<&mut Transform>,
    #[cfg(feature = "physics")] mut impulses: Query<&mut ExternalImpulse>,
) {
    if phase.phase != RunPhase::Combat {
        return;
    }
    let Ok((cam, cam_tf)) = camera_q.single() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    for (e, w, mut bow, airdodge) in &mut players {
        if w.is_dead {
            continue;
        }
        // Aim
        let Ok(torso_gt) = torso_tf.get(w.torso) else {
            continue;
        };
        let origin = torso_gt.translation().truncate();
        let mut aim = None;
        if let Some(cursor) = window.cursor_position() {
            if let Ok(world) = cam.viewport_to_world_2d(cam_tf, cursor) {
                aim = Some(world);
            }
        }
        // stick fallback
        let stick = Vec2::new(
            keys.pressed(KeyCode::ArrowRight) as i32 as f32
                - keys.pressed(KeyCode::ArrowLeft) as i32 as f32,
            keys.pressed(KeyCode::ArrowUp) as i32 as f32
                - keys.pressed(KeyCode::ArrowDown) as i32 as f32,
        );
        let target = if stick.length() > 0.2 {
            origin + stick.normalize() * 100.0
        } else {
            aim.unwrap_or(origin + Vec2::X * 100.0)
        };
        let angle = (target - origin).y.atan2((target - origin).x);
        if let Ok(mut bt) = bow_tf.get_mut(w.bow_pivot) {
            bt.rotation = Quat::from_rotation_z(angle);
        }
        // Draw / release
        if mouse.just_pressed(MouseButton::Left) || keys.just_pressed(KeyCode::Space) {
            bow.drawing = true;
            bow.draw_power = 0.0;
        }
        if bow.drawing {
            let speed = 160.0 * w.draw_speed_mult.max(0.1);
            bow.draw_power = (bow.draw_power + speed * time.delta_secs()).min(100.0);
        }
        if (mouse.just_released(MouseButton::Left) || keys.just_released(KeyCode::Space))
            && bow.drawing
        {
            if let Ok(gt) = torso_tf.get(w.bow_pivot) {
                super::warrior::fire_from_bow(&mut commands, e, &w, &bow, gt);
            }
            bow.drawing = false;
            bow.draw_power = 0.0;
        }
        // Airdodge
        if let Some(mut ad) = airdodge {
            ad.remaining = (ad.remaining - time.delta_secs()).max(0.0);
            if keys.just_pressed(KeyCode::ShiftLeft) && ad.remaining <= 0.0 {
                ad.remaining = ad.cooldown;
                #[cfg(feature = "physics")]
                {
                    if let Ok(mut imp) = impulses.get_mut(w.torso) {
                        imp.impulse += Vec2::Y * ad.impulse;
                    }
                }
            }
        }
    }
}