use bevy::prelude::*;
use rand::RngExt;
use super::components::*;
use super::round_manager::{RunPhase, RoundManager};
use super::warrior::fire_from_bow;

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
    mut commands: Commands,
    mut enemies: Query<(Entity, &mut WarriorRoot, &mut BowState, &mut EnemyAi), With<EnemyTag>>,
    players: Query<
        (Entity, &WarriorRoot, &GlobalTransform),
        (With<PlayerTag>, Without<EnemyTag>),
    >,
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
    for (e, w, mut bow, mut ai) in &mut enemies {
        if w.is_dead {
            continue;
        }
        ai.target = Some(player_e);
        // Aim with error offset
        let Ok(et) = torso_tf.get(w.torso) else {
            continue;
        };
        let origin = et.translation().truncate();
        let target = player_pos + ai.aim_offset;
        let angle = (target - origin).y.atan2((target - origin).x);
        if let Ok(mut bt) = bow_tf.get_mut(w.bow_pivot) {
            bt.rotation = Quat::from_rotation_z(angle);
        }
        if ai.drawing {
            let speed = 160.0 * w.draw_speed_mult.max(0.1);
            bow.drawing = true;
            bow.draw_power = (bow.draw_power + speed * time.delta_secs()).min(100.0);
            ai.draw_timer -= time.delta_secs();
            if ai.draw_timer <= 0.0 {
                if let Ok(gt) = torso_tf.get(w.bow_pivot) {
                    fire_from_bow(&mut commands, e, &w, &bow, gt);
                }
                bow.drawing = false;
                bow.draw_power = 0.0;
                ai.drawing = false;
                ai.decision_timer =
                    rand::rng().random_range(ai.decision_min..ai.decision_max);
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
                let full = 100.0 / (160.0 * w.draw_speed_mult.max(0.1));
                ai.draw_timer = rand::rng().random_range((full * 0.45).min(0.45)..full * 0.9);
            }
        }
    }
}