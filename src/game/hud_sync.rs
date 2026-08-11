use bevy::prelude::*;
use crate::app::{rarity_accent, RewardCardUi, UiBridge};
use crate::game::components::{EnemyTag, PlayerTag, WarriorRoot};
use crate::game::round_manager::{FINAL_ROUND, RoundManager, RunPhase};

pub fn sync_run_to_ui(
    bridge: Res<UiBridge>,
    rm: Res<RoundManager>,
    warriors: Query<(Entity, &WarriorRoot, Option<&PlayerTag>, Option<&EnemyTag>)>,
) {
    let Ok(mut ui) = bridge.shared.lock() else {
        return;
    };

    ui.run_round = rm.round;
    ui.run_score = rm.score;
    ui.run_phase = match rm.phase {
        RunPhase::Combat => 0,
        RunPhase::Reward => 1,
        RunPhase::GameOver => 2,
    };
    ui.bones_earned = rm.bones_earned;
    ui.victory = rm.victory;
    ui.status_line = match rm.phase {
        RunPhase::Combat => {
            format!(
                "{}: {}  {}: {}",
                ui.translations.get("score").cloned().unwrap_or_else(|| "Score".into()),
                rm.score,
                ui.translations.get("round").cloned().unwrap_or_else(|| "Round".into()),
                rm.round,
            )
        }
        RunPhase::Reward => ui
            .translations
            .get("choose-reward")
            .cloned()
            .unwrap_or_else(|| "Choose a Reward".into()),
        RunPhase::GameOver => {
            if rm.victory {
                ui.translations
                    .get("run-complete")
                    .cloned()
                    .unwrap_or_else(|| "RUN COMPLETE".into())
            } else {
                ui.translations
                    .get("you-lose")
                    .cloned()
                    .unwrap_or_else(|| "YOU LOSE".into())
            }
        }
    };

    ui.end_reason = if rm.phase == RunPhase::GameOver {
        if rm.victory {
            if rm.round > FINAL_ROUND {
                format!("Cleared round {} after first victory", rm.round)
            } else {
                format!("Cleared round {}", rm.round)
            }
        } else {
            format!("Fell on round {}", rm.round)
        }
    } else {
        String::new()
    };

    let mut player: Option<&WarriorRoot> = None;
    let mut active_enemy: Option<&WarriorRoot> = None;
    let mut fallback_enemy: Option<&WarriorRoot> = None;

    for (entity, warrior, is_player, is_enemy) in &warriors {
        if is_player.is_some() {
            player = Some(warrior);
        }

        if is_enemy.is_some() && !warrior.is_dead {
            if Some(entity) == rm.active_enemy {
                active_enemy = Some(warrior);
            } else if fallback_enemy.is_none() {
                fallback_enemy = Some(warrior);
            }
        }
    }

    let enemy = active_enemy.or(fallback_enemy);

    ui.player_hp = player.map(|w| w.health).unwrap_or(0);
    ui.player_max_hp = player.map(|w| w.max_health).unwrap_or(0);
    ui.enemy_hp = enemy.map(|w| w.health).unwrap_or(0);
    ui.enemy_max_hp = enemy.map(|w| w.max_health).unwrap_or(0);

    ui.reward_cards = rm
        .reward_choices
        .iter()
        .map(|r| {
            let stacks = *rm.upgrade_counts.get(r.id).unwrap_or(&0);
            RewardCardUi {
                id: r.id.to_string(),
                title: r.title.to_string(),
                description: r.description.to_string(),
                rarity: r.rarity,
                current_stacks: stacks,
                max_stacks: r.max_stacks,
                category: r.category.to_string(),
                accent_rgb: rarity_accent(r.rarity),
            }
        })
        .collect();
}
