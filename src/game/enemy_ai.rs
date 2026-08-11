use bevy::prelude::*;
use rand::RngExt;

use super::components::*;
use super::round_manager::{RoundManager, RunPhase};

#[derive(Clone, Debug)]
pub struct EnemySpawnConfig {
    pub health: i32,
    pub damage_mult: f32,
    pub draw_speed_mult: f32,
    pub aim_error: f32,
    pub decision_min: f32,
    pub decision_max: f32,
    pub boss: bool,
    pub score_value: u32,
    pub archetype: EnemyArchetype,
}

pub fn configure_enemy(ai: &mut EnemyAi, cfg: &EnemySpawnConfig) {
    ai.aim_error = cfg.aim_error;
    ai.decision_min = cfg.decision_min;
    ai.decision_max = cfg.decision_max;
    ai.decision_timer = rand::rng().random_range(ai.decision_min..ai.decision_max);
    ai.drawing = false;
    ai.draw_timer = 0.0;
}

pub fn enemy_ai_system(
    time: Res<Time>,
    phase: Res<RoundManager>,
    asset_server: Res<AssetServer>,
    sfx: Res<super::audio_fx::CombatSfx>,
    mut commands: Commands,
    mut enemies: Query<(Entity, &mut WarriorRoot, &mut BowState, &mut EnemyAi), With<EnemyTag>>,
    players: Query<(Entity, &WarriorRoot, &GlobalTransform), (With<PlayerTag>, Without<EnemyTag>)>,
    torso_tf: Query<&GlobalTransform>,
    mut bow_tf: Query<&mut Transform>,
) {
    if phase.phase != RunPhase::Combat {
        return;
    }

    let Ok((player_e, player_w, _)) = players.single() else {
        return;
    };
    if player_w.is_dead {
        return;
    }

    let Ok(pt) = torso_tf.get(player_w.torso) else {
        return;
    };
    let player_pos = pt.translation().truncate();

    for (enemy_entity, warrior, mut bow, mut ai) in &mut enemies {
        if warrior.is_dead {
            continue;
        }

        ai.target = Some(player_e);

        let Ok(enemy_torso_tf) = torso_tf.get(warrior.torso) else {
            continue;
        };
        let origin = enemy_torso_tf.translation().truncate();

        let target = player_pos + ai.aim_offset;
        let angle = (target - origin).y.atan2((target - origin).x);

        let torso_angle = {
            let right = (enemy_torso_tf.compute_transform().rotation * Vec3::X).truncate();
            right.y.atan2(right.x)
        };

        let local_angle = angle - torso_angle;

        if let Ok(mut local_bow_tf) = bow_tf.get_mut(warrior.bow_pivot) {
            local_bow_tf.rotation = Quat::from_rotation_z(local_angle);
        }

        if ai.drawing {
            let speed = 160.0 * warrior.draw_speed_mult.max(0.1);
            bow.drawing = true;
            bow.draw_power = (bow.draw_power + speed * time.delta_secs()).min(100.0);
            ai.draw_timer -= time.delta_secs();

            if ai.draw_timer <= 0.0 {
                let spawn_origin = origin + Vec2::from_angle(angle) * 36.0;

                super::warrior::fire_from_bow_angled(
                    &mut commands,
                    &asset_server,
                    &sfx,
                    enemy_entity,
                    &*warrior,
                    &bow,
                    spawn_origin,
                    angle,
                );
                bow.drawing = false;
                bow.draw_power = 0.0;
                ai.drawing = false;
                ai.decision_timer = rand::rng().random_range(ai.decision_min..ai.decision_max);
            }
        } else {
            ai.decision_timer -= time.delta_secs();
            if ai.decision_timer <= 0.0 {
                ai.aim_offset = Vec2::new(
                    rand::rng().random_range(-ai.aim_error..ai.aim_error),
                    rand::rng().random_range(-ai.aim_error..ai.aim_error),
                );
                ai.drawing = true;
                bow.drawing = true;
                bow.draw_power = 0.0;

                let full_draw = 100.0 / (160.0 * warrior.draw_speed_mult.max(0.1));
                ai.draw_timer =
                    rand::rng().random_range((full_draw * 0.45).min(0.45)..full_draw * 0.9);
            }
        }
    }
}
