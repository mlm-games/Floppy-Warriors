use std::collections::HashMap;

use crate::game::art::WarriorArt;
use crate::game::components::*;
use crate::game::enemy_ai::{EnemySpawnConfig, configure_enemy};
use crate::game::meta::{self, apply_meta_to_mods};
use crate::game::warrior::{SpawnWarrior, spawn_warrior};
use crate::save::SaveData;
use bevy::prelude::*;
use game_utils_bevy::save::SaveManager;
use rand::RngExt;

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
    pub loop_count: u32,
    pub upgrade_counts: HashMap<&'static str, u32>,
    pub victory_claimed: bool,
}

#[derive(Clone, Debug)]
pub struct RewardDef {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub rarity: u8,
    pub category: &'static str,
    pub max_stacks: u32,
    pub min_round: u32,
    pub weight: u32,
}

const REWARD_POOL: &[RewardDef] = &[
    RewardDef {
        id: "power",
        title: "Sharpened Arrows",
        description: "+25% arrow damage.",
        rarity: 1,
        category: "damage",
        max_stacks: 8,
        min_round: 1,
        weight: 12,
    },
    RewardDef {
        id: "vitality",
        title: "Reinforced Body",
        description: "+25 max HP.",
        rarity: 1,
        category: "defense",
        max_stacks: 6,
        min_round: 1,
        weight: 10,
    },
    RewardDef {
        id: "quickdraw",
        title: "Quick Draw",
        description: "+20% draw speed.",
        rarity: 1,
        category: "utility",
        max_stacks: 5,
        min_round: 1,
        weight: 9,
    },
    RewardDef {
        id: "velocity",
        title: "Tighter String",
        description: "+15% arrow velocity.",
        rarity: 1,
        category: "utility",
        max_stacks: 5,
        min_round: 1,
        weight: 8,
    },
    RewardDef {
        id: "head_hunter",
        title: "Head Hunter",
        description: "+25% headshot damage.",
        rarity: 1,
        category: "damage",
        max_stacks: 5,
        min_round: 1,
        weight: 8,
    },
    RewardDef {
        id: "quiver",
        title: "Split Shot",
        description: "+1 arrow with spread.",
        rarity: 2,
        category: "utility",
        max_stacks: 4,
        min_round: 2,
        weight: 7,
    },
    RewardDef {
        id: "knockback",
        title: "Braced Limbs",
        description: "+30% knockback.",
        rarity: 1,
        category: "utility",
        max_stacks: 4,
        min_round: 1,
        weight: 7,
    },
    RewardDef {
        id: "field_dressing",
        title: "Field Dressing",
        description: "Heal 45% max HP.",
        rarity: 2,
        category: "defense",
        max_stacks: 999,
        min_round: 1,
        weight: 6,
    },
    RewardDef {
        id: "crit",
        title: "Lucky Fletching",
        description: "+12% crit chance. Crits deal 2x damage.",
        rarity: 2,
        category: "damage",
        max_stacks: 5,
        min_round: 2,
        weight: 7,
    },
    RewardDef {
        id: "leech",
        title: "Bone Leech",
        description: "Heal 8 HP whenever you kill an enemy.",
        rarity: 3,
        category: "defense",
        max_stacks: 5,
        min_round: 3,
        weight: 6,
    },
    RewardDef {
        id: "glass",
        title: "Glass Cannon",
        description: "+75% damage, but lose 20 max HP.",
        rarity: 3,
        category: "damage",
        max_stacks: 2,
        min_round: 4,
        weight: 4,
    },
    RewardDef {
        id: "second_heart",
        title: "Second Heart",
        description: "Revive once at 50% HP.",
        rarity: 4,
        category: "defense",
        max_stacks: 1,
        min_round: 5,
        weight: 3,
    },
    RewardDef {
        id: "last_stand",
        title: "Last Stand",
        description: "+75% damage while below 35% HP.",
        rarity: 4,
        category: "damage",
        max_stacks: 1,
        min_round: 6,
        weight: 4,
    },
    RewardDef {
        id: "giant_arrows",
        title: "Giant Arrows",
        description: "+45% damage and knockback, -15% velocity.",
        rarity: 2,
        category: "damage",
        max_stacks: 3,
        min_round: 3,
        weight: 5,
    },
    RewardDef {
        id: "needlepoint",
        title: "Needlepoint",
        description: "+30% velocity and +20% headshot damage.",
        rarity: 2,
        category: "damage",
        max_stacks: 4,
        min_round: 3,
        weight: 6,
    },
];

#[derive(Message, Clone, Debug)]
pub struct HitConfirmed {
    pub shooter: Entity,
    pub target: Entity,
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
    textures: Res<WarriorArt>,
) {
    *rm = RoundManager::default();
    let mut mods = CombatMods::default();
    apply_meta_to_mods(&save, &mut mods);
    let player = spawn_warrior(
        &mut commands,
        &textures,
        SpawnWarrior {
            translation: Vec2::new(-350.0, -112.0),
            team: Team::Player,
            mods,
            base_hp: 100,
            scale: 1.0,
        },
    );
    rm.player_entity = Some(player);
    start_next_round(&mut rm);
}

fn start_next_round(rm: &mut RoundManager) {
    rm.round += 1;
    // Boss pacing: rounds 5-29 = 1 boss, 30-59 = 2, 60+ = 3 bosses.
    rm.enemies_total = if rm.round.is_multiple_of(5) {
        1 + rm.round / 30
    } else {
        (1 + (rm.round - 1) / 3).min(8)
    };
    rm.enemies_remaining = rm.enemies_total;
    rm.enemies_spawned = 0;
    rm.phase = RunPhase::Combat;
    rm.spawn_cooldown = 0.35;
    rm.active_enemy = None;
}

fn build_enemy_config(round: u32, boss: bool) -> EnemySpawnConfig {
    let archetype = choose_enemy_archetype(round, boss);

    let mut hp_m = 1.0 + (round - 1) as f32 * 0.14;
    let mut dmg_m = (1.0 + (round - 1) as f32 * 0.055).min(3.0);
    let mut draw_m = 1.0 + ((round - 1) as f32 * 0.025).min(0.65);
    let mut aim = (26.0 - round as f32 * 1.2).max(6.0);
    let mut dmin = (1.6 - round as f32 * 0.04).max(0.55);
    let mut dmax = (3.2 - round as f32 * 0.06).max(1.15);

    match archetype {
        EnemyArchetype::Grunt => {}

        EnemyArchetype::Fast => {
            hp_m *= 0.75;
            dmg_m *= 0.85;
            draw_m *= 1.35;
            dmin *= 0.65;
            dmax *= 0.65;
            aim *= 1.15;
        }

        EnemyArchetype::Tank => {
            hp_m *= 1.85;
            dmg_m *= 0.9;
            draw_m *= 0.75;
            dmin *= 1.25;
            dmax *= 1.25;
        }

        EnemyArchetype::Sniper => {
            hp_m *= 0.85;
            dmg_m *= 1.45;
            draw_m *= 0.95;
            aim *= 0.45;
            dmin *= 1.2;
            dmax *= 1.2;
        }

        EnemyArchetype::Splitter => {
            hp_m *= 0.95;
            dmg_m *= 0.7;
            draw_m *= 1.05;
            aim *= 1.05;
        }

        EnemyArchetype::Boss => {
            hp_m *= 2.6;
            dmg_m *= 1.25;
            draw_m *= 1.15;
            aim *= 0.6;
            dmin *= 0.8;
            dmax *= 0.8;
        }
    }

    EnemySpawnConfig {
        health: (100.0 * hp_m).round() as i32,
        damage_mult: dmg_m,
        draw_speed_mult: draw_m,
        aim_error: aim,
        decision_min: dmin,
        decision_max: dmax,
        score_value: if boss {
            50 + round * 10
        } else {
            10 + round * 2
        },
        archetype,
    }
}

fn choose_enemy_archetype(round: u32, boss: bool) -> EnemyArchetype {
    if boss {
        return EnemyArchetype::Boss;
    }

    if round < 3 {
        return EnemyArchetype::Grunt;
    }

    let roll = rand::rng().random_range(0..100);

    match round {
        0..=4 => {
            if roll < 70 {
                EnemyArchetype::Grunt
            } else {
                EnemyArchetype::Fast
            }
        }
        5..=9 => {
            if roll < 35 {
                EnemyArchetype::Grunt
            } else if roll < 60 {
                EnemyArchetype::Fast
            } else if roll < 82 {
                EnemyArchetype::Tank
            } else {
                EnemyArchetype::Sniper
            }
        }
        _ => {
            if roll < 22 {
                EnemyArchetype::Grunt
            } else if roll < 42 {
                EnemyArchetype::Fast
            } else if roll < 62 {
                EnemyArchetype::Tank
            } else if roll < 82 {
                EnemyArchetype::Sniper
            } else {
                EnemyArchetype::Splitter
            }
        }
    }
}

pub fn spawn_enemies_system(
    time: Res<Time>,
    mut rm: ResMut<RoundManager>,
    mut commands: Commands,
    textures: Res<WarriorArt>,
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

    let boss = rm.round > 0 && rm.round.is_multiple_of(5);
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

    let mut enemy_mods = CombatMods {
        damage_mult: cfg.damage_mult,
        draw_speed_mult: cfg.draw_speed_mult,
        ..default()
    };

    match cfg.archetype {
        EnemyArchetype::Tank => {
            enemy_mods.damage_taken_mult = 0.85;
        }
        EnemyArchetype::Splitter => {
            enemy_mods.arrow_count = 3;
            enemy_mods.spread_deg = 20.0;
        }
        EnemyArchetype::Boss => {
            enemy_mods.arrow_count = 3;
            enemy_mods.spread_deg = 24.0;
            enemy_mods.crit_chance = 0.15;
        }
        _ => {}
    }

    let (enemy_tint, enemy_scale) = match cfg.archetype {
        EnemyArchetype::Grunt => (Color::srgb(0.95, 0.95, 0.98), 1.0),
        EnemyArchetype::Fast => (Color::srgb(0.45, 0.95, 1.0), 0.82),
        EnemyArchetype::Tank => (Color::srgb(0.85, 0.55, 0.30), 1.35),
        EnemyArchetype::Sniper => (Color::srgb(0.75, 0.50, 1.0), 0.95),
        EnemyArchetype::Splitter => (Color::srgb(1.0, 0.55, 0.25), 1.05),
        EnemyArchetype::Boss => (Color::srgb(1.0, 0.40, 0.35), 1.55),
    };

    let enemy = spawn_warrior(
        &mut commands,
        &textures,
        SpawnWarrior {
            translation: Vec2::new(spawn_x, -112.0),
            team: Team::Enemy,
            mods: enemy_mods,
            base_hp: cfg.health,
            scale: enemy_scale,
        },
    );
    commands.entity(enemy).insert(cfg.archetype);
    commands
        .entity(enemy)
        .insert(ArchetypeVisual { tint: enemy_tint });

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
    asset_server: Res<AssetServer>,
    sfx: Res<super::audio_fx::CombatSfx>,
    mut commands: Commands,
    mut warriors: Query<&mut WarriorRoot>,
) {
    for h in hits.read() {
        if let Some(player) = rm.player_entity
            && h.shooter == player
        {
            rm.damage_dealt += h.damage as u32;
            if h.headshot {
                rm.headshots += 1;
            }
        }
        if h.killed {
            // Bone Leech / Vampirism: heal the player on enemy kills.
            if let Some(player) = rm.player_entity
                && h.target != player
                && let Ok(mut pw) = warriors.get_mut(player)
                && !pw.is_dead
                && pw.kill_heal > 0
            {
                pw.health = (pw.health + pw.kill_heal).min(pw.max_health);
            }
            if let Ok(w) = warriors.get(h.target) {
                if w.team == Team::Enemy {
                    rm.kills += 1;
                    rm.score += rm.current_enemy_score;
                    rm.enemies_remaining = rm.enemies_remaining.saturating_sub(1);
                    rm.active_enemy = None;
                    if rm.enemies_remaining == 0 {
                        if rm.round >= FINAL_ROUND && rm.round.is_multiple_of(FINAL_ROUND) {
                            if !rm.victory_claimed {
                                rm.victory = true;
                                rm.victory_claimed = true;
                                rm.score += 250;
                            }

                            rm.loop_count += 1;

                            let player = rm.player_entity;
                            enter_reward(
                                &mut rm,
                                &mut warriors,
                                player,
                                &mut commands,
                                &asset_server,
                                &sfx,
                            );
                        } else {
                            let player = rm.player_entity;
                            enter_reward(
                                &mut rm,
                                &mut warriors,
                                player,
                                &mut commands,
                                &asset_server,
                                &sfx,
                            );
                        }
                    } else {
                        rm.spawn_cooldown = 1.0;
                    }
                } else if w.team == Team::Player {
                    super::audio_fx::play_sfx(&mut commands, &asset_server, &sfx.death, 0.6, 0.0);
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
    commands: &mut Commands,
    asset_server: &AssetServer,
    sfx: &super::audio_fx::CombatSfx,
) {
    // Already in reward (e.g. multi-kill same frame): don't re-roll / re-heal.
    if rm.phase == RunPhase::Reward {
        return;
    }

    rm.phase = RunPhase::Reward;

    super::audio_fx::play_sfx(commands, asset_server, &sfx.reward, 0.5, 0.0);

    if let Some(p) = player
        && let Ok(mut w) = warriors.get_mut(p)
    {
        let heal = ((w.max_health as f32) * 0.12).round().max(5.0) as i32;
        w.health = (w.health + heal).min(w.max_health);
    }

    rm.reward_choices = roll_rewards(rm);

    // Hard guarantee: reward phase must always have ≥1 selectable card.
    if rm.reward_choices.is_empty() {
        rm.reward_choices.push(RewardDef {
            id: "field_dressing",
            title: "Field Dressing",
            description: "Heal 45% max HP.",
            rarity: 2,
            category: "defense",
            max_stacks: 999,
            min_round: 1,
            weight: 1,
        });
    }
}

fn roll_rewards(rm: &RoundManager) -> Vec<RewardDef> {
    let mut pool: Vec<RewardDef> = REWARD_POOL
        .iter()
        .filter(|r| rm.round >= r.min_round)
        .filter(|r| {
            let count = *rm.upgrade_counts.get(r.id).unwrap_or(&0);
            count < r.max_stacks
        })
        .cloned()
        .collect();

    let mut result = Vec::with_capacity(3);

    while result.len() < 3 && !pool.is_empty() {
        let total_weight: u32 = pool.iter().map(|r| r.weight).sum();
        // random_range(0..0) panics — never allow it.
        if total_weight == 0 {
            result.push(pool.remove(0));
            continue;
        }

        let mut roll = rand::rng().random_range(0..total_weight);
        let mut chosen_index = 0;

        for (i, reward) in pool.iter().enumerate() {
            if roll < reward.weight {
                chosen_index = i;
                break;
            }
            roll -= reward.weight;
        }

        result.push(pool.remove(chosen_index));
    }

    result
}

fn end_run(rm: &mut RoundManager, victory: bool) {
    rm.phase = RunPhase::GameOver;
    if victory {
        rm.victory = true;
        if !rm.victory_claimed {
            rm.score += 250;
            rm.victory_claimed = true;
        }
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
            *rm.upgrade_counts.entry(*id).or_insert(0) += 1;

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
                    w.arrow_count = (w.arrow_count + 1).min(4);
                    w.spread_deg += 4.0;
                }
                "knockback" => w.knockback_mult *= 1.3,
                "field_dressing" => {
                    let h = ((w.max_health as f32) * 0.45).round() as i32;
                    w.health = (w.health + h).min(w.max_health);
                }
                "crit" => {
                    w.crit_chance = (w.crit_chance + 0.12).min(0.6);
                    w.crit_mult = w.crit_mult.max(2.0);
                }
                "leech" => {
                    w.kill_heal += 8;
                }
                "glass" => {
                    w.damage_mult *= 1.75;
                    w.max_health = (w.max_health - 20).max(1);
                    w.health = w.health.min(w.max_health);
                }
                "second_heart" => {
                    w.revives += 1;
                }
                "last_stand" => {
                    w.last_stand = true;
                }
                "giant_arrows" => {
                    w.damage_mult *= 1.45;
                    w.knockback_mult *= 1.45;
                    w.velocity_mult *= 0.85;
                }
                "needlepoint" => {
                    w.velocity_mult *= 1.30;
                    w.headshot_mult *= 1.20;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn round_at(rm: &mut RoundManager, round: u32) {
        rm.round = round - 1;
        start_next_round(rm);
    }

    #[test]
    fn roll_rewards_never_exceeds_three() {
        let rm = RoundManager::default();
        let rolled = roll_rewards(&rm);
        assert!(rolled.len() <= 3);
    }

    #[test]
    fn roll_rewards_respects_min_round() {
        let rm = RoundManager {
            round: 1,
            ..Default::default()
        };
        let rolled = roll_rewards(&rm);
        assert!(rolled.len() <= 3);
        assert!(rolled.iter().all(|r| r.min_round <= 1));
    }

    #[test]
    fn roll_rewards_respects_stack_caps() {
        let mut rm = RoundManager {
            round: 99,
            ..Default::default()
        };
        rm.upgrade_counts.insert("second_heart", 1);
        let rolled = roll_rewards(&rm);
        assert!(rolled.iter().all(|r| r.id != "second_heart"));
    }

    #[test]
    fn end_run_does_not_clear_existing_victory() {
        let mut rm = RoundManager {
            victory: true,
            ..Default::default()
        };
        end_run(&mut rm, false);
        assert!(rm.victory);
        assert_eq!(rm.phase, RunPhase::GameOver);
    }

    #[test]
    fn next_round_increases_round_and_seeds_spawn_counters() {
        let mut rm = RoundManager::default();
        start_next_round(&mut rm);
        assert_eq!(rm.round, 1);
        assert!(rm.enemies_total >= 1);
        assert_eq!(rm.enemies_remaining, rm.enemies_total);
        assert_eq!(rm.enemies_spawned, 0);
        assert_eq!(rm.phase, RunPhase::Combat);
    }

    #[test]
    fn boss_rounds_pace_endless() {
        let mut rm = RoundManager::default();
        round_at(&mut rm, 5);
        assert_eq!(rm.enemies_total, 1);
        round_at(&mut rm, 30);
        assert_eq!(rm.enemies_total, 2);
        round_at(&mut rm, 60);
        assert_eq!(rm.enemies_total, 3);
    }

    #[test]
    fn regular_rounds_cap_at_eight() {
        let mut rm = RoundManager::default();
        // Round 24 (non-boss): (1 + 23/3) = 8.
        round_at(&mut rm, 24);
        assert_eq!(rm.enemies_total, 8);
        // Later non-boss round stays capped at 8.
        round_at(&mut rm, 36);
        assert_eq!(rm.enemies_total, 8);
    }

    #[test]
    fn rewards_carry_full_card_metadata() {
        let def = REWARD_POOL.iter().find(|r| r.id == "crit").unwrap();
        assert!(!def.category.is_empty());
        assert!(def.max_stacks >= 1);
        assert!(def.min_round >= 1);
        assert!(def.weight >= 1);
    }
}
