use bevy_ecs::prelude::*;
use glam::Vec2;
use rand::RngExt;

use super::comps::*;
use super::rounds::RunPhase;
use super::state::SimStep;
use super::warrior::{MAX_DRAW, MAX_FORCE_ADD, MIN_FORCE};

pub const ARROW_GRAVITY: f32 = 420.0;
pub const STUCK_LIFETIME: f32 = 2.0;
pub const HITSTUN_SECS: f32 = 0.22;

pub fn spawn_arrow(
    world: &mut World,
    origin: Vec2,
    velocity: Vec2,
    shooter: Entity,
    team: Team,
    damage: f32,
    damage_mult: f32,
    headshot_mult: f32,
    knockback_mult: f32,
    crit_chance: f32,
    crit_mult: f32,
) {
    world.spawn(Arrow {
        shooter,
        team,
        damage,
        damage_mult,
        headshot_mult,
        knockback_mult,
        crit_chance,
        crit_mult,
        pos: origin,
        velocity,
        has_hit: false,
        stuck_life: STUCK_LIFETIME,
        stuck_to: None,
        stuck_local_offset: Vec2::ZERO,
        stuck_angle_offset: 0.0,
        angle: velocity.y.atan2(velocity.x),
    });
}

pub fn fire_from_bow(
    world: &mut World,
    warrior_e: Entity,
    origin: Vec2,
    base_angle: f32,
    cues: &mut Vec<super::audio::AudioCue>,
) {
    let bow = world.get::<BowState>(warrior_e);
    let (Some(bow),) = (bow,) else { return };
    if !bow.drawing {
        return;
    }
    let draw_power = bow.draw_power;
    let warrior = world.get::<WarriorRoot>(warrior_e).map(clone_warrior);
    let Some(warrior) = warrior else { return };
    if warrior.is_dead {
        return;
    }
    cues.push(super::audio::AudioCue {
        name: "bow_release",
        volume: 0.45,
        variance: 0.08,
    });
    let mut damage_mult = warrior.damage_mult;
    if warrior.last_stand {
        let hp_pct = warrior.health.max(0) as f32 / warrior.max_health.max(1) as f32;
        if hp_pct <= 0.35 {
            damage_mult *= 1.75;
        }
    }
    let force = MIN_FORCE + (draw_power / MAX_DRAW) * MAX_FORCE_ADD;
    let n = warrior.arrow_count.max(1);
    for i in 0..n {
        let mut angle = base_angle;
        if n > 1 {
            let step = warrior.spread_deg / (n - 1) as f32;
            let off = -warrior.spread_deg * 0.5 + step * i as f32;
            angle += off.to_radians();
        }
        let dir = Vec2::from_angle(angle);
        spawn_arrow(
            world,
            origin,
            dir * force * warrior.velocity_mult,
            warrior_e,
            warrior.team,
            20.0,
            damage_mult,
            warrior.headshot_mult,
            warrior.knockback_mult,
            warrior.crit_chance,
            warrior.crit_mult,
        );
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

#[derive(Clone, Debug)]
pub struct HitConfirmed {
    pub shooter: Entity,
    pub target: Entity,
    pub damage: i32,
    pub killed: bool,
    pub headshot: bool,
}

pub fn update_arrows(world: &mut World) {
    let dt: f32;
    let phase: RunPhase;
    {
        let step = match world.get_resource::<SimStep>() {
            Some(v) => v,
            None => return,
        };
        dt = step.dt;
        phase = world.resource::<super::rounds::RunState>().phase;
    }
    if phase != RunPhase::Combat {
        return;
    }
    let arrows: Vec<(Entity, Arrow)> = world
        .query::<(Entity, &Arrow)>()
        .iter(world)
        .map(|(e, a)| (e, clone_arrow(a)))
        .collect();
    for (arrow_e, arrow) in arrows {
        if arrow.has_hit {
            let mut stuck_angle = arrow.angle;
            let mut stuck_pos = arrow.pos;
            if let Some(parent) = arrow.stuck_to
                && let Some(snapshot) = world.get::<repame_rapier2d::BodySnapshot2d>(parent)
            {
                let parent_angle = snapshot.angle;
                let parent_pos = snapshot.pos;
                let world_offset = rotate_vec(arrow.stuck_local_offset, parent_angle);
                stuck_pos = parent_pos + world_offset;
                stuck_angle = parent_angle + arrow.stuck_angle_offset;
            }
            let life = arrow.stuck_life - dt;
            if life <= 0.0 {
                world.despawn(arrow_e);
                continue;
            }
            if let Some(mut a) = world.get_mut::<Arrow>(arrow_e) {
                a.pos = stuck_pos;
                a.angle = stuck_angle;
                a.stuck_life = life;
            }
            continue;
        }
        let prev = arrow.pos;
        let mut vel = arrow.velocity;
        vel.y += ARROW_GRAVITY * dt;
        let next = prev + vel * dt;
        let arrow_angle = vel.y.atan2(vel.x);
        let hit = find_limb_hit(world, arrow_e, prev, next);
        if let Some((limb_e, root_e, torso_e, kind, hit_point, limb_angle, limb_center)) = hit {
            resolve_limb_hit(
                world,
                arrow_e,
                limb_e,
                root_e,
                torso_e,
                kind,
                hit_point,
                limb_angle,
                limb_center,
                arrow_angle,
                vel,
                &arrow,
            );
            continue;
        }
        if let Some(hit_point) = find_ground_hit(world, prev, next) {
            if let Some(mut a) = world.get_mut::<Arrow>(arrow_e) {
                a.has_hit = true;
                a.velocity = Vec2::ZERO;
                a.stuck_life = STUCK_LIFETIME;
                a.stuck_to = None;
                a.pos = hit_point;
                a.angle = arrow_angle;
            }
            continue;
        }
        if let Some(mut a) = world.get_mut::<Arrow>(arrow_e) {
            a.pos = next;
            a.velocity = vel;
            a.angle = arrow_angle;
        }
    }
}

fn clone_arrow(a: &Arrow) -> Arrow {
    Arrow {
        shooter: a.shooter,
        team: a.team,
        damage: a.damage,
        damage_mult: a.damage_mult,
        headshot_mult: a.headshot_mult,
        knockback_mult: a.knockback_mult,
        crit_chance: a.crit_chance,
        crit_mult: a.crit_mult,
        pos: a.pos,
        velocity: a.velocity,
        has_hit: a.has_hit,
        stuck_life: a.stuck_life,
        stuck_to: a.stuck_to,
        stuck_local_offset: a.stuck_local_offset,
        stuck_angle_offset: a.stuck_angle_offset,
        angle: a.angle,
    }
}

fn find_limb_hit(
    world: &mut World,
    arrow_e: Entity,
    prev: Vec2,
    next: Vec2,
) -> Option<(Entity, Entity, Entity, LimbKind, Vec2, f32, Vec2)> {
    use repame_rapier2d::BodySnapshot2d;
    let arrow = world.get::<Arrow>(arrow_e).map(clone_arrow)?;
    let limbs: Vec<(Entity, Entity, LimbKind, f32, Vec2, f32, Vec2)> = {
        let mut out = Vec::new();
        let mut limb_q = world.query::<(Entity, &WarriorLimb)>();
        let limb_list: Vec<(Entity, Entity, LimbKind, f32)> = limb_q
            .iter(world)
            .map(|(e, l)| (e, l.root, l.kind, l.hit_radius))
            .collect();
        for (limb_e, root, kind, hit_radius) in limb_list {
            let Some(warrior) = world.get::<WarriorRoot>(root).map(clone_warrior) else {
                continue;
            };
            if warrior.is_dead || root == arrow.shooter || warrior.team == arrow.team {
                continue;
            }
            let Some(snapshot) = world.get::<BodySnapshot2d>(limb_e) else {
                continue;
            };
            let center = snapshot.pos;
            let radius = hit_radius + 4.0;
            if let Some((t, hit_point)) = segment_circle_hit(prev, next, center, radius) {
                out.push((limb_e, root, kind, t, hit_point, snapshot.angle, center));
            }
        }
        out
    };
    let mut best: Option<(f32, Entity, Entity, Entity, LimbKind, Vec2, f32, Vec2)> = None;
    for (limb_e, root, kind, t, hit_point, limb_angle, center) in limbs {
        let torso = world.get::<WarriorRoot>(root).map(|w| w.torso);
        let Some(torso_e) = torso else { continue };
        let replace = best.as_ref().map(|(bt, ..)| t < *bt).unwrap_or(true);
        if replace {
            best = Some((
                t, limb_e, root, torso_e, kind, hit_point, limb_angle, center,
            ));
        }
    }
    best.map(
        |(_, limb_e, root_e, torso_e, kind, hit_point, limb_angle, limb_center)| {
            (
                limb_e,
                root_e,
                torso_e,
                kind,
                hit_point,
                limb_angle,
                limb_center,
            )
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn resolve_limb_hit(
    world: &mut World,
    arrow_e: Entity,
    limb_e: Entity,
    root_e: Entity,
    torso_e: Entity,
    kind: LimbKind,
    hit_point: Vec2,
    limb_angle: f32,
    limb_center: Vec2,
    arrow_angle: f32,
    vel: Vec2,
    arrow: &Arrow,
) {
    use repame_rapier2d::ImpulseRequest;
    let mult = kind.damage_mult()
        * if kind == LimbKind::Head {
            arrow.headshot_mult
        } else {
            1.0
        };
    let headshot = kind == LimbKind::Head;
    let target_dt = world
        .get::<WarriorRoot>(root_e)
        .map(|w| w.damage_taken_mult)
        .unwrap_or(1.0);
    let crit = rand::rng().random::<f32>() < arrow.crit_chance;
    let mut damage_f = arrow.damage * arrow.damage_mult * mult * target_dt;
    if crit {
        damage_f *= arrow.crit_mult;
    }
    let damage = damage_f.round().max(1.0) as i32;
    let mut killed = false;
    if let Some(mut warrior) = world.get_mut::<WarriorRoot>(root_e)
        && !warrior.is_dead
    {
        warrior.health = (warrior.health - damage).max(0);
        killed = warrior.health <= 0;
        if killed {
            if warrior.team == Team::Player && warrior.revives > 0 {
                warrior.revives -= 1;
                warrior.health = (warrior.max_health / 2).max(1);
                warrior.is_dead = false;
                killed = false;
                if let Some(mut trauma) = world.get_resource_mut::<repame_fx::Trauma>() {
                    trauma.add(0.65);
                }
            } else {
                warrior.is_dead = true;
            }
        }
    }
    let knock = vel.normalize_or_zero();
    if let Some(mut impulse) = world.get_mut::<ImpulseRequest>(torso_e) {
        impulse.linear += knock * damage as f32 * 8.0 * arrow.knockback_mult;
    }
    if let Some(mut impulse) = world.get_mut::<ImpulseRequest>(limb_e) {
        impulse.linear += knock * damage as f32 * 3.5 * arrow.knockback_mult;
    }
    world.entity_mut(root_e).insert(HitStun {
        remaining: HITSTUN_SECS,
    });
    super::fx::spawn_damage_number(world, damage, hit_point, headshot);
    super::fx::spawn_burst(world, hit_point, headshot);
    let hit_player = world.get::<PlayerTag>(root_e).is_some();
    if let Some(mut trauma) = world.get_resource_mut::<repame_fx::Trauma>() {
        if !hit_player {
            if headshot && killed {
                trauma.add(0.7);
            } else if killed {
                trauma.add(0.45);
            } else if headshot {
                trauma.add(0.35);
            } else {
                trauma.add(0.15);
            }
        } else {
            trauma.add(if headshot { 0.35 } else { 0.18 });
        }
    }
    {
        if world.get_resource::<super::rounds::SlowMo>().is_none() {
            return;
        }
        let mut slow = world.resource_mut::<super::rounds::SlowMo>();
        if !hit_player {
            if headshot && killed {
                slow.start(0.25, 0.22);
            } else if killed {
                slow.start(0.4, 0.08);
            } else if headshot {
                slow.start(0.05, 0.06);
            } else {
                slow.start(0.05, 0.05);
            }
        }
    }
    {
        let cues = world.resource::<super::audio::CueQueue>().clone();
        let mut cues = cues.0.lock().unwrap();
        cues.push(super::audio::AudioCue {
            name: if headshot { "headshot" } else { "hit" },
            volume: if headshot { 0.6 } else { 0.5 },
            variance: if headshot { 0.08 } else { 0.1 },
        });
        if killed {
            cues.push(super::audio::AudioCue {
                name: "kill",
                volume: 0.55,
                variance: 0.05,
            });
        }
    }
    {
        if world.get_resource::<super::rounds::HitEvents>().is_none() {
            return;
        }
        let mut hits = world.resource_mut::<super::rounds::HitEvents>();
        hits.0.push(HitConfirmed {
            shooter: arrow.shooter,
            target: root_e,
            damage,
            killed,
            headshot,
        });
    }
    if let Some(mut a) = world.get_mut::<Arrow>(arrow_e) {
        a.has_hit = true;
        a.velocity = Vec2::ZERO;
        a.stuck_life = STUCK_LIFETIME;
        a.stuck_to = Some(limb_e);
        a.stuck_local_offset = rotate_vec(hit_point - limb_center, -limb_angle);
        a.stuck_angle_offset = arrow_angle - limb_angle;
        a.pos = hit_point;
        a.angle = arrow_angle;
    }
}

pub fn clear_arrows(world: &mut World) {
    let arrows: Vec<Entity> = world
        .query_filtered::<Entity, With<Arrow>>()
        .iter(world)
        .collect();
    for entity in arrows {
        world.despawn(entity);
    }
}

fn find_ground_hit(world: &mut World, prev: Vec2, next: Vec2) -> Option<Vec2> {
    let ground = *world.get_resource::<super::rounds::GroundLine>()?;
    segment_top_rect_hit(
        prev,
        next,
        Vec2::new(0.0, ground.center_y()),
        Vec2::new(ground.half_width * 2.0, ground.half_height * 2.0),
    )
    .map(|(_, p)| p)
}

pub fn segment_circle_hit(a: Vec2, b: Vec2, center: Vec2, radius: f32) -> Option<(f32, Vec2)> {
    let ab = b - a;
    let ab_len_sq = ab.length_squared();
    if ab_len_sq <= 0.0001 {
        return None;
    }
    let t = ((center - a).dot(ab) / ab_len_sq).clamp(0.0, 1.0);
    let p = a + ab * t;
    if p.distance_squared(center) <= radius * radius {
        Some((t, p))
    } else {
        None
    }
}

fn segment_top_rect_hit(a: Vec2, b: Vec2, center: Vec2, size: Vec2) -> Option<(f32, Vec2)> {
    let half = size * 0.5;
    let left = center.x - half.x;
    let right = center.x + half.x;
    let top = center.y - half.y;
    if a.y <= top && b.y >= top {
        let denom = a.y - b.y;
        if denom.abs() > 0.0001 {
            let t = ((a.y - top) / denom).clamp(0.0, 1.0);
            let x = a.x + (b.x - a.x) * t;
            if x >= left && x <= right {
                return Some((t, Vec2::new(x, top)));
            }
        }
    }
    None
}

pub fn rotate_vec(v: Vec2, angle: f32) -> Vec2 {
    let (s, c) = angle.sin_cos();
    Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::state::{GameOverFlushed, SimStep};
    use repame_rapier2d::{BodySnapshot2d, ImpulseRequest, RapierBody2d, RapierWorld2d};

    fn test_world() -> World {
        let mut world = World::new();
        world.insert_resource(SimStep::default());
        world.insert_resource(crate::sim::rounds::RunState {
            phase: crate::sim::rounds::RunPhase::Combat,
            ..Default::default()
        });
        world.insert_resource(RapierWorld2d::new(glam::Vec2::ZERO, 20.0));
        world.insert_resource(repame_fx::Trauma::default());
        world.insert_resource(crate::sim::audio::CueQueue::default());
        world.insert_resource(crate::sim::rounds::HitEvents::default());
        world.insert_resource(GameOverFlushed::default());
        world.insert_resource(crate::sim::rounds::SlowMo::default());
        world.insert_resource(crate::sim::rounds::GroundLine::default());
        world.insert_resource(crate::sim::save::SaveData::default());
        world
    }

    fn spawn_root(world: &mut World, team: Team, pos: Vec2) -> (Entity, Entity) {
        let torso = world
            .spawn((
                RapierBody2d {
                    spawn_pos: pos,
                    ..Default::default()
                },
                ImpulseRequest::default(),
                BodySnapshot2d {
                    pos,
                    ..Default::default()
                },
            ))
            .id();
        let root = world
            .spawn(WarriorRoot {
                team,
                health: 100,
                max_health: 100,
                is_dead: false,
                torso,
                ..warrior_defaults()
            })
            .id();
        (root, torso)
    }

    fn warrior_defaults() -> WarriorRoot {
        WarriorRoot {
            team: Team::Enemy,
            health: 100,
            max_health: 100,
            is_dead: false,
            torso: Entity::PLACEHOLDER,
            damage_mult: 1.0,
            velocity_mult: 1.0,
            knockback_mult: 1.0,
            headshot_mult: 1.0,
            draw_speed_mult: 1.0,
            arrow_count: 1,
            spread_deg: 10.0,
            damage_taken_mult: 1.0,
            crit_chance: 0.0,
            crit_mult: 2.0,
            kill_heal: 0,
            revives: 0,
            last_stand: false,
        }
    }

    #[test]
    fn arrow_flight_integrates_gravity() {
        let mut world = test_world();
        let (shooter, _) = spawn_root(&mut world, Team::Player, Vec2::new(-350.0, 0.0));
        spawn_arrow(
            &mut world,
            Vec2::ZERO,
            Vec2::new(500.0, 0.0),
            shooter,
            Team::Player,
            20.0,
            1.0,
            1.0,
            1.0,
            0.0,
            2.0,
        );
        update_arrows(&mut world);
        let arrow = world.query::<&Arrow>().iter(&world).next().unwrap();
        assert!(arrow.velocity.y > 0.0);
        assert!(arrow.pos.x > 0.0);
    }

    #[test]
    fn arrow_hits_limb_and_kills() {
        let mut world = test_world();
        let (shooter, _) = spawn_root(&mut world, Team::Player, Vec2::new(-350.0, 0.0));
        let (target, _) = spawn_root(&mut world, Team::Enemy, Vec2::new(100.0, 0.0));
        let limb = world
            .spawn((
                WarriorLimb {
                    root: target,
                    kind: LimbKind::Torso,
                    hit_radius: 16.0,
                    size: Vec2::new(30.0, 52.0),
                },
                BodySnapshot2d {
                    pos: Vec2::new(100.0, 0.0),
                    ..Default::default()
                },
            ))
            .id();
        world.entity_mut(target).insert(PlayerTag);
        spawn_arrow(
            &mut world,
            Vec2::new(90.0, 0.0),
            Vec2::new(600.0, 0.0),
            shooter,
            Team::Player,
            200.0,
            1.0,
            1.0,
            1.0,
            0.0,
            2.0,
        );
        update_arrows(&mut world);
        let warrior = world.get::<WarriorRoot>(target).unwrap();
        assert!(warrior.is_dead);
        let arrow = world.query::<&Arrow>().iter(&world).next().unwrap();
        assert!(arrow.has_hit);
        assert_eq!(arrow.stuck_to, Some(limb));
        assert!(world.get::<HitStun>(target).is_some());
    }

    #[test]
    fn arrow_sticks_to_ground() {
        let mut world = test_world();
        let (shooter, _) = spawn_root(&mut world, Team::Player, Vec2::new(-350.0, 0.0));
        spawn_arrow(
            &mut world,
            Vec2::new(0.0, -300.0),
            Vec2::new(0.0, 600.0),
            shooter,
            Team::Player,
            20.0,
            1.0,
            1.0,
            1.0,
            0.0,
            2.0,
        );
        for _ in 0..60 {
            update_arrows(&mut world);
        }
        let arrow = world.query::<&Arrow>().iter(&world).next().unwrap();
        assert!(arrow.has_hit);
        assert_eq!(arrow.stuck_to, None);
    }
}
