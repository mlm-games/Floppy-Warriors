use bevy::prelude::*;
#[cfg(feature = "physics")]
use bevy_rapier2d::prelude::*;
use super::components::*;

const DRAW_SPEED: f32 = 160.0;
const MAX_DRAW: f32 = 100.0;
const MIN_FORCE: f32 = 100.0;
const MAX_FORCE_ADD: f32 = 1000.0;

pub struct SpawnWarrior {
    pub translation: Vec2,
    pub team: Team,
    pub mods: CombatMods,
    pub base_hp: i32,
}

pub fn spawn_warrior(commands: &mut Commands, cfg: SpawnWarrior) -> Entity {
    let hp = (cfg.base_hp + cfg.mods.max_hp_bonus).max(1);
    let root = commands
        .spawn((
            GameCleanup,
            Transform::from_translation(cfg.translation.extend(0.0)),
            Visibility::default(),
            InheritedVisibility::default(),
            Name::new(if cfg.team == Team::Player {
                "PlayerWarrior"
            } else {
                "EnemyWarrior"
            }),
        ))
        .id();

    // --- limbs (local offsets match Godot body) ---
    let torso = spawn_limb(
        commands,
        root,
        LimbKind::Torso,
        Vec2::ZERO,
        Vec2::new(20.0, 40.0),
        1.5,
        Color::srgb(0.35, 0.55, 0.85),
    );
    let head = spawn_limb(
        commands,
        root,
        LimbKind::Head,
        Vec2::new(0.0, 30.0),
        Vec2::splat(18.0),
        0.8,
        Color::srgb(0.9, 0.75, 0.55),
    );
    let arm_l = spawn_limb(
        commands,
        root,
        LimbKind::ArmL,
        Vec2::new(-13.0, 7.0),
        Vec2::new(8.0, 26.0),
        0.4,
        Color::srgb(0.9, 0.7, 0.5),
    );
    let arm_r = spawn_limb(
        commands,
        root,
        LimbKind::ArmR,
        Vec2::new(14.0, 7.0),
        Vec2::new(8.0, 26.0),
        0.4,
        Color::srgb(0.9, 0.7, 0.5),
    );
    let leg_l = spawn_limb(
        commands,
        root,
        LimbKind::LegL,
        Vec2::new(-7.0, -33.0),
        Vec2::new(8.0, 28.0),
        0.5,
        Color::srgb(0.45, 0.4, 0.55),
    );
    let leg_r = spawn_limb(
        commands,
        root,
        LimbKind::LegR,
        Vec2::new(7.0, -33.0),
        Vec2::new(8.0, 28.0),
        0.5,
        Color::srgb(0.45, 0.4, 0.55),
    );

    #[cfg(feature = "physics")]
    {
        // Revolute joints — soft floppy limits.
        joint(commands, torso, head, Vec2::new(0.0, 20.0), Vec2::new(0.0, -9.0));
        joint(commands, torso, arm_l, Vec2::new(-10.0, 12.0), Vec2::new(0.0, 10.0));
        joint(commands, torso, arm_r, Vec2::new(10.0, 12.0), Vec2::new(0.0, 10.0));
        joint(commands, torso, leg_l, Vec2::new(-7.0, -18.0), Vec2::new(0.0, 12.0));
        joint(commands, torso, leg_r, Vec2::new(7.0, -18.0), Vec2::new(0.0, 12.0));
    }

    let bow_pivot = commands
        .spawn((
            GameCleanup,
            Transform::from_xyz(0.0, 10.0, 2.0),
            Visibility::default(),
            InheritedVisibility::default(),
            ChildOf(torso),
        ))
        .with_children(|b| {
            b.spawn((
                GameCleanup,
                Sprite {
                    color: Color::srgb(0.55, 0.35, 0.15),
                    custom_size: Some(Vec2::new(28.0, 8.0)),
                    ..default()
                },
                Transform::from_xyz(18.0, 0.0, 0.1),
            ));
        })
        .id();

    let warrior = WarriorRoot {
        team: cfg.team,
        health: hp,
        max_health: hp,
        is_dead: false,
        torso,
        bow_pivot,
        damage_mult: cfg.mods.damage_mult,
        velocity_mult: cfg.mods.velocity_mult,
        knockback_mult: cfg.mods.knockback_mult,
        headshot_mult: cfg.mods.headshot_mult,
        draw_speed_mult: cfg.mods.draw_speed_mult,
        arrow_count: cfg.mods.arrow_count,
        spread_deg: cfg.mods.spread_deg,
    };
    commands.entity(root).insert((
        warrior,
        BowState {
            drawing: false,
            draw_power: 0.0,
        },
    ));

    if cfg.team == Team::Player {
        commands.entity(root).insert((
            PlayerTag,
            Airdodge {
                impulse: 320.0,
                cooldown: 1.0 * cfg.mods.airdodge_cd_mult,
                remaining: 0.0,
            },
        ));
    } else {
        commands.entity(root).insert(EnemyTag);
    }

    root
}

fn spawn_limb(
    commands: &mut Commands,
    root: Entity,
    kind: LimbKind,
    local: Vec2,
    size: Vec2,
    mass: f32,
    color: Color,
) -> Entity {
    let mut e = commands.spawn((
        GameCleanup,
        WarriorLimb { root, kind },
        Sprite {
            color,
            custom_size: Some(size),
            ..default()
        },
        Transform::from_translation(local.extend(1.0)),
        Visibility::default(),
        InheritedVisibility::default(),
        ChildOf(root),
    ));
    #[cfg(feature = "physics")]
    {
        e.insert((
            RigidBody::Dynamic,
            Collider::cuboid(size.x * 0.5, size.y * 0.5),
            AdditionalMassProperties::Mass(mass),
            Damping {
                linear_damping: 1.2,
                angular_damping: 2.5,
            },
            ExternalImpulse::default(),
            // Collision layers: warriors vs world+arrows
            CollisionGroups::new(
                Group::GROUP_2,
                Group::GROUP_1 | Group::GROUP_3 | Group::GROUP_4,
            ),
        ));
    }
    e.id()
}

#[cfg(feature = "physics")]
fn joint(commands: &mut Commands, a: Entity, b: Entity, anchor_a: Vec2, anchor_b: Vec2) {
    let joint = RevoluteJointBuilder::new()
        .local_anchor1(anchor_a)
        .local_anchor2(anchor_b);
    commands.entity(b).insert(ImpulseJoint::new(a, joint));
}

pub fn fire_from_bow(
    commands: &mut Commands,
    warrior_e: Entity,
    warrior: &WarriorRoot,
    bow: &BowState,
    bow_tf: &GlobalTransform,
) {
    if warrior.is_dead || !bow.drawing {
        return;
    }
    let force = MIN_FORCE + (bow.draw_power / MAX_DRAW) * MAX_FORCE_ADD;
    let base_angle = bow_tf.rotation().to_euler(EulerRot::ZXY).0;
    let origin = bow_tf.translation().truncate() + Vec2::from_angle(base_angle) * 24.0;
    let n = warrior.arrow_count.max(1);
    for i in 0..n {
        let mut angle = base_angle;
        if n > 1 {
            let step = warrior.spread_deg / (n - 1) as f32;
            let off = -warrior.spread_deg * 0.5 + step * i as f32;
            angle += off.to_radians();
        }
        let dir = Vec2::from_angle(angle);
        super::arrow::spawn_arrow(
            commands,
            origin,
            dir * force * warrior.velocity_mult,
            warrior_e,
            warrior.team,
            20.0,
            warrior.damage_mult,
            warrior.headshot_mult,
            warrior.knockback_mult,
        );
    }
}

pub fn launch_force_constants() -> (f32, f32, f32) {
    (MIN_FORCE, MAX_FORCE_ADD, MAX_DRAW)
}