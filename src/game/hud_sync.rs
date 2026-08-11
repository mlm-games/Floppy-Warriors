use bevy::prelude::*;
use crate::app::UiBridge;
use crate::game::components::{EnemyTag, PlayerTag, WarriorRoot};
use crate::game::round_manager::{RoundManager, RunPhase};

pub fn sync_run_to_ui(
    bridge: Res<UiBridge>,
    rm: Res<RoundManager>,
    warriors: Query<(&WarriorRoot, Option<&PlayerTag>, Option<&EnemyTag>)>,
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

    let mut player: Option<&WarriorRoot> = None;
    let mut enemy: Option<&WarriorRoot> = None;
    for (w, is_p, is_e) in &warriors {
        if is_p.is_some() {
            player = Some(w);
        }
        if is_e.is_some() {
            enemy = Some(w);
        }
    }
    ui.player_hp = player.map(|w| w.health).unwrap_or(0);
    ui.player_max_hp = player.map(|w| w.max_health).unwrap_or(0);
    ui.enemy_hp = enemy.map(|w| w.health).unwrap_or(0);
    ui.enemy_max_hp = enemy.map(|w| w.max_health).unwrap_or(0);
    ui.reward_titles = rm
        .reward_choices
        .iter()
        .map(|r| r.title.to_string())
        .collect();
    ui.reward_descs = rm
        .reward_choices
        .iter()
        .map(|r| r.description.to_string())
        .collect();
}