use bevy::prelude::*;
#[cfg(feature = "physics")]
use bevy_rapier2d::prelude::*;
use rand::RngExt;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use super::art::WarriorArt;
use super::components::*;

const MAX_DRAW: f32 = 100.0;
const MIN_FORCE: f32 = 100.0;
const MAX_FORCE_ADD: f32 = 1000.0;
const HIP_LIMIT: f32 = 0.035;

pub struct SpawnWarrior {
    pub translation: Vec2,
    pub team: Team,
    pub mods: CombatMods,
    pub base_hp: i32,
    pub scale: f32,
}

const SKIN: Color = Color::srgb(0.96, 0.82, 0.68);
const SKIN_DARK: Color = Color::srgb(0.86, 0.7, 0.55);
const CLOTH: Color = Color::srgb(0.3, 0.42, 0.62);

pub fn spawn_warrior(commands: &mut Commands, art: &WarriorArt, cfg: SpawnWarrior) -> Entity {
    let hp = (cfg.base_hp + cfg.mods.max_hp_bonus).max(1);
    let s = cfg.scale.max(0.2);

    let root = commands
        .spawn((
            GameCleanup,
            Name::new(if cfg.team == Team::Player {
                "PlayerWarrior"
            } else {
                "EnemyWarrior"
            }),
            Transform::from_translation(cfg.translation.extend(0.0)),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
        ))
        .id();

    let torso = spawn_limb(
        commands,
        root,
        LimbKind::Torso,
        &art.body,
        Vec2::ZERO,
        Vec2::new(30.0, 52.0) * s,
        1.5 * s,
        16.0 * s,
        CLOTH,
    );
    let head = spawn_limb(
        commands,
        root,
        LimbKind::Head,
        &art.head,
        Vec2::new(0.0, 28.0) * s,
        Vec2::new(24.0, 24.0) * s,
        0.8 * s,
        12.0 * s,
        SKIN,
    );
    let up_arm_l = spawn_limb(
        commands,
        root,
        LimbKind::UpperArmL,
        &art.upper_arm,
        Vec2::new(-19.0, 15.0) * s,
        Vec2::new(13.0, 28.0) * s,
        0.45 * s,
        8.0 * s,
        SKIN_DARK,
    );
    let up_arm_r = spawn_limb(
        commands,
        root,
        LimbKind::UpperArmR,
        &art.upper_arm,
        Vec2::new(19.0, 15.0) * s,
        Vec2::new(13.0, 28.0) * s,
        0.45 * s,
        8.0 * s,
        SKIN_DARK,
    );
    let arm_l = spawn_limb(
        commands,
        root,
        LimbKind::ArmL,
        &art.hand,
        Vec2::new(-30.0, -8.0) * s,
        Vec2::new(13.0, 38.0) * s,
        0.35 * s,
        8.0 * s,
        SKIN,
    );
    let arm_r = spawn_limb(
        commands,
        root,
        LimbKind::ArmR,
        &art.hand,
        Vec2::new(30.0, -8.0) * s,
        Vec2::new(13.0, 38.0) * s,
        0.35 * s,
        8.0 * s,
        SKIN,
    );
    let leg_l = spawn_limb(
        commands,
        root,
        LimbKind::LegL,
        &art.leg,
        Vec2::new(-8.0, -41.0) * s,
        Vec2::new(13.0, 35.0) * s,
        0.5 * s,
        9.0 * s,
        SKIN_DARK,
    );
    let leg_r = spawn_limb(
        commands,
        root,
        LimbKind::LegR,
        &art.leg,
        Vec2::new(8.0, -41.0) * s,
        Vec2::new(13.0, 35.0) * s,
        0.5 * s,
        9.0 * s,
        SKIN_DARK,
    );

    #[cfg(feature = "physics")]
    {
        // Limits ported 1:1 from the original Godot PinJoint2D angular_limit values.
        joint(
            commands,
            torso,
            head,
            Vec2::new(0.0, 24.0) * s,
            Vec2::new(0.0, -9.0) * s,
            (0.0, 10.0, 6.0), // neck
            [-FRAC_PI_4, FRAC_PI_4],
        ); // ±45°
        joint(
            commands,
            torso,
            up_arm_l,
            Vec2::new(-14.0, 12.0) * s,
            Vec2::new(0.0, 13.0) * s,
            (0.0, 2.0, 1.6), // shoulder, light so knockback flings visibly
            [-FRAC_PI_2, FRAC_PI_2],
        ); // ±90°
        joint(
            commands,
            torso,
            up_arm_r,
            Vec2::new(14.0, 12.0) * s,
            Vec2::new(0.0, 13.0) * s,
            (0.0, 2.0, 1.6),
            [-FRAC_PI_2, FRAC_PI_2],
        ); // ±90°
        joint(
            commands,
            up_arm_l,
            arm_l,
            Vec2::new(0.0, -13.0) * s,
            Vec2::new(0.0, 15.0) * s,
            (0.0, 2.0, 1.6), // elbow
            [-FRAC_PI_2, FRAC_PI_2],
        );
        joint(
            commands,
            up_arm_r,
            arm_r,
            Vec2::new(0.0, -13.0) * s,
            Vec2::new(0.0, 15.0) * s,
            (0.0, 2.0, 1.6),
            [-FRAC_PI_2, FRAC_PI_2],
        );
        joint(
            commands,
            torso,
            leg_l,
            Vec2::new(-8.0, -21.0) * s,
            Vec2::new(0.0, 15.0) * s,
            (0.0, 15.0, 10.0), // hips stay planted
            [-HIP_LIMIT, HIP_LIMIT],
        );
        joint(
            commands,
            torso,
            leg_r,
            Vec2::new(8.0, -21.0) * s,
            Vec2::new(0.0, 15.0) * s,
            (0.0, 15.0, 10.0),
            [-HIP_LIMIT, HIP_LIMIT],
        );
    }

    let bow_base = Vec2::new(12.0, 60.0) * s;
    let bow_rest_x = 16.0 * s;
    let bow_pull = 10.0 * s;

    let bow_pivot = commands
        .spawn((
            GameCleanup,
            Transform::from_xyz(0.0, 10.0 * s, 2.0),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ChildOf(torso),
        ))
        .with_children(|b| {
            b.spawn((
                GameCleanup,
                SkipTint,
                BowVisual {
                    base_size: bow_base,
                    rest_x: bow_rest_x,
                    pull_distance: bow_pull,
                },
                Sprite {
                    image: art.bow.clone(),
                    color: Color::WHITE,
                    custom_size: Some(bow_base),
                    flip_x: true,
                    ..default()
                },
                Transform {
                    translation: Vec3::new(bow_rest_x, 0.0, 0.1),
                    ..default()
                },
                GlobalTransform::default(),
                Visibility::default(),
                InheritedVisibility::default(),
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
        damage_taken_mult: cfg.mods.damage_taken_mult,
        crit_chance: cfg.mods.crit_chance,
        crit_mult: cfg.mods.crit_mult,
        kill_heal: 0,
        revives: 0,
        last_stand: false,
    };

    commands.entity(root).insert((
        warrior,
        BowState {
            drawing: false,
            draw_power: 0.0,
        },
        ActivePuppetMotor {
            stand_y: cfg.translation.y,
            hover_strength: 5.0,
            hover_damping: 1.8,
            upright_strength: 10.0,
            upright_damping: 2.4,
        },
    ));

    if cfg.team == Team::Player {
        commands.entity(root).insert((
            PlayerTag,
            Airdodge {
                impulse: 420.0,
                cooldown: 1.0 * cfg.mods.airdodge_cd_mult,
                remaining: 0.0,
            },
        ));
    } else {
        commands.entity(root).insert(EnemyTag);
        spawn_world_health_bar(commands, root, s);
    }

    root
}

fn spawn_limb(
    commands: &mut Commands,
    root: Entity,
    kind: LimbKind,
    tex: &Handle<Image>,
    local: Vec2,
    size: Vec2,
    mass: f32,
    hit_radius: f32,
    color: Color,
) -> Entity {
    let mut e = commands.spawn((
        GameCleanup,
        WarriorLimb {
            root,
            kind,
            hit_radius,
        },
        Sprite {
            image: tex.clone(),
            color,
            custom_size: Some(size),
            ..default()
        },
        Transform::from_translation(local.extend(1.0)),
        GlobalTransform::default(),
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
            Velocity::default(),
            CollisionGroups::new(Group::GROUP_2, Group::GROUP_1 | Group::GROUP_3),
        ));
    }

    e.id()
}

#[cfg(feature = "physics")]
fn joint(
    commands: &mut Commands,
    a: Entity,
    b: Entity,
    anchor_a: Vec2,
    anchor_b: Vec2,
    servo: (f32, f32, f32),
    limits: [f32; 2],
) {
    let (rest_angle, stiffness, damping) = servo;
    let joint = RevoluteJointBuilder::new()
        .local_anchor1(anchor_a)
        .local_anchor2(anchor_b)
        .motor_position(rest_angle, stiffness, damping)
        .limits(limits);
    commands.entity(b).insert(ImpulseJoint::new(a, joint));
}

fn spawn_world_health_bar(commands: &mut Commands, root: Entity, scale: f32) {
    let width = 42.0 * scale;

    let bg = commands
        .spawn((
            GameCleanup,
            Sprite {
                color: Color::srgba(0.12, 0.12, 0.14, 0.95),
                custom_size: Some(Vec2::new(width + 2.0, 8.0)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, 30.0),
            GlobalTransform::default(),
            WorldHealthBar {
                root,
                fill: Entity::PLACEHOLDER,
                width,
                y_offset: 56.0 * scale,
            },
        ))
        .id();

    let fill = commands
        .spawn((
            GameCleanup,
            Sprite {
                color: Color::srgb(0.28, 0.85, 0.4),
                custom_size: Some(Vec2::new(width, 6.0)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, 0.1),
            GlobalTransform::default(),
            ChildOf(bg),
        ))
        .id();

    commands.entity(bg).insert(WorldHealthBar {
        root,
        fill,
        width,
        y_offset: 56.0 * scale,
    });
}

pub fn sync_world_health_bars(
    mut commands: Commands,
    mut bars: Query<(Entity, &WorldHealthBar, &mut Transform, &mut Sprite)>,
    warriors: Query<&WarriorRoot>,
    global_tf: Query<&GlobalTransform>,
) {
    for (bar_entity, bar, mut bar_tf, mut bg_sprite) in &mut bars {
        let Ok(warrior) = warriors.get(bar.root) else {
            commands.entity(bar_entity).despawn();
            continue;
        };

        if warrior.is_dead {
            bg_sprite.color.set_alpha(0.0);
            continue;
        }

        bg_sprite.color.set_alpha(0.95);

        let Ok(torso_tf) = global_tf.get(warrior.torso) else {
            continue;
        };

        bar_tf.translation = torso_tf.translation() + Vec3::new(0.0, bar.y_offset, 30.0);
    }
}

pub fn sync_health_fills(
    bars: Query<&WorldHealthBar>,
    warriors: Query<&WarriorRoot>,
    mut fill_tf: Query<&mut Transform>,
    mut fill_sprites: Query<&mut Sprite>,
) {
    for bar in &bars {
        let Ok(warrior) = warriors.get(bar.root) else {
            continue;
        };

        let pct = (warrior.health.max(0) as f32 / warrior.max_health.max(1) as f32).clamp(0.0, 1.0);
        let fill_width = (bar.width * pct).max(0.001);

        if let Ok(mut sprite) = fill_sprites.get_mut(bar.fill) {
            sprite.custom_size = Some(Vec2::new(fill_width, 6.0));
            sprite.color = if warrior.is_dead {
                Color::srgba(0.28, 0.85, 0.4, 0.0)
            } else if pct > 0.6 {
                Color::srgb(0.28, 0.85, 0.4)
            } else if pct > 0.3 {
                Color::srgb(0.95, 0.78, 0.2)
            } else {
                Color::srgb(0.92, 0.28, 0.25)
            };
        }

        if let Ok(mut tf) = fill_tf.get_mut(bar.fill) {
            tf.translation.x = -0.5 * (bar.width - fill_width);
        }
    }
}

/// Stretch / pull the bow sprite from `BowState.draw_power`.
/// Safe for player + enemies; resets when not drawing or dead.
pub fn sync_bow_draw_visuals(
    warriors: Query<(&WarriorRoot, &BowState)>,
    children: Query<&Children>,
    mut visuals: Query<(&BowVisual, &mut Sprite, &mut Transform)>,
) {
    const MAX_DRAW: f32 = 100.0;

    for (warrior, bow) in &warriors {
        let Ok(kids) = children.get(warrior.bow_pivot) else {
            continue;
        };

        let raw = if warrior.is_dead || !bow.drawing {
            0.0
        } else {
            (bow.draw_power / MAX_DRAW).clamp(0.0, 1.0)
        };

        let t = raw * raw * (3.0 - 2.0 * raw);

        for child in kids.iter() {
            let Ok((vis, mut sprite, mut tf)) = visuals.get_mut(child) else {
                continue;
            };

            let size = Vec2::new(
                vis.base_size.x * (1.0 + 0.12 * t),
                vis.base_size.y * (1.0 + 0.55 * t),
            );
            sprite.custom_size = Some(size);

            tf.translation.x = vis.rest_x - vis.pull_distance * t;
            tf.translation.y = 0.0;

            let heat = t;
            sprite.color = Color::srgb(1.0, 1.0 - 0.18 * heat, 1.0 - 0.42 * heat);
        }
    }
}

pub fn fire_from_bow_angled(
    commands: &mut Commands,
    asset_server: &AssetServer,
    sfx: &super::audio_fx::CombatSfx,
    warrior_e: Entity,
    warrior: &WarriorRoot,
    bow: &BowState,
    origin: Vec2,
    base_angle: f32,
) {
    if warrior.is_dead || !bow.drawing {
        return;
    }

    super::audio_fx::play_sfx(commands, asset_server, &sfx.bow_release, 0.45, 0.08);

    let mut damage_mult = warrior.damage_mult;

    if warrior.last_stand {
        let hp_pct = warrior.health.max(0) as f32 / warrior.max_health.max(1) as f32;
        if hp_pct <= 0.35 {
            damage_mult *= 1.75;
        }
    }

    let force = MIN_FORCE + (bow.draw_power / MAX_DRAW) * MAX_FORCE_ADD;
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
            damage_mult,
            warrior.headshot_mult,
            warrior.knockback_mult,
            warrior.crit_chance,
            warrior.crit_mult,
        );
    }
}

#[cfg(feature = "physics")]
pub fn apply_ragdoll_on_death(
    mut commands: Commands,
    warriors: Query<(Entity, &WarriorRoot), Without<RagdollApplied>>,
    mut impulses: Query<&mut ExternalImpulse>,
    limbs: Query<(Entity, &WarriorLimb, Has<ImpulseJoint>)>,
) {
    for (entity, warrior) in &warriors {
        if !warrior.is_dead {
            continue;
        }

        commands.entity(entity).insert(RagdollApplied);
        commands.entity(entity).remove::<ActivePuppetMotor>();
        commands.entity(entity).remove::<HitStun>();
        commands.entity(entity).remove::<Recovery>();
        commands.entity(warrior.torso).remove::<LockedAxes>();

        for (limb_entity, limb, has_joint) in &limbs {
            if limb.root == entity && has_joint {
                commands.entity(limb_entity).remove::<ImpulseJoint>();
            }
        }

        if let Ok(mut impulse) = impulses.get_mut(warrior.torso) {
            let mut rng = rand::rng();

            impulse.impulse +=
                Vec2::new(rng.random_range(-90.0..90.0), rng.random_range(40.0..140.0));

            impulse.torque_impulse += rng.random_range(-12.0..12.0);
        }
    }
}

#[cfg(not(feature = "physics"))]
pub fn apply_ragdoll_on_death(
    mut commands: Commands,
    warriors: Query<(Entity, &WarriorRoot), Without<RagdollApplied>>,
) {
    for (entity, warrior) in &warriors {
        if warrior.is_dead {
            commands.entity(entity).insert(RagdollApplied);
            commands.entity(entity).remove::<ActivePuppetMotor>();
            commands.entity(entity).remove::<HitStun>();
            commands.entity(entity).remove::<Recovery>();
        }
    }
}

#[cfg(feature = "physics")]
pub fn active_puppet_motor(
    time: Res<Time>,
    warriors: Query<
        (
            &WarriorRoot,
            &ActivePuppetMotor,
            Option<&HitStun>,
            Option<&Recovery>,
        ),
        Without<RagdollApplied>,
    >,
    global_tf: Query<&GlobalTransform>,
    mut physics: Query<(&mut ExternalImpulse, &Velocity)>,
) {
    let dt = time.delta_secs().clamp(0.001, 0.05);

    for (warrior, motor, hitstun, recovery) in &warriors {
        if warrior.is_dead {
            continue;
        }

        // Knockback hit: suspend all motor control so the impulse reads.
        if let Some(stun) = hitstun
            && stun.remaining > 0.0
        {
            continue;
        }
        let recovering = recovery.is_some_and(|r| r.remaining > 0.0);

        let Ok(torso_tf) = global_tf.get(warrior.torso) else {
            continue;
        };
        let Ok((mut impulse, velocity)) = physics.get_mut(warrior.torso) else {
            continue;
        };

        let torso_pos = torso_tf.translation().truncate();
        let torso_angle = global_z_angle(torso_tf);

        // Hover keeps the warrior near stand_y but only ever damps *downward*
        // velocity, so airdodges / knockback launches are not cancelled mid-air.
        if !recovering {
            let y_error = motor.stand_y - torso_pos.y;
            let downward = velocity.linear.y.min(0.0);
            let y_impulse = y_error * motor.hover_strength - downward * motor.hover_damping;

            impulse.impulse += Vec2::Y * y_impulse * dt;
        }

        let angle_error = wrap_angle(torso_angle);
        let torque =
            -angle_error * motor.upright_strength - velocity.angular * motor.upright_damping;

        impulse.torque_impulse += torque * dt;
    }
}

#[cfg(not(feature = "physics"))]
pub fn active_puppet_motor() {}

pub fn tick_motor_state(time: Res<Time>, mut q: Query<&mut HitStun>, mut rq: Query<&mut Recovery>) {
    let dt = time.delta_secs();
    for mut stun in &mut q {
        stun.remaining = (stun.remaining - dt).max(0.0);
    }
    for mut recovery in &mut rq {
        recovery.remaining = (recovery.remaining - dt).max(0.0);
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

fn global_z_angle(tf: &GlobalTransform) -> f32 {
    let right = (tf.compute_transform().rotation * Vec3::X).truncate();
    right.y.atan2(right.x)
}

/// Runs once when ArchetypeVisual is added; tints all body parts. Physics scale
/// is baked into the body at spawn time (see SpawnWarrior::scale).
pub fn apply_archetype_visuals(
    mut commands: Commands,
    q: Query<(Entity, &ArchetypeVisual), Added<ArchetypeVisual>>,
    children: Query<&Children>,
    mut sprites: Query<&mut Sprite>,
    skip: Query<(), With<SkipTint>>,
) {
    for (root, vis) in &q {
        tint_warrior_tree(root, vis.tint, &children, &mut sprites, &skip);
        commands.entity(root).remove::<ArchetypeVisual>();
    }
}

/// Shared with the title demo so both tints behave identically: every sprite
/// descendant gets the tint except `SkipTint` subtrees (e.g. the bow).
pub fn tint_warrior_tree(
    e: Entity,
    tint: Color,
    children: &Query<&Children>,
    sprites: &mut Query<&mut Sprite>,
    skip: &Query<(), With<SkipTint>>,
) {
    if skip.contains(e) {
        return;
    }
    if let Ok(mut s) = sprites.get_mut(e) {
        s.color = tint;
    }
    if let Ok(kids) = children.get(e) {
        for c in kids.iter() {
            tint_warrior_tree(c, tint, children, sprites, skip);
        }
    }
}
