use bevy::prelude::*;
#[cfg(feature = "physics")]
use bevy_rapier2d::prelude::Velocity;

use super::components::*;
use super::round_manager::{RoundManager, RunPhase};

/// Cheap run-loop sanity checks. `debug_assert!` compiles away in release, so
/// this is free shipping and a loud tripwire during development.
pub fn validate_round_manager(rm: Res<RoundManager>, warriors: Query<&WarriorRoot>) {
    if !rm.is_changed() {
        return;
    }

    debug_assert!(
        rm.enemies_remaining <= rm.enemies_total,
        "enemies_remaining {} > enemies_total {}",
        rm.enemies_remaining,
        rm.enemies_total
    );

    debug_assert!(
        !(rm.phase == RunPhase::Reward && rm.reward_choices.is_empty()),
        "Entered reward phase with no reward choices"
    );

    if let Some(player) = rm.player_entity {
        debug_assert!(
            warriors.get(player).is_ok(),
            "RoundManager.player_entity points to missing entity"
        );
    }

    if let Some(enemy) = rm.active_enemy {
        debug_assert!(
            warriors.get(enemy).is_ok(),
            "RoundManager.active_enemy points to missing entity"
        );
    }

    if rm.phase == RunPhase::GameOver {
        debug_assert!(
            rm.player_entity.is_some(),
            "GameOver reached with no player entity tracked"
        );
    }
}

#[cfg(feature = "physics")]
pub fn diag_physics_anomalies(
    limbs: Query<(&WarriorLimb, &Transform, &Velocity)>,
    roots: Query<(Entity, &WarriorRoot, Option<&HitStun>, Option<&Recovery>, Has<RagdollApplied>)>,
) {
    use super::components::*;
    let root_state = roots
        .iter()
        .map(|(e, w, stun, rec, rag)| {
            (
                e,
                w.is_dead,
                stun.map_or(0.0, |s| s.remaining),
                rec.map_or(0.0, |r| r.remaining),
                rag,
            )
        })
        .collect::<Vec<_>>();

    for (limb, tf, vel) in &limbs {
        let sp = vel.linear.length();
        if sp > 600.0 || vel.angular.abs() > 8.0 {
            let (is_dead, stun, rec, rag) = root_state
                .iter()
                .find(|(e, ..)| *e == limb.root)
                .map_or((false, 0.0, 0.0, false), |(_, d, s, r, g)| (*d, *s, *r, *g));
            eprintln!(
                ">>> DIAG root={:?} limb={:?} speed={:.0} angvel={:.1} linvel=({:.0},{:.0}) pos=({:.0},{:.0}) dead={} stun={:.1} rec={:.1} rag={}",
                limb.root, limb.kind, sp, vel.angular, vel.linear.x, vel.linear.y, tf.translation.x, tf.translation.y, is_dead, stun, rec, rag
            );
        }
    }
}

pub fn validate_warriors(q: Query<(Entity, &WarriorRoot)>) {
    for (e, w) in &q {
        debug_assert!(w.max_health >= 1, "Warrior {e:?} max_health < 1");
        debug_assert!(
            w.health <= w.max_health,
            "Warrior {e:?} health > max_health"
        );
        debug_assert!(w.arrow_count >= 1, "Warrior {e:?} arrow_count < 1");
        debug_assert!(w.crit_chance <= 1.0, "Warrior {e:?} crit chance > 1.0");
        debug_assert!(
            w.damage_taken_mult > 0.0,
            "Warrior {e:?} damage_taken_mult <= 0"
        );
    }
}
