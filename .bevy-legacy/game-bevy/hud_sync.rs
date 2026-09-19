use crate::app::{RewardCardUi, UiBridge, rarity_accent};
use crate::game::components::{BowState, EnemyTag, PlayerTag, WarriorRoot};
use crate::game::round_manager::{FINAL_ROUND, RoundManager, RunPhase};
use bevy::prelude::*;

const BOSS_BANNER_SEC: f32 = 2.5;

pub fn sync_run_to_ui(
    mut last_banner_round: Local<u32>,
    bridge: Res<UiBridge>,
    rm: Res<RoundManager>,
    time: Res<Time>,
    warriors: Query<(Entity, &WarriorRoot, Option<&PlayerTag>, Option<&EnemyTag>)>,
    player_bow: Query<&BowState, With<PlayerTag>>,
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
        RunPhase::Combat => String::new(),
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

    // A fresh run rolls the round counter back below the last bannered round,
    // so reset the tracker before the next identical boss round can fire again.
    if rm.round < *last_banner_round {
        *last_banner_round = 0;
    }
    if rm.round != 0 && rm.round.is_multiple_of(5) && *last_banner_round != rm.round {
        *last_banner_round = rm.round;
        ui.boss_banner_timer = BOSS_BANNER_SEC;
    }
    if ui.boss_banner_timer > 0.0 {
        ui.boss_banner_timer = (ui.boss_banner_timer - time.delta_secs()).max(0.0);
    }

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

    if let Ok(bow) = player_bow.single() {
        ui.drawing = bow.drawing && rm.phase == RunPhase::Combat;
        ui.draw_power = if ui.drawing { bow.draw_power } else { 0.0 };
    } else {
        ui.drawing = false;
        ui.draw_power = 0.0;
    }

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
