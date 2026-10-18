use bevy_ecs::prelude::*;
use glam::Vec2;
use std::collections::HashMap;

use super::comps::*;
use super::enemy_ai::build_enemy_config;
use super::save::SaveData;
use super::state::SimStep;
use super::warrior::{SpawnWarrior, spawn_warrior};

pub const FINAL_ROUND: u32 = 15;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RunPhase {
    #[default]
    Combat,
    Reward,
    GameOver,
}

#[derive(Resource, Clone, Default)]
pub struct RunState {
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
    pub player: Option<Entity>,
    pub active_enemy: Option<Entity>,
    pub loop_count: u32,
    pub upgrade_counts: HashMap<String, u32>,
    pub victory_claimed: bool,
}

#[derive(Resource, Default)]
pub struct HitEvents(pub Vec<super::arrow::HitConfirmed>);

#[derive(Resource, Default)]
pub struct RewardPick(pub Option<String>);

#[derive(Resource, Default)]
pub struct SlowMo {
    pub scale: f32,
    pub remaining: f32,
}

impl SlowMo {
    pub fn start(&mut self, scale: f32, duration: f32) {
        if !self.active() || scale < self.scale {
            self.scale = scale.clamp(0.01, 1.0);
            self.remaining = duration.max(0.0);
        }
    }

    pub fn active(&self) -> bool {
        self.remaining > 0.0
    }

    pub fn time_scale(&self) -> f32 {
        if self.active() {
            self.scale.clamp(0.05, 1.0)
        } else {
            1.0
        }
    }

    pub fn tick(&mut self, dt: f32) {
        if self.remaining > 0.0 {
            self.remaining = (self.remaining - dt).max(0.0);
            if self.remaining <= 0.0 {
                self.scale = 1.0;
            }
        }
    }
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct GroundLine {
    pub top: f32,
    pub half_width: f32,
    pub half_height: f32,
}

impl Default for GroundLine {
    fn default() -> Self {
        Self {
            top: super::arena::GROUND_TOP,
            half_width: super::arena::GROUND_HALF_WIDTH,
            half_height: super::arena::GROUND_HALF_HEIGHT,
        }
    }
}

impl GroundLine {
    pub fn center_y(self) -> f32 {
        self.top + self.half_height
    }
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

pub const REWARD_POOL: &[RewardDef] = &[
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

pub fn begin_run(world: &mut World) {
    let save = world.resource::<SaveData>().clone();
    world.insert_resource(RunState::default());
    let mut mods = CombatMods::default();
    super::meta::apply_meta_to_mods(&save, &mut mods);
    let player = spawn_warrior(
        world,
        SpawnWarrior {
            translation: Vec2::new(-350.0, super::arena::STAND_Y),
            team: Team::Player,
            mods,
            base_hp: 100,
            scale: 1.0,
            tint: [0.95, 0.95, 0.98, 1.0],
        },
    );
    if world.get_resource::<RunState>().is_none() {
        return;
    }
    world.resource_mut::<RunState>().player = Some(player);
    start_next_round(world);
}

pub fn start_next_round(world: &mut World) {
    if world.get_resource::<RunState>().is_none() {
        return;
    }
    super::arrow::clear_arrows(world);
    let player = world.resource::<RunState>().player;
    if let Some(player) = player
        && let Some(mut bow) = world.get_mut::<super::comps::BowState>(player)
    {
        bow.drawing = false;
        bow.draw_power = 0.0;
    }
    let mut rm = world.resource_mut::<RunState>();
    rm.round += 1;
    let round = rm.round;
    rm.enemies_total = if round.is_multiple_of(5) {
        1 + round / 30
    } else {
        (1 + (round - 1) / 3).min(8)
    };
    rm.enemies_remaining = rm.enemies_total;
    rm.enemies_spawned = 0;
    rm.phase = RunPhase::Combat;
    rm.spawn_cooldown = 0.35;
    rm.active_enemy = None;
}

pub fn spawn_enemies_system(world: &mut World) {
    let dt: f32;
    {
        let step = match world.get_resource::<SimStep>() {
            Some(v) => v,
            None => return,
        };
        dt = step.dt;
    }
    let (phase, active, spawned, total) = {
        let rm = match world.get_resource::<RunState>() {
            Some(v) => v,
            None => return,
        };
        (
            rm.phase,
            rm.active_enemy,
            rm.enemies_spawned,
            rm.enemies_total,
        )
    };
    if phase != RunPhase::Combat || active.is_some() {
        return;
    }
    if spawned >= total {
        return;
    }
    {
        if world.get_resource::<RunState>().is_none() {
            return;
        }
        let mut rm = world.resource_mut::<RunState>();
        rm.spawn_cooldown -= dt;
        if rm.spawn_cooldown > 0.0 {
            return;
        }
    }
    let round = world.resource::<RunState>().round;
    let boss = round > 0 && round.is_multiple_of(5);
    let cfg = build_enemy_config(round, boss);
    if world.get_resource::<RunState>().is_none() {
        return;
    }
    world.resource_mut::<RunState>().current_enemy_score = cfg.score_value;
    let player_x = world
        .resource::<RunState>()
        .player
        .and_then(|p| world.get::<WarriorRoot>(p))
        .and_then(|w| {
            world
                .get::<repame_rapier2d::BodySnapshot2d>(w.torso)
                .map(|s| s.pos.x)
        })
        .unwrap_or(-350.0);
    let half_width = world
        .get_resource::<super::arena::ViewExtent>()
        .map(|extent| extent.half_width)
        .unwrap_or(super::arena::BASE_HALF_WIDTH)
        .max(320.0);
    let home_x = super::arena::CAMERA_HOME.x;
    let (near, far) = (half_width / 6.0, half_width * 0.75);
    let side = if player_x < home_x { 1.0 } else { -1.0 };
    let spawn_x = (home_x + side * rand::RngExt::random_range(&mut rand::rng(), near..far)).clamp(
        -super::arena::GROUND_HALF_WIDTH + 60.0,
        super::arena::GROUND_HALF_WIDTH - 60.0,
    );
    let mut enemy_mods = CombatMods {
        damage_mult: cfg.damage_mult,
        draw_speed_mult: cfg.draw_speed_mult,
        ..Default::default()
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
        EnemyArchetype::Grunt => ([0.95, 0.95, 0.98, 1.0], 1.0),
        EnemyArchetype::Fast => ([0.45, 0.95, 1.0, 1.0], 0.82),
        EnemyArchetype::Tank => ([0.85, 0.55, 0.30, 1.0], 1.35),
        EnemyArchetype::Sniper => ([0.75, 0.50, 1.0, 1.0], 0.95),
        EnemyArchetype::Splitter => ([1.0, 0.55, 0.25, 1.0], 1.05),
        EnemyArchetype::Boss => ([1.0, 0.40, 0.35, 1.0], 1.55),
    };
    let player = world.resource::<RunState>().player;
    let enemy = spawn_warrior(
        world,
        SpawnWarrior {
            translation: Vec2::new(spawn_x, super::arena::STAND_Y),
            team: Team::Enemy,
            mods: enemy_mods,
            base_hp: cfg.health,
            scale: enemy_scale,
            tint: enemy_tint,
        },
    );
    world.entity_mut(enemy).insert(cfg.archetype);
    let mut ai = EnemyAi {
        aim_error: cfg.aim_error,
        decision_min: cfg.decision_min,
        decision_max: cfg.decision_max,
        decision_timer: 0.5,
        draw_timer: 0.0,
        drawing: false,
        aim_offset: Vec2::ZERO,
        target: player,
    };
    super::enemy_ai::configure_enemy(&mut ai, &cfg);
    world.entity_mut(enemy).insert(ai);
    if world.get_resource::<RunState>().is_none() {
        return;
    }
    let mut rm = world.resource_mut::<RunState>();
    rm.enemies_spawned += 1;
    rm.active_enemy = Some(enemy);
}

pub fn on_hits(world: &mut World) {
    if world.get_resource::<HitEvents>().is_none() {
        return;
    }
    let hits: Vec<super::arrow::HitConfirmed> =
        std::mem::take(&mut world.resource_mut::<HitEvents>().0);
    for h in hits {
        if world.resource::<RunState>().phase != RunPhase::Combat {
            break;
        }
        let player = world.resource::<RunState>().player;
        if let Some(p) = player
            && h.shooter == p
        {
            let mut rm = world.resource_mut::<RunState>();
            rm.damage_dealt += h.damage as u32;
            if h.headshot {
                rm.headshots += 1;
            }
        }
        if h.killed {
            if let Some(p) = player
                && h.target != p
                && let Some(pw) = world
                    .get::<WarriorRoot>(p)
                    .map(|w| (w.kill_heal, w.is_dead))
                && !pw.1
                && pw.0 > 0
                && let Some(mut target) = world.get_mut::<WarriorRoot>(p)
            {
                target.health = (target.health + target.kill_heal).min(target.max_health);
            }
            let team = world.get::<WarriorRoot>(h.target).map(|w| w.team);
            if let Some(team) = team {
                if team == Team::Enemy {
                    let wave_cleared = {
                        let mut rm = world.resource_mut::<RunState>();
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
                            }
                            true
                        } else {
                            rm.spawn_cooldown = 1.0;
                            false
                        }
                    };
                    if wave_cleared {
                        enter_reward(world);
                    }
                } else {
                    super::audio::push_cue(
                        world,
                        super::audio::AudioCue {
                            name: "death",
                            volume: 0.6,
                            variance: 0.0,
                        },
                    );
                    end_run(world, false);
                }
            }
            world
                .entity_mut(h.target)
                .insert(super::state::DyingFade(2.0));
        }
    }
}

pub(crate) fn enter_reward(world: &mut World) {
    if world.get_resource::<RunState>().is_none()
        || world.resource::<RunState>().phase != RunPhase::Combat
    {
        return;
    }
    world.resource_mut::<RunState>().phase = RunPhase::Reward;
    super::arrow::clear_arrows(world);
    if let Some(player) = world.resource::<RunState>().player
        && let Some(mut bow) = world.get_mut::<super::comps::BowState>(player)
    {
        bow.drawing = false;
        bow.draw_power = 0.0;
    }
    world
        .resource::<super::audio::CueQueue>()
        .clone()
        .0
        .lock()
        .unwrap()
        .push(super::audio::AudioCue {
            name: "reward",
            volume: 0.5,
            variance: 0.0,
        });
    if let Some(p) = world.resource::<RunState>().player
        && let Some(mut w) = world.get_mut::<WarriorRoot>(p)
    {
        let heal = ((w.max_health as f32) * 0.12).round().max(5.0) as i32;
        w.health = (w.health + heal).min(w.max_health);
    }
    let rm = world.resource::<RunState>().clone_state();
    let mut choices = roll_rewards(&rm);
    if choices.is_empty() {
        choices.push(RewardDef {
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
    if world.get_resource::<RunState>().is_none() {
        return;
    }
    world.resource_mut::<RunState>().reward_choices = choices;
}

fn roll_rewards(rm: &RunStateSnapshot) -> Vec<RewardDef> {
    let mut pool: Vec<RewardDef> = REWARD_POOL
        .iter()
        .filter(|r| rm.round >= r.min_round)
        .filter(|r| {
            let count = rm.upgrade_counts.get(r.id).copied().unwrap_or(0);
            count < r.max_stacks
        })
        .cloned()
        .collect();
    let mut result = Vec::with_capacity(3);
    while result.len() < 3 && !pool.is_empty() {
        let total_weight: u32 = pool.iter().map(|r| r.weight).sum();
        if total_weight == 0 {
            result.push(pool.remove(0));
            continue;
        }
        let mut roll = rand::RngExt::random_range(&mut rand::rng(), 0..total_weight);
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

fn end_run(world: &mut World, victory: bool) {
    if world.get_resource::<RunState>().is_none() {
        return;
    }
    world.resource_mut::<RunState>().phase = RunPhase::GameOver;
    if victory {
        let mut rm = world.resource_mut::<RunState>();
        rm.victory = true;
        if !rm.victory_claimed {
            rm.score += 250;
            rm.victory_claimed = true;
        }
    }
}

pub fn apply_reward_system(world: &mut World) {
    if world.get_resource::<RewardPick>().is_none() {
        return;
    }
    let pick = world.resource_mut::<RewardPick>().0.take();
    let Some(id) = pick else { return };
    if world.resource::<RunState>().phase != RunPhase::Reward {
        return;
    }
    if !world
        .resource::<RunState>()
        .reward_choices
        .iter()
        .any(|r| r.id == id)
    {
        return;
    }
    let player = world.resource::<RunState>().player;
    if let Some(p) = player
        && world.get::<WarriorRoot>(p).is_some()
    {
        {
            world
                .resource_mut::<RunState>()
                .upgrade_counts
                .entry(id.to_string())
                .or_insert(0);
            *world
                .resource_mut::<RunState>()
                .upgrade_counts
                .get_mut(&id)
                .unwrap() += 1;
        }
        let mut w = world.get_mut::<WarriorRoot>(p).unwrap();
        match id.as_str() {
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
            _ => {}
        }
    }
    if world.get_resource::<RunState>().is_none() {
        return;
    }
    world.resource_mut::<RunState>().reward_choices.clear();
    start_next_round(world);
}

pub fn finalize_game_over_bones(world: &mut World) {
    if world.resource::<RunState>().phase != RunPhase::GameOver {
        if world
            .get_resource::<super::state::GameOverFlushed>()
            .is_none()
        {
            return;
        }
        world.resource_mut::<super::state::GameOverFlushed>().0 = false;
        return;
    }
    if world.resource::<super::state::GameOverFlushed>().0 {
        return;
    }
    if world
        .get_resource::<super::state::GameOverFlushed>()
        .is_none()
    {
        return;
    }
    world.resource_mut::<super::state::GameOverFlushed>().0 = true;
    let (victory, round, score, kills, headshots) = {
        let rm = match world.get_resource::<RunState>() {
            Some(v) => v,
            None => return,
        };
        (rm.victory, rm.round, rm.score, rm.kills, rm.headshots)
    };
    let earned = {
        let save = match world.get_resource::<SaveData>() {
            Some(v) => v,
            None => return,
        };
        super::meta::calculate_run_bones(save, victory, round, score, kills, headshots)
    };
    if world.get_resource::<RunState>().is_none() {
        return;
    }
    world.resource_mut::<RunState>().bones_earned = earned;
    if world.get_resource::<SaveData>().is_none() {
        return;
    }
    let mut save = world.resource_mut::<SaveData>();
    save.bones += earned;
    save.total_runs += 1;
    save.total_kills += kills;
    save.best_round = save.best_round.max(round);
    if victory {
        save.total_victories += 1;
    }
    save.high_score = save.high_score.max(score);
    save.last_played_unix = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
}

#[derive(Clone, Debug)]
pub struct RunStateSnapshot {
    pub round: u32,
    pub upgrade_counts: HashMap<String, u32>,
}

impl RunState {
    pub fn clone_state(&self) -> RunStateSnapshot {
        RunStateSnapshot {
            round: self.round,
            upgrade_counts: self.upgrade_counts.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::arena;
    use crate::sim::warrior;

    fn world_with_round(round: u32) -> World {
        let mut world = World::new();
        world.insert_resource(RunState {
            round: round - 1,
            ..Default::default()
        });
        start_next_round(&mut world);
        world
    }

    fn roll_rewards_in(world: &World) -> Vec<RewardDef> {
        let rm = world.resource::<RunState>();
        roll_rewards(&rm.clone_state())
    }

    #[test]
    fn enemy_spawns_inside_the_published_view_extent() {
        for half_width in [320.0f32, 640.0, 1280.0] {
            let mut world = World::new();
            world.insert_resource(SimStep::default());
            world.insert_resource(crate::sim::audio::CueQueue::default());
            world.insert_resource(crate::sim::state::GameOverFlushed::default());
            world.insert_resource(crate::sim::input::PlayerInput::default());
            world.insert_resource(HitEvents::default());
            world.insert_resource(GroundLine::default());
            world.insert_resource(arena::ViewExtent {
                half_width,
                half_height: 360.0,
            });
            world.insert_resource(RunState {
                round: 1,
                ..Default::default()
            });
            start_next_round(&mut world);
            let player = warrior::spawn_warrior(
                &mut world,
                warrior::SpawnWarrior {
                    translation: Vec2::new(0.0, arena::STAND_Y),
                    team: Team::Player,
                    mods: CombatMods::default(),
                    base_hp: 100,
                    scale: 1.0,
                    tint: [1.0, 1.0, 1.0, 1.0],
                },
            );
            world.resource_mut::<RunState>().player = Some(player);

            let mut schedule = crate::sim::state::build_schedule();
            let mut enemy_x = f32::NAN;
            for _ in 0..40 {
                schedule.run(&mut world);
                if enemy_x.is_nan()
                    && let Some(enemy) = world.resource::<RunState>().active_enemy
                {
                    enemy_x = world
                        .get::<WarriorRoot>(enemy)
                        .and_then(|w| world.get::<repame_rapier2d::BodySnapshot2d>(w.torso))
                        .map(|s| s.pos.x)
                        .unwrap_or(f32::NAN);
                }
            }
            assert!(enemy_x.is_finite(), "enemy spawns");
            let home = arena::CAMERA_HOME.x;
            let offset = enemy_x - home;
            assert!(
                offset > 0.0
                    && offset >= half_width / 6.0 - 40.0
                    && offset <= half_width * 0.75 + 40.0
                    && enemy_x.abs() < arena::GROUND_HALF_WIDTH,
                "half_width {half_width}: enemy spawned at x={enemy_x} (home {home})"
            );
        }
    }

    #[test]
    fn combat_schedule_spawns_kills_and_advances_rounds() {
        let mut world = World::new();
        world.insert_resource(SimStep::default());
        world.insert_resource(crate::sim::audio::CueQueue::default());
        world.insert_resource(crate::sim::state::GameOverFlushed::default());
        world.insert_resource(crate::sim::input::PlayerInput::default());
        world.insert_resource(HitEvents::default());
        world.insert_resource(RunState {
            round: 3,
            ..Default::default()
        });
        world.insert_resource(GroundLine::default());
        start_next_round(&mut world);
        let player = warrior::spawn_warrior(
            &mut world,
            warrior::SpawnWarrior {
                translation: Vec2::new(0.0, arena::STAND_Y),
                team: Team::Player,
                mods: CombatMods::default(),
                base_hp: 100,
                scale: 1.0,
                tint: [1.0, 1.0, 1.0, 1.0],
            },
        );
        world.resource_mut::<RunState>().player = Some(player);
        assert_eq!(world.resource::<RunState>().round, 4);
        assert_eq!(world.resource::<RunState>().enemies_total, 2);

        let mut schedule = crate::sim::state::build_schedule();
        for _ in 0..40 {
            schedule.run(&mut world);
        }
        let first = world.resource::<RunState>().active_enemy;
        assert_eq!(world.resource::<RunState>().enemies_spawned, 1);
        let first = first.expect("first enemy spawns");

        world
            .resource_mut::<HitEvents>()
            .0
            .push(crate::sim::arrow::HitConfirmed {
                shooter: player,
                target: first,
                damage: 999,
                killed: true,
                headshot: false,
            });
        for _ in 0..40 {
            schedule.run(&mut world);
        }
        let rm = world.resource::<RunState>();
        assert_eq!(rm.kills, 1);
        assert_eq!(rm.enemies_remaining, 1);
        assert_eq!(rm.enemies_spawned, 1);
        assert!(rm.active_enemy.is_none());

        for _ in 0..120 {
            schedule.run(&mut world);
        }
        let second = world.resource::<RunState>().active_enemy;
        assert_eq!(world.resource::<RunState>().enemies_spawned, 2);
        let second = second.expect("second enemy spawns after the kill");

        world
            .resource_mut::<HitEvents>()
            .0
            .push(crate::sim::arrow::HitConfirmed {
                shooter: player,
                target: second,
                damage: 999,
                killed: true,
                headshot: false,
            });
        schedule.run(&mut world);
        let rm = world.resource::<RunState>();
        assert_eq!(rm.phase, RunPhase::Reward);
        assert_eq!(
            world
                .query_filtered::<Entity, With<crate::sim::comps::Arrow>>()
                .iter(&world)
                .count(),
            0
        );
    }

    #[test]
    fn roll_rewards_never_exceeds_three() {
        let mut world = World::new();
        world.insert_resource(RunState::default());
        let rolled = roll_rewards_in(&world);
        assert!(rolled.len() <= 3);
    }

    #[test]
    fn roll_rewards_respects_min_round() {
        let world = world_with_round(1);
        let rm = world.resource::<RunState>();
        assert!(rm.round >= 1);
        let rolled = roll_rewards_in(&world);
        assert!(rolled.len() <= 3);
        assert!(rolled.iter().all(|r| r.min_round <= rm.round.max(1)));
    }

    #[test]
    fn roll_rewards_respects_stack_caps() {
        let mut world = World::new();
        world.insert_resource(RunState {
            round: 99,
            upgrade_counts: [("second_heart".to_string(), 1)].into_iter().collect(),
            ..Default::default()
        });
        let rolled = roll_rewards_in(&world);
        assert!(rolled.iter().all(|r| r.id != "second_heart"));
    }

    #[test]
    fn end_run_does_not_clear_existing_victory() {
        let mut world = World::new();
        world.insert_resource(RunState {
            victory: true,
            ..Default::default()
        });
        end_run(&mut world, false);
        let rm = world.resource::<RunState>();
        assert!(rm.victory);
        assert_eq!(rm.phase, RunPhase::GameOver);
    }

    #[test]
    fn next_round_increases_round_and_seeds_spawn_counters() {
        let mut world = World::new();
        world.insert_resource(RunState::default());
        start_next_round(&mut world);
        let rm = world.resource::<RunState>();
        assert_eq!(rm.round, 1);
        assert!(rm.enemies_total >= 1);
        assert_eq!(rm.enemies_remaining, rm.enemies_total);
        assert_eq!(rm.enemies_spawned, 0);
        assert_eq!(rm.phase, RunPhase::Combat);
    }

    #[test]
    fn boss_rounds_pace_endless() {
        let world = world_with_round(5);
        assert_eq!(world.resource::<RunState>().enemies_total, 1);
        let world = world_with_round(30);
        assert_eq!(world.resource::<RunState>().enemies_total, 2);
        let world = world_with_round(60);
        assert_eq!(world.resource::<RunState>().enemies_total, 3);
    }

    #[test]
    fn regular_rounds_cap_at_eight() {
        let world = world_with_round(24);
        assert_eq!(world.resource::<RunState>().enemies_total, 8);
        let world = world_with_round(36);
        assert_eq!(world.resource::<RunState>().enemies_total, 8);
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
