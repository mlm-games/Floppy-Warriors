use bevy::prelude::*;

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

pub fn validate_warriors(q: Query<(Entity, &WarriorRoot)>) {
    for (e, w) in &q {
        debug_assert!(w.max_health >= 1, "Warrior {e:?} max_health < 1");
        debug_assert!(w.health <= w.max_health, "Warrior {e:?} health > max_health");
        debug_assert!(w.arrow_count >= 1, "Warrior {e:?} arrow_count < 1");
        debug_assert!(w.crit_chance <= 1.0, "Warrior {e:?} crit chance > 1.0");
        debug_assert!(
            w.damage_taken_mult > 0.0,
            "Warrior {e:?} damage_taken_mult <= 0"
        );
    }
}
