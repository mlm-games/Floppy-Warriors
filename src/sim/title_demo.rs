use bevy_ecs::prelude::*;
use glam::Vec2;
use rand::RngExt;

use super::arena::STAND_Y;
use super::comps::*;
use super::rounds::RunState;
use super::state::SimStep;
use super::warrior::{SpawnWarrior, spawn_warrior};

#[derive(Component)]
pub struct TitleDemo;

#[derive(Component)]
pub struct DemoSide {
    pub pos: Vec2,
    pub tint: [f32; 4],
    pub other: Entity,
}

#[derive(Component)]
pub struct DemoAi {
    pub aim_error: f32,
    pub decision_min: f32,
    pub decision_max: f32,
    pub decision_timer: f32,
    pub draw_timer: f32,
    pub drawing: bool,
    pub aim_offset: Vec2,
}

#[derive(Component)]
pub struct DemoArrow {
    pub shooter: Entity,
    pub velocity: Vec2,
    pub pos: Vec2,
    pub stuck_life: f32,
}

#[derive(Resource, Default)]
pub struct DemoRespawn {
    pub queued: Option<RespawnJob>,
}

pub struct RespawnJob {
    pub corpse: Entity,
    pub pos: Vec2,
    pub tint: [f32; 4],
    pub other: Entity,
    pub timer: f32,
    pub health: i32,
}

pub const DEMO_GRAVITY: f32 = 420.0;
pub const DEMO_ARROW_DAMAGE: i32 = 14;
pub const DEMO_HP: i32 = 46;

pub fn title_demo_setup(world: &mut World) {
    super::arena::spawn_ground(world);
    let a_pos = Vec2::new(-280.0, STAND_Y);
    let b_pos = Vec2::new(280.0, STAND_Y);
    let a = spawn_demo_warrior(world, a_pos, [0.36, 0.6, 1.0, 1.0], DEMO_HP);
    let b = spawn_demo_warrior(world, b_pos, [1.0, 0.42, 0.38, 1.0], DEMO_HP);
    world.entity_mut(a).insert((
        DemoSide {
            pos: a_pos,
            tint: [0.36, 0.6, 1.0, 1.0],
            other: b,
        },
        DemoAi {
            aim_error: 34.0,
            decision_min: 0.7,
            decision_max: 1.6,
            decision_timer: 0.4,
            draw_timer: 0.0,
            drawing: false,
            aim_offset: Vec2::ZERO,
        },
    ));
    world.entity_mut(b).insert((
        DemoSide {
            pos: b_pos,
            tint: [1.0, 0.42, 0.38, 1.0],
            other: a,
        },
        DemoAi {
            aim_error: 34.0,
            decision_min: 0.7,
            decision_max: 1.6,
            decision_timer: 0.9,
            draw_timer: 0.0,
            drawing: false,
            aim_offset: Vec2::ZERO,
        },
    ));
}

pub fn title_demo_teardown(world: &mut World) {
    let demo: Vec<Entity> = world
        .query::<(Entity, &TitleDemo)>()
        .iter(world)
        .map(|(e, _)| e)
        .collect();
    for e in demo {
        world.despawn(e);
    }
    world.insert_resource(DemoRespawn::default());
}

fn spawn_demo_warrior(world: &mut World, pos: Vec2, tint: [f32; 4], hp: i32) -> Entity {
    let root = spawn_warrior(
        world,
        SpawnWarrior {
            translation: pos,
            team: Team::Player,
            mods: CombatMods::default(),
            base_hp: hp,
            scale: 1.0,
            tint,
        },
    );
    world.entity_mut(root).insert(TitleDemo);
    root
}

pub fn demo_ai(world: &mut World) {
    let dt = world
        .get_resource::<SimStep>()
        .map(|s| s.dt)
        .unwrap_or(1.0 / 60.0);
    let demos: Vec<(Entity, WarriorRoot, DemoAi, DemoSide)> = world
        .query::<(Entity, &WarriorRoot, &DemoAi, &DemoSide)>()
        .iter(world)
        .map(|(e, w, ai, side)| {
            (
                e,
                clone_root(w),
                DemoAi {
                    aim_error: ai.aim_error,
                    decision_min: ai.decision_min,
                    decision_max: ai.decision_max,
                    decision_timer: ai.decision_timer,
                    draw_timer: ai.draw_timer,
                    drawing: ai.drawing,
                    aim_offset: ai.aim_offset,
                },
                DemoSide {
                    pos: side.pos,
                    tint: side.tint,
                    other: side.other,
                },
            )
        })
        .collect();
    for (entity, warrior, ai, side) in demos {
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
        let target_torso = world.get::<WarriorRoot>(side.other).map(|w| w.torso);
        let Some(target_torso) = target_torso else {
            continue;
        };
        let target_pos = world
            .get::<repame_rapier2d::BodySnapshot2d>(target_torso)
            .map(|s| s.pos)
            .unwrap_or(Vec2::ZERO);
        let aim = target_pos + ai.aim_offset;
        let angle = (aim - origin).y.atan2((aim - origin).x);
        let local_angle = angle - torso_angle;
        if let Some(mut bow_aim) = world.get_mut::<BowAim>(entity) {
            bow_aim.angle = local_angle;
        }
        if ai.drawing {
            let speed = 170.0 * warrior.draw_speed_mult.max(0.1);
            if let Some(mut bow) = world.get_mut::<BowState>(entity) {
                bow.drawing = true;
                bow.draw_power = (bow.draw_power + speed * dt).min(100.0);
            }
            let draw_timer = ai.draw_timer - dt;
            if let Some(mut a) = world.get_mut::<DemoAi>(entity) {
                a.draw_timer = draw_timer;
            }
            if draw_timer <= 0.0 {
                let spawn_origin = origin + Vec2::from_angle(angle) * 40.0;
                let force = rand::rng().random_range(680.0..880.0);
                world.spawn((
                    TitleDemo,
                    DemoArrow {
                        shooter: entity,
                        velocity: Vec2::from_angle(angle) * force,
                        pos: spawn_origin,
                        stuck_life: 0.0,
                    },
                    Arrow {
                        shooter: entity,
                        team: Team::Player,
                        damage: DEMO_ARROW_DAMAGE as f32,
                        damage_mult: 1.0,
                        headshot_mult: 1.0,
                        knockback_mult: 1.0,
                        crit_chance: 0.0,
                        crit_mult: 2.0,
                        pos: spawn_origin,
                        velocity: Vec2::from_angle(angle) * force,
                        has_hit: false,
                        stuck_life: 0.0,
                        stuck_to: None,
                        stuck_local_offset: Vec2::ZERO,
                        stuck_angle_offset: 0.0,
                        angle,
                    },
                ));
                super::audio::push_cue(
                    world,
                    super::audio::AudioCue {
                        name: "bow_release",
                        volume: 0.22,
                        variance: 0.08,
                    },
                );
                if let Some(mut bow) = world.get_mut::<BowState>(entity) {
                    bow.drawing = false;
                    bow.draw_power = 0.0;
                }
                if let Some(mut a) = world.get_mut::<DemoAi>(entity) {
                    a.drawing = false;
                    a.decision_timer = rand::rng().random_range(a.decision_min..a.decision_max);
                }
            }
        } else {
            let decision_timer = ai.decision_timer - dt;
            if let Some(mut a) = world.get_mut::<DemoAi>(entity) {
                a.decision_timer = decision_timer;
            }
            if decision_timer <= 0.0 {
                if let Some(mut a) = world.get_mut::<DemoAi>(entity) {
                    a.aim_offset = Vec2::new(
                        rand::rng().random_range(-a.aim_error..a.aim_error),
                        rand::rng().random_range(-a.aim_error..a.aim_error),
                    );
                    a.drawing = true;
                    a.draw_timer = rand::rng().random_range(0.5..0.9);
                }
                if let Some(mut bow) = world.get_mut::<BowState>(entity) {
                    bow.drawing = true;
                    bow.draw_power = 0.0;
                }
            }
        }
    }
}

pub fn update_demo_arrows(world: &mut World) {
    let dt = world
        .get_resource::<SimStep>()
        .map(|s| s.dt)
        .unwrap_or(1.0 / 60.0);
    let arrows: Vec<(Entity, Entity, Vec2, Vec2)> = world
        .query::<(Entity, &DemoArrow)>()
        .iter(world)
        .map(|(e, a)| (e, a.shooter, a.pos, a.velocity))
        .collect();
    for (arrow_e, shooter, pos, vel) in arrows {
        let prev = pos;
        let vel = vel + Vec2::new(0.0, -DEMO_GRAVITY * dt);
        let next = prev + vel * dt;
        if let Some(mut a) = world.get_mut::<DemoArrow>(arrow_e) {
            a.velocity = vel;
            a.pos = next;
        }
        if let Some(mut a) = world.get_mut::<Arrow>(arrow_e) {
            a.velocity = vel;
            a.pos = next;
            a.angle = vel.y.atan2(vel.x);
        }
        let mut hit: Option<(Entity, Entity, f32)> = None;
        let limbs: Vec<(Entity, Entity, Vec2, f32)> = world
            .query::<(Entity, &WarriorLimb, &repame_rapier2d::BodySnapshot2d)>()
            .iter(world)
            .map(|(e, l, s)| (e, l.root, s.pos, l.hit_radius + 4.0))
            .collect();
        for (limb_e, root, center, radius) in limbs {
            if root == shooter {
                continue;
            }
            let dead = world
                .get::<WarriorRoot>(root)
                .map(|w| w.is_dead)
                .unwrap_or(true);
            if dead {
                continue;
            }
            if let Some(t) = segment_circle_t(prev, next, center, radius) {
                let replace = hit.as_ref().map(|(.., bt)| t < *bt).unwrap_or(true);
                if replace {
                    hit = Some((root, limb_e, t));
                }
            }
        }
        if let Some((root, limb_e, t)) = hit {
            let hit_pos = prev + (next - prev) * t;
            let dead = {
                let mut w = world.get_mut::<WarriorRoot>(root).unwrap();
                w.health = (w.health - DEMO_ARROW_DAMAGE).max(0);
                if w.health <= 0 {
                    w.is_dead = true;
                }
                w.is_dead
            };
            if dead {
                world.entity_mut(root).insert(super::state::DyingFade(2.0));
            }
            super::fx::spawn_burst(world, hit_pos, false);
            super::fx::spawn_damage_number(world, DEMO_ARROW_DAMAGE, hit_pos, false);
            let _ = limb_e;
            world.despawn(arrow_e);
            continue;
        }
        if next.y < super::arena::GROUND_TOP - 4.0 || next.x.abs() > 1000.0 || next.y > 300.0 {
            world.despawn(arrow_e);
            continue;
        }
    }
}

pub fn demo_respawn(world: &mut World) {
    let dt = world
        .get_resource::<SimStep>()
        .map(|s| s.dt)
        .unwrap_or(1.0 / 60.0);
    let has_queued = world.resource::<DemoRespawn>().queued.is_some();
    if !has_queued {
        let dead: Vec<(Entity, Vec2, [f32; 4], Entity)> = world
            .query::<(Entity, &WarriorRoot, &DemoSide)>()
            .iter(world)
            .filter(|(_, w, _)| w.is_dead)
            .map(|(e, _, side)| (e, side.pos, side.tint, side.other))
            .collect();
        if let Some((corpse, pos, tint, other)) = dead.into_iter().next() {
            world.resource_mut::<DemoRespawn>().queued = Some(RespawnJob {
                corpse,
                pos,
                tint,
                other,
                timer: 2.6,
                health: DEMO_HP,
            });
        }
        return;
    }
    let done = {
        let mut res = world.resource_mut::<DemoRespawn>();
        let job = res.queued.as_mut().unwrap();
        job.timer -= dt;
        job.timer <= 0.0
    };
    if !done {
        return;
    }
    let job = world.resource_mut::<DemoRespawn>().queued.take().unwrap();
    world.entity_mut(job.corpse).insert(RagdollApplied);
    let new_root = spawn_demo_warrior(world, job.pos, job.tint, job.health);
    world.entity_mut(new_root).insert((
        DemoSide {
            pos: job.pos,
            tint: job.tint,
            other: job.other,
        },
        DemoAi {
            aim_error: 34.0,
            decision_min: 0.7,
            decision_max: 1.6,
            decision_timer: 0.3,
            draw_timer: 0.0,
            drawing: false,
            aim_offset: Vec2::ZERO,
        },
    ));
    world.despawn(job.corpse);
    let _ = world.resource::<RunState>();
}

fn clone_root(w: &WarriorRoot) -> WarriorRoot {
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

fn segment_circle_t(a: Vec2, b: Vec2, center: Vec2, radius: f32) -> Option<f32> {
    let ab = b - a;
    let ab_len_sq = ab.length_squared();
    if ab_len_sq <= 0.0001 {
        return None;
    }
    let t = ((center - a).dot(ab) / ab_len_sq).clamp(0.0, 1.0);
    let p = a + ab * t;
    if p.distance_squared(center) <= radius * radius {
        Some(t)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_spawns_two_warriors_with_ai() {
        let mut world = World::new();
        world.insert_resource(SimStep { dt: 1.0 / 60.0 });
        world.insert_resource(DemoRespawn::default());
        world.insert_resource(crate::sim::audio::CueQueue::default());
        title_demo_setup(&mut world);
        let warriors: Vec<_> = world.query::<&WarriorRoot>().iter(&world).collect();
        assert_eq!(warriors.len(), 2);
        let ais: Vec<_> = world.query::<&DemoAi>().iter(&world).collect();
        assert_eq!(ais.len(), 2);
        let limbs: Vec<_> = world.query::<&WarriorLimb>().iter(&world).collect();
        assert_eq!(limbs.len(), 16);
    }

    #[test]
    fn demo_ai_starts_drawing_after_decision() {
        let mut world = World::new();
        world.insert_resource(SimStep { dt: 1.0 / 60.0 });
        world.insert_resource(crate::sim::audio::CueQueue::default());
        title_demo_setup(&mut world);
        let first: Entity = world
            .query::<(Entity, &DemoAi)>()
            .iter(&world)
            .map(|(e, _)| e)
            .next()
            .unwrap();
        world.get_mut::<DemoAi>(first).unwrap().decision_timer = 0.0;
        demo_ai(&mut world);
        assert!(world.get::<DemoAi>(first).unwrap().drawing);
        assert!(world.get::<BowState>(first).unwrap().drawing);
    }

    #[test]
    fn demo_death_queues_respawn() {
        let mut world = World::new();
        world.insert_resource(SimStep { dt: 1.0 / 60.0 });
        world.insert_resource(DemoRespawn::default());
        title_demo_setup(&mut world);
        let first: Entity = world
            .query::<(Entity, &WarriorRoot)>()
            .iter(&world)
            .map(|(e, _)| e)
            .next()
            .unwrap();
        world.get_mut::<WarriorRoot>(first).unwrap().is_dead = true;
        demo_respawn(&mut world);
        assert!(world.resource::<DemoRespawn>().queued.is_some());
    }
}
