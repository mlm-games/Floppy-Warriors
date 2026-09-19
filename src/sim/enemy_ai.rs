use bevy_ecs::prelude::*;
use glam::Vec2;

use super::comps::*;
use super::rounds::{RunPhase, RunState};
use super::state::SimStep;

#[derive(Clone, Debug)]
pub struct EnemySpawnConfig {
    pub health: i32,
    pub damage_mult: f32,
    pub draw_speed_mult: f32,
    pub aim_error: f32,
    pub decision_min: f32,
    pub decision_max: f32,
    pub score_value: u32,
    pub archetype: EnemyArchetype,
}

pub fn configure_enemy(ai: &mut EnemyAi, cfg: &EnemySpawnConfig) {
    ai.aim_error = cfg.aim_error;
    ai.decision_min = cfg.decision_min;
    ai.decision_max = cfg.decision_max;
    ai.decision_timer =
        rand::RngExt::random_range(&mut rand::rng(), ai.decision_min..ai.decision_max);
    ai.drawing = false;
    ai.draw_timer = 0.0;
}

pub fn build_enemy_config(round: u32, boss: bool) -> EnemySpawnConfig {
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
    let roll = rand::RngExt::random_range(&mut rand::rng(), 0..100);
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

pub fn enemy_ai_system(world: &mut World) {
    let dt: f32;
    let combat: bool;
    {
        let step = world.resource::<SimStep>();
        dt = step.dt;
        combat = world.resource::<RunState>().phase == RunPhase::Combat;
    }
    if !combat {
        return;
    }
    let player = world.resource::<RunState>().player;
    let Some(player_e) = player else { return };
    let (player_dead, player_pos) = {
        let Some(w) = world.get::<WarriorRoot>(player_e) else {
            return;
        };
        if w.is_dead {
            return;
        }
        let pos = world
            .get::<repame_rapier2d::BodySnapshot2d>(w.torso)
            .map(|s| s.pos)
            .unwrap_or(Vec2::ZERO);
        (w.is_dead, pos)
    };
    let _ = player_dead;
    let enemies: Vec<(Entity, WarriorRoot, BowState, EnemyAi)> = {
        let mut q = world.query::<(Entity, &WarriorRoot, &BowState, &EnemyAi)>();
        q.iter(world)
            .filter(|(_, _, _, _)| true)
            .map(|(e, w, b, ai)| {
                (
                    e,
                    clone_warrior(w),
                    BowState {
                        drawing: b.drawing,
                        draw_power: b.draw_power,
                    },
                    clone_ai(ai),
                )
            })
            .collect()
    };
    let has_enemy_tag = |world: &World, e: Entity| world.get::<EnemyTag>(e).is_some();
    for (enemy_entity, warrior, _bow, ai) in enemies {
        if !has_enemy_tag(world, enemy_entity) {
            continue;
        }
        if warrior.is_dead {
            continue;
        }
        let origin = world
            .get::<repame_rapier2d::BodySnapshot2d>(warrior.torso)
            .map(|s| s.pos)
            .unwrap_or(Vec2::ZERO);
        let torso_angle = world
            .get::<repame_rapier2d::BodySnapshot2d>(warrior.torso)
            .map(|s| s.angle)
            .unwrap_or(0.0);
        let target = player_pos + ai.aim_offset;
        let angle = (target - origin).y.atan2((target - origin).x);
        let local_angle = angle - torso_angle;
        if let Some(mut aim) = world.get_mut::<BowAim>(enemy_entity) {
            aim.angle = local_angle;
        }
        if ai.drawing {
            let speed = 160.0 * warrior.draw_speed_mult.max(0.1);
            let draw_power = {
                let b = world.get::<BowState>(enemy_entity).map(|b| b.draw_power);
                b.unwrap_or(0.0)
            };
            let draw_power = (draw_power + speed * dt).min(100.0);
            if let Some(mut b) = world.get_mut::<BowState>(enemy_entity) {
                b.drawing = true;
                b.draw_power = draw_power;
            }
            let draw_timer = ai.draw_timer - dt;
            if let Some(mut a) = world.get_mut::<EnemyAi>(enemy_entity) {
                a.draw_timer = draw_timer;
            }
            if draw_timer <= 0.0 {
                let spawn_origin = origin + Vec2::from_angle(angle) * 36.0;
                let bow = BowState {
                    drawing: true,
                    draw_power,
                };
                super::arrow::fire_from_bow(
                    world,
                    enemy_entity,
                    spawn_origin,
                    angle,
                    &mut Vec::new(),
                );
                let _ = bow;
                if let Some(mut b) = world.get_mut::<BowState>(enemy_entity) {
                    b.drawing = false;
                    b.draw_power = 0.0;
                }
                if let Some(mut a) = world.get_mut::<EnemyAi>(enemy_entity) {
                    a.drawing = false;
                    a.decision_timer = rand::RngExt::random_range(
                        &mut rand::rng(),
                        a.decision_min..a.decision_max,
                    );
                }
            }
        } else {
            let decision_timer = ai.decision_timer - dt;
            if let Some(mut a) = world.get_mut::<EnemyAi>(enemy_entity) {
                a.decision_timer = decision_timer;
            }
            if decision_timer <= 0.0 {
                let aim_offset = Vec2::new(
                    rand::RngExt::random_range(&mut rand::rng(), -ai.aim_error..ai.aim_error),
                    rand::RngExt::random_range(&mut rand::rng(), -ai.aim_error..ai.aim_error),
                );
                if let Some(mut a) = world.get_mut::<EnemyAi>(enemy_entity) {
                    a.aim_offset = aim_offset;
                    a.drawing = true;
                    a.draw_timer = {
                        let full_draw = 100.0 / (160.0 * warrior.draw_speed_mult.max(0.1));
                        rand::RngExt::random_range(
                            &mut rand::rng(),
                            (full_draw * 0.45).min(0.45)..full_draw * 0.9,
                        )
                    };
                }
                if let Some(mut b) = world.get_mut::<BowState>(enemy_entity) {
                    b.drawing = true;
                    b.draw_power = 0.0;
                }
            }
        }
    }
}

fn clone_warrior(w: &WarriorRoot) -> WarriorRoot {
    WarriorRoot {
        team: w.team,
        health: w.health,
        max_health: w.max_health,
        is_dead: w.is_dead,
        torso: w.torso,
        damage_mult: w.damage_mult,
        velocity_mult: w.velocity_mult,
        knockback_mult: w.knockback_mult,
        headshot_mult: w.headshot_mult,
        draw_speed_mult: w.draw_speed_mult,
        arrow_count: w.arrow_count,
        spread_deg: w.spread_deg,
        damage_taken_mult: w.damage_taken_mult,
        crit_chance: w.crit_chance,
        crit_mult: w.crit_mult,
        kill_heal: w.kill_heal,
        revives: w.revives,
        last_stand: w.last_stand,
    }
}

fn clone_ai(ai: &EnemyAi) -> EnemyAi {
    EnemyAi {
        aim_error: ai.aim_error,
        decision_min: ai.decision_min,
        decision_max: ai.decision_max,
        decision_timer: ai.decision_timer,
        draw_timer: ai.draw_timer,
        drawing: ai.drawing,
        aim_offset: ai.aim_offset,
        target: ai.target,
    }
}
