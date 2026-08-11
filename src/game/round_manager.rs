use bevy::prelude::*;
use rand::seq::SliceRandom;
use rand::RngExt;
use crate::game::components::*;
use crate::game::enemy_ai::{configure_enemy, EnemySpawnConfig};
use crate::game::meta::{self, apply_meta_to_mods};
use crate::game::warrior::{spawn_warrior, SpawnWarrior};
use crate::save::SaveData;
use game_utils_bevy::save::SaveManager;

pub const FINAL_ROUND: u32 = 15;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RunPhase {
    #[default]
    Combat,
    Reward,
    GameOver,
}

#[derive(Resource, Default)]
pub struct RoundManager {
    pub phase: RunPhase,
    pub round: u32,
    pub enemies_total: u32,
    pub enemies_remaining: u32,
    pub enemies_spawned: u32,
    pub score: u32,
    pub kills: u32,
    pub headshots: u32,
    pub damage_dealt: u32,
    pub victory: bool,
    pub bones_earned: u32,
    pub current_enemy_score: u32,
    pub reward_choices: Vec<RewardDef>,
    pub spawn_cooldown: f32,
    pub player_entity: Option<Entity>,
    pub active_enemy: Option<Entity>,
}

#[derive(Clone, Debug)]
pub struct RewardDef {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
}

const REWARD_POOL: &[RewardDef] = &[
    RewardDef { id: "power", title: "Sharpened Arrows", description: "+25% arrow damage." },
    RewardDef { id: "vitality", title: "Reinforced Body", description: "+25 max HP." },
    RewardDef { id: "quickdraw", title: "Quick Draw", description: "+20% draw speed." },
    RewardDef { id: "velocity", title: "Tighter String", description: "+15% arrow velocity." },
    RewardDef { id: "head_hunter", title: "Head Hunter", description: "+25% headshot damage." },
    RewardDef { id: "quiver", title: "Split Shot", description: "+1 arrow with spread." },
    RewardDef { id: "knockback", title: "Braced Limbs", description: "+30% knockback." },
    RewardDef { id: "field_dressing", title: "Field Dressing", description: "Heal 45% max HP." },
    RewardDef { id: "acrobatics", title: "Acrobatics", description: "-15% airdodge CD." },
];

#[derive(Message, Clone, Debug)]
pub struct HitConfirmed {
    pub shooter: Entity,
    pub target: Entity,
    pub limb: LimbKind,
    pub damage: i32,
    pub killed: bool,
    pub headshot: bool,
}

#[derive(Message, Clone, Debug)]
pub struct ChooseReward(pub &'static str);

#[derive(Component)]
pub struct DyingFade(pub f32);

pub fn begin_run(
    mut rm: ResMut<RoundManager>,
    mut commands: Commands,
    save: Res<SaveData>,
) {
    *rm = RoundManager::default();
    let mut mods = CombatMods::default();
    apply_meta_to_mods(&save, &mut mods);
    let player = spawn_warrior(
        &mut commands,
        SpawnWarrior {
            translation: Vec2::new(-350.0, -80.0),
            team: Team::Player,
            mods,
            base_hp: 100,
        },
    );
    rm.player_entity = Some(player);
    start_next_round(&mut rm);
}

fn start_next_round(rm: &mut RoundManager) {
    rm.round += 1;
    rm.enemies_total = if rm.round % 5 == 0 {
        1
    } else {
        (1 + (rm.round - 1) / 3).min(4)
    };
    rm.enemies_remaining = rm.enemies_total;
    rm.enemies_spawned = 0;
    rm.phase = RunPhase::Combat;
    rm.spawn_cooldown = 0.35;
    rm.active_enemy = None;
}

fn build_enemy_config(round: u32, boss: bool) -> EnemySpawnConfig {
    let mut hp_m = 1.0 + (round - 1) as f32 * 0.14;
    let mut dmg_m = 1.0 + (round - 1) as f32 * 0.055;
    let mut draw_m = 1.0 + ((round - 1) as f32 * 0.025).min(0.45);
    let mut aim = (26.0 - round as f32 * 1.2).max(7.0);
    let mut dmin = (1.6 - round as f32 * 0.04).max(0.75);
    let mut dmax = (3.2 - round as f32 * 0.06).max(1.5);
    if boss {
        hp_m *= 2.25;
        dmg_m *= 1.15;
        draw_m *= 1.1;
        aim *= 0.65;
        dmin *= 0.85;
        dmax *= 0.85;
    }
    EnemySpawnConfig {
        health: (100.0 * hp_m).round() as i32,
        damage_mult: dmg_m,
        draw_speed_mult: draw_m,
        aim_error: aim,
        decision_min: dmin,
        decision_max: dmax,
        boss,
        score_value: if boss { 50 + round * 10 } else { 10 + round * 2 },
    }
}

pub fn spawn_enemies_system(
    time: Res<Time>,
    mut rm: ResMut<RoundManager>,
    mut commands: Commands,
    players: Query<&WarriorRoot, With<PlayerTag>>,
    torso_tf: Query<&GlobalTransform>,
) {
    if rm.phase != RunPhase::Combat || rm.active_enemy.is_some() {
        return;
    }
    if rm.enemies_spawned >= rm.enemies_total {
        return;
    }

    rm.spawn_cooldown -= time.delta_secs();
    if rm.spawn_cooldown > 0.0 {
        return;
    }

    let boss = rm.round > 0 && rm.round % 5 == 0;
    let cfg = build_enemy_config(rm.round, boss);
    rm.current_enemy_score = cfg.score_value;

    let player_x = players
        .single()
        .ok()
        .and_then(|w| torso_tf.get(w.torso).ok())
        .map(|tf| tf.translation().x)
        .unwrap_or(-350.0);

    let spawn_x = if player_x < 0.0 {
        rand::rng().random_range(250.0..480.0)
    } else {
        rand::rng().random_range(-480.0..-250.0)
    };

    let enemy = spawn_warrior(
        &mut commands,
        SpawnWarrior {
            translation: Vec2::new(spawn_x, -80.0),
            team: Team::Enemy,
            mods: CombatMods {
                damage_mult: cfg.damage_mult,
                draw_speed_mult: cfg.draw_speed_mult,
                ..default()
            },
            base_hp: cfg.health,
        },
    );

    let mut ai = EnemyAi {
        aim_error: cfg.aim_error,
        decision_min: cfg.decision_min,
        decision_max: cfg.decision_max,
        decision_timer: 0.5,
        draw_timer: 0.0,
        drawing: false,
        aim_offset: Vec2::ZERO,
        target: rm.player_entity,
    };
    configure_enemy(&mut ai, &cfg);
    commands.entity(enemy).insert(ai);

    rm.enemies_spawned += 1;
    rm.active_enemy = Some(enemy);
}

pub fn on_hits(
    mut hits: MessageReader<HitConfirmed>,
    mut rm: ResMut<RoundManager>,
    mut commands: Commands,
    mut warriors: Query<&mut WarriorRoot>,
) {
    for h in hits.read() {
        if let Some(player) = rm.player_entity {
            if h.shooter == player {
                rm.damage_dealt += h.damage as u32;
                if h.headshot {
                    rm.headshots += 1;
                }
            }
        }
        if h.killed {
            if let Ok(w) = warriors.get(h.target) {
                if w.team == Team::Enemy {
                    rm.kills += 1;
                    rm.score += rm.current_enemy_score;
                    rm.enemies_remaining = rm.enemies_remaining.saturating_sub(1);
                    rm.active_enemy = None;
                    if rm.enemies_remaining == 0 {
                        if rm.round >= FINAL_ROUND {
                            end_run(&mut rm, true);
                        } else {
                            let player = rm.player_entity;
                            enter_reward(&mut rm, &mut warriors, player);
                        }
                    } else {
                        rm.spawn_cooldown = 1.0;
                    }
                } else if w.team == Team::Player {
                    end_run(&mut rm, false);
                }
            }
            commands.entity(h.target).insert(DyingFade(2.0));
        }
    }
}

fn enter_reward(
    rm: &mut RoundManager,
    warriors: &mut Query<&mut WarriorRoot>,
    player: Option<Entity>,
) {
    rm.phase = RunPhase::Reward;
    if let Some(p) = player {
        if let Ok(mut w) = warriors.get_mut(p) {
            let heal = ((w.max_health as f32) * 0.12).round().max(5.0) as i32;
            w.health = (w.health + heal).min(w.max_health);
        }
    }
    let mut pool: Vec<RewardDef> = REWARD_POOL.to_vec();
    pool.shuffle(&mut rand::rng());
    rm.reward_choices = pool.into_iter().take(3).collect();
}

fn end_run(rm: &mut RoundManager, victory: bool) {
    rm.phase = RunPhase::GameOver;
    rm.victory = victory;
    if victory {
        rm.score += 250;
    }
}

pub fn apply_reward_system(
    mut events: MessageReader<ChooseReward>,
    mut rm: ResMut<RoundManager>,
    mut warriors: Query<(&mut WarriorRoot, Option<&mut Airdodge>), With<PlayerTag>>,
) {
    for ChooseReward(id) in events.read() {
        if rm.phase != RunPhase::Reward {
            continue;
        }
        if !rm.reward_choices.iter().any(|r| r.id == *id) {
            continue;
        }
        if let Ok((mut w, ad)) = warriors.single_mut() {
            match *id {
                "power" => w.damage_mult *= 1.25,
                "vitality" => {
                    w.max_health += 25;
                    w.health += 25;
                }
                "quickdraw" => w.draw_speed_mult *= 1.2,
                "velocity" => w.velocity_mult *= 1.15,
                "head_hunter" => w.headshot_mult *= 1.25,
                "quiver" => {
                    w.arrow_count = (w.arrow_count + 1).min(5);
                    w.spread_deg += 4.0;
                }
                "knockback" => w.knockback_mult *= 1.3,
                "field_dressing" => {
                    let h = ((w.max_health as f32) * 0.45).round() as i32;
                    w.health = (w.health + h).min(w.max_health);
                }
                "acrobatics" => {
                    if let Some(mut a) = ad {
                        a.cooldown = (a.cooldown * 0.85).max(0.35);
                    }
                }
                _ => {}
            }
        }
        rm.reward_choices.clear();
        start_next_round(&mut rm);
    }
}

pub fn finalize_game_over_bones(
    mut rm: ResMut<RoundManager>,
    mut save: ResMut<SaveData>,
    manager: Res<SaveManager>,
    mut done: Local<bool>,
) {
    if rm.phase != RunPhase::GameOver {
        *done = false;
        return;
    }
    if *done {
        return;
    }
    *done = true;
    let earned = meta::calculate_run_bones(
        &save,
        rm.victory,
        rm.round,
        rm.score,
        rm.kills,
        rm.headshots,
    );
    rm.bones_earned = earned;
    save.bones += earned;
    save.total_runs += 1;
    save.total_kills += rm.kills;
    save.best_round = save.best_round.max(rm.round);
    if rm.victory {
        save.total_victories += 1;
    }
    save.high_score = save.high_score.max(rm.score);
    save.last_played_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let _ = manager.save(&*save);
}