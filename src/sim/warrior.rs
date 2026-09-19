use bevy_ecs::prelude::*;
use glam::Vec2;
use repame_rapier2d::{BodySnapshot2d, ImpulseRequest, RapierBody2d, RapierRevolute};
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use super::comps::*;
use super::state::SimStep;

pub const MAX_DRAW: f32 = 100.0;
pub const MIN_FORCE: f32 = 100.0;
pub const MAX_FORCE_ADD: f32 = 1000.0;
pub const HIP_LIMIT: f32 = 0.035;

pub const SKIN_TINT: [f32; 4] = [0.96, 0.82, 0.68, 1.0];
pub const SKIN_DARK_TINT: [f32; 4] = [0.86, 0.7, 0.55, 1.0];
pub const CLOTH_TINT: [f32; 4] = [0.3, 0.42, 0.62, 1.0];

pub struct SpawnWarrior {
    pub translation: Vec2,
    pub team: Team,
    pub mods: CombatMods,
    pub base_hp: i32,
    pub scale: f32,
    pub tint: [f32; 4],
}

pub fn spawn_warrior(world: &mut World, cfg: SpawnWarrior) -> Entity {
    let hp = (cfg.base_hp + cfg.mods.max_hp_bonus).max(1);
    let s = cfg.scale.max(0.2);

    let root = world
        .spawn((
            WarriorRoot {
                team: cfg.team,
                health: hp,
                max_health: hp,
                is_dead: false,
                torso: Entity::PLACEHOLDER, // filled below
                damage_mult: cfg.mods.damage_mult,
                velocity_mult: cfg.mods.velocity_mult,
                knockback_mult: cfg.mods.knockback_mult,
                headshot_mult: cfg.mods.headshot_mult,
                draw_speed_mult: cfg.mods.draw_speed_mult,
                arrow_count: cfg.mods.arrow_count,
                spread_deg: cfg.mods.spread_deg,
                damage_taken_mult: cfg.mods.damage_taken_mult,
                crit_chance: cfg.mods.crit_chance,
                crit_mult: cfg.mods.crit_mult,
                kill_heal: 0,
                revives: 0,
                last_stand: false,
            },
            BowState {
                drawing: false,
                draw_power: 0.0,
            },
            BowAim { angle: 0.0 },
            ActivePuppetMotor {
                stand_y: cfg.translation.y,
                hover_strength: 5.0,
                hover_damping: 1.8,
                upright_strength: 10.0,
                upright_damping: 2.4,
            },
            ArchetypeTint { tint: cfg.tint },
        ))
        .id();
    if cfg.team == Team::Player {
        world.entity_mut(root).insert((
            PlayerTag,
            Airdodge {
                impulse: 420.0,
                cooldown: 1.0 * cfg.mods.airdodge_cd_mult,
                remaining: 0.0,
            },
        ));
    } else {
        world.entity_mut(root).insert(EnemyTag);
    }

    let torso = spawn_limb(
        world,
        root,
        cfg.translation,
        LimbKind::Torso,
        Vec2::ZERO,
        Vec2::new(30.0, 52.0) * s,
        1.5 * s,
        16.0 * s,
    );
    let head = spawn_limb(
        world,
        root,
        cfg.translation,
        LimbKind::Head,
        Vec2::new(0.0, 28.0) * s,
        Vec2::new(24.0, 24.0) * s,
        0.8 * s,
        12.0 * s,
    );
    let up_arm_l = spawn_limb(
        world,
        root,
        cfg.translation,
        LimbKind::UpperArmL,
        Vec2::new(-19.0, 15.0) * s,
        Vec2::new(13.0, 28.0) * s,
        0.45 * s,
        8.0 * s,
    );
    let up_arm_r = spawn_limb(
        world,
        root,
        cfg.translation,
        LimbKind::UpperArmR,
        Vec2::new(19.0, 15.0) * s,
        Vec2::new(13.0, 28.0) * s,
        0.45 * s,
        8.0 * s,
    );
    let arm_l = spawn_limb(
        world,
        root,
        cfg.translation,
        LimbKind::ArmL,
        Vec2::new(-30.0, -8.0) * s,
        Vec2::new(13.0, 38.0) * s,
        0.35 * s,
        8.0 * s,
    );
    let arm_r = spawn_limb(
        world,
        root,
        cfg.translation,
        LimbKind::ArmR,
        Vec2::new(30.0, -8.0) * s,
        Vec2::new(13.0, 38.0) * s,
        0.35 * s,
        8.0 * s,
    );
    let leg_l = spawn_limb(
        world,
        root,
        cfg.translation,
        LimbKind::LegL,
        Vec2::new(-8.0, -41.0) * s,
        Vec2::new(13.0, 35.0) * s,
        0.5 * s,
        9.0 * s,
    );
    let leg_r = spawn_limb(
        world,
        root,
        cfg.translation,
        LimbKind::LegR,
        Vec2::new(8.0, -41.0) * s,
        Vec2::new(13.0, 35.0) * s,
        0.5 * s,
        9.0 * s,
    );

    joint(
        world,
        torso,
        head,
        Vec2::new(0.0, 24.0) * s,
        Vec2::new(0.0, -9.0) * s,
        (0.0, 10.0, 6.0),
        [-FRAC_PI_4, FRAC_PI_4],
    );
    joint(
        world,
        torso,
        up_arm_l,
        Vec2::new(-14.0, 12.0) * s,
        Vec2::new(0.0, 13.0) * s,
        (0.0, 2.0, 1.6),
        [-FRAC_PI_2, FRAC_PI_2],
    );
    joint(
        world,
        torso,
        up_arm_r,
        Vec2::new(14.0, 12.0) * s,
        Vec2::new(0.0, 13.0) * s,
        (0.0, 2.0, 1.6),
        [-FRAC_PI_2, FRAC_PI_2],
    );
    joint(
        world,
        up_arm_l,
        arm_l,
        Vec2::new(0.0, -13.0) * s,
        Vec2::new(0.0, 15.0) * s,
        (0.0, 2.0, 1.6),
        [-FRAC_PI_2, FRAC_PI_2],
    );
    joint(
        world,
        up_arm_r,
        arm_r,
        Vec2::new(0.0, -13.0) * s,
        Vec2::new(0.0, 15.0) * s,
        (0.0, 2.0, 1.6),
        [-FRAC_PI_2, FRAC_PI_2],
    );
    joint(
        world,
        torso,
        leg_l,
        Vec2::new(-8.0, -21.0) * s,
        Vec2::new(0.0, 15.0) * s,
        (0.0, 15.0, 10.0),
        [-HIP_LIMIT, HIP_LIMIT],
    );
    joint(
        world,
        torso,
        leg_r,
        Vec2::new(8.0, -21.0) * s,
        Vec2::new(0.0, 15.0) * s,
        (0.0, 15.0, 10.0),
        [-HIP_LIMIT, HIP_LIMIT],
    );

    world.entity_mut(root).insert(super::comps::WarriorRoot {
        torso,
        team: cfg.team,
        health: hp,
        max_health: hp,
        is_dead: false,
        damage_mult: cfg.mods.damage_mult,
        velocity_mult: cfg.mods.velocity_mult,
        knockback_mult: cfg.mods.knockback_mult,
        headshot_mult: cfg.mods.headshot_mult,
        draw_speed_mult: cfg.mods.draw_speed_mult,
        arrow_count: cfg.mods.arrow_count,
        spread_deg: cfg.mods.spread_deg,
        damage_taken_mult: cfg.mods.damage_taken_mult,
        crit_chance: cfg.mods.crit_chance,
        crit_mult: cfg.mods.crit_mult,
        kill_heal: 0,
        revives: 0,
        last_stand: false,
    });
    world.entity_mut(root).insert(super::comps::BowVisual {
        base_size: Vec2::new(12.0, 60.0) * s,
        rest_x: 16.0 * s,
        pull_distance: 10.0 * s,
    });
    if cfg.team == Team::Enemy {
        world.entity_mut(root).insert(super::comps::WorldHealthBar {
            root,
            width: 42.0 * s,
            y_offset: 56.0 * s,
        });
    }

    root
}

fn spawn_limb(
    world: &mut World,
    root: Entity,
    origin: Vec2,
    kind: LimbKind,
    local: Vec2,
    size: Vec2,
    mass: f32,
    hit_radius: f32,
) -> Entity {
    let pos = origin + local;
    world
        .spawn((
            WarriorLimb {
                root,
                kind,
                hit_radius,
                size,
            },
            RapierBody2d {
                spawn_pos: pos,
                spawn_angle: 0.0,
                half_extents: size * 0.5,
                mass,
                linear_damping: 1.2,
                angular_damping: 2.5,
                memberships: 0x0002,
                filter: 0x0001 | 0x0004,
                ..Default::default()
            },
            ImpulseRequest::default(),
            BodySnapshot2d {
                pos,
                ..Default::default()
            },
        ))
        .id()
}

fn joint(
    world: &mut World,
    a: Entity,
    b: Entity,
    anchor_a: Vec2,
    anchor_b: Vec2,
    servo: (f32, f32, f32),
    limits: [f32; 2],
) {
    let (rest_angle, stiffness, damping) = servo;
    world.spawn(RapierRevolute {
        parent: a,
        child: b,
        anchor_a,
        anchor_b,
        rest_angle,
        stiffness,
        damping,
        limits,
    });
}

pub fn apply_ragdoll_on_death(
    mut commands: Commands,
    mut warriors: Query<(Entity, &WarriorRoot), Without<RagdollApplied>>,
    mut impulses: Query<&mut ImpulseRequest>,
    joints: Query<(Entity, &RapierRevolute)>,
    limbs: Query<(Entity, &WarriorLimb)>,
    step: Res<SimStep>,
) {
    for (entity, warrior) in &mut warriors {
        if !warrior.is_dead {
            continue;
        }
        commands.entity(entity).insert(RagdollApplied);
        commands.entity(entity).remove::<ActivePuppetMotor>();
        commands.entity(entity).remove::<HitStun>();
        commands.entity(entity).remove::<Recovery>();
        for (joint_e, joint) in &joints {
            let is_limb = |e: Entity| limbs.get(e).is_ok_and(|(_, l)| l.root == entity);
            if is_limb(joint.parent) && is_limb(joint.child) {
                commands.entity(joint_e).despawn();
            }
        }
        if let Ok(mut impulse) = impulses.get_mut(warrior.torso) {
            let mut rng = rand::rng();
            use rand::RngExt;
            impulse.linear +=
                Vec2::new(rng.random_range(-90.0..90.0), rng.random_range(40.0..140.0));
            impulse.torque += rng.random_range(-12.0..12.0);
        }
        let _ = step;
    }
}

pub fn active_puppet_motor(
    step: Res<SimStep>,
    warriors: Query<
        (
            &WarriorRoot,
            &ActivePuppetMotor,
            Option<&HitStun>,
            Option<&Recovery>,
        ),
        Without<RagdollApplied>,
    >,
    snapshots: Query<&BodySnapshot2d>,
    mut impulses: Query<&mut ImpulseRequest>,
) {
    let dt = step.dt.clamp(0.001, 0.05);
    for (warrior, motor, hitstun, recovery) in &warriors {
        if warrior.is_dead {
            continue;
        }
        if let Some(stun) = hitstun
            && stun.remaining > 0.0
        {
            continue;
        }
        let recovering = recovery.is_some_and(|r| r.remaining > 0.0);
        let Ok(snapshot) = snapshots.get(warrior.torso) else {
            continue;
        };
        let Ok(mut impulse) = impulses.get_mut(warrior.torso) else {
            continue;
        };
        if !recovering {
            let y_error = motor.stand_y - snapshot.pos.y;
            let downward = snapshot.linvel.y.min(0.0);
            let y_impulse = y_error * motor.hover_strength - downward * motor.hover_damping;
            impulse.linear += Vec2::Y * y_impulse * dt;
        }
        let angle_error = wrap_angle(snapshot.angle);
        let torque =
            -angle_error * motor.upright_strength - snapshot.angvel * motor.upright_damping;
        impulse.torque += torque * dt;
    }
}

pub fn tick_motor_state(
    step: Res<SimStep>,
    mut q: Query<&mut HitStun>,
    mut rq: Query<&mut Recovery>,
) {
    let dt = step.dt;
    for mut stun in &mut q {
        stun.remaining = (stun.remaining - dt).max(0.0);
    }
    for mut recovery in &mut rq {
        recovery.remaining = (recovery.remaining - dt).max(0.0);
    }
}

pub fn tick_motor_state_by_world(world: &mut World) {
    let dt = world.resource::<SimStep>().dt;
    let stuns: Vec<Entity> = world
        .query::<(Entity, &HitStun)>()
        .iter(world)
        .map(|(e, _)| e)
        .collect();
    for e in stuns {
        if let Some(mut s) = world.get_mut::<HitStun>(e) {
            s.remaining = (s.remaining - dt).max(0.0);
        }
    }
    let recs: Vec<Entity> = world
        .query::<(Entity, &Recovery)>()
        .iter(world)
        .map(|(e, _)| e)
        .collect();
    for e in recs {
        if let Some(mut r) = world.get_mut::<Recovery>(e) {
            r.remaining = (r.remaining - dt).max(0.0);
        }
    }
}

fn wrap_angle(mut angle: f32) -> f32 {
    while angle > std::f32::consts::PI {
        angle -= std::f32::consts::TAU;
    }
    while angle < -std::f32::consts::PI {
        angle += std::f32::consts::TAU;
    }
    angle
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::state::{DyingFade, GameOverFlushed};
    use bevy_ecs::system::RunSystemOnce;

    #[test]
    fn warrior_spawns_eight_limbs() {
        let mut world = World::new();
        let root = spawn_warrior(
            &mut world,
            SpawnWarrior {
                translation: Vec2::new(-350.0, crate::sim::arena::STAND_Y),
                team: Team::Player,
                mods: CombatMods::default(),
                base_hp: 100,
                scale: 1.0,
                tint: [1.0, 1.0, 1.0, 1.0],
            },
        );
        let limbs: Vec<_> = world.query::<&WarriorLimb>().iter(&world).collect();
        assert_eq!(limbs.len(), 8);
        let joints: Vec<_> = world.query::<&RapierRevolute>().iter(&world).collect();
        assert_eq!(joints.len(), 7);
        let warrior = world.get::<WarriorRoot>(root).unwrap();
        assert_eq!(warrior.health, 100);
        assert!(world.get::<PlayerTag>(root).is_some());
        assert!(world.get::<Airdodge>(root).is_some());
    }

    #[test]
    fn ragdoll_marks_dead_and_clears_motor() {
        let mut world = World::new();
        world.insert_resource(SimStep { dt: 1.0 / 60.0 });
        let root = spawn_warrior(
            &mut world,
            SpawnWarrior {
                translation: Vec2::ZERO,
                team: Team::Enemy,
                mods: CombatMods::default(),
                base_hp: 100,
                scale: 1.0,
                tint: [1.0, 1.0, 1.0, 1.0],
            },
        );
        world.get_mut::<WarriorRoot>(root).unwrap().is_dead = true;
        let _ = world.run_system_once(apply_ragdoll_on_death);
        assert!(world.get::<RagdollApplied>(root).is_some());
        assert!(world.get::<ActivePuppetMotor>(root).is_none());
    }

    #[test]
    fn motor_pushes_up_when_below_stand() {
        let mut world = World::new();
        world.insert_resource(SimStep { dt: 1.0 / 60.0 });
        let root = spawn_warrior(
            &mut world,
            SpawnWarrior {
                translation: Vec2::new(0.0, -102.0),
                team: Team::Player,
                mods: CombatMods::default(),
                base_hp: 100,
                scale: 1.0,
                tint: [1.0, 1.0, 1.0, 1.0],
            },
        );
        let torso = world.get::<WarriorRoot>(root).unwrap().torso;
        world.get_mut::<BodySnapshot2d>(torso).unwrap().pos = Vec2::new(0.0, -150.0);
        let _ = world.run_system_once(active_puppet_motor);
        let impulse = world.get::<ImpulseRequest>(torso).unwrap();
        assert!(impulse.linear.y > 0.0);
    }

    #[test]
    fn hitstun_suspends_motor() {
        let mut world = World::new();
        world.insert_resource(SimStep { dt: 1.0 / 60.0 });
        let root = spawn_warrior(
            &mut world,
            SpawnWarrior {
                translation: Vec2::new(0.0, -102.0),
                team: Team::Player,
                mods: CombatMods::default(),
                base_hp: 100,
                scale: 1.0,
                tint: [1.0, 1.0, 1.0, 1.0],
            },
        );
        let torso = world.get::<WarriorRoot>(root).unwrap().torso;
        world.entity_mut(root).insert(HitStun { remaining: 0.2 });
        world.get_mut::<BodySnapshot2d>(torso).unwrap().pos = Vec2::new(0.0, -150.0);
        let _ = world.run_system_once(active_puppet_motor);
        let impulse = world.get::<ImpulseRequest>(torso).unwrap();
        assert_eq!(impulse.linear, Vec2::ZERO);
        let _ = DyingFade(0.0);
    }
}
