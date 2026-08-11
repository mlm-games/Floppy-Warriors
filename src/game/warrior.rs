use bevy::prelude::*;
#[cfg(feature = "physics")]
use bevy_rapier2d::prelude::*;
use rand::RngExt;

use super::components::*;

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
        Vec2::ZERO,
        Vec2::new(20.0, 40.0),
        1.5,
        16.0,
        Color::srgb(0.35, 0.55, 0.85),
    );
    let head = spawn_limb(
        commands,
        root,
        LimbKind::Head,
        Vec2::new(0.0, 30.0),
        Vec2::splat(18.0),
        0.8,
        11.0,
        Color::srgb(0.9, 0.75, 0.55),
    );
    let arm_l = spawn_limb(
        commands,
        root,
        LimbKind::ArmL,
        Vec2::new(-13.0, 7.0),
        Vec2::new(8.0, 26.0),
        0.4,
        8.0,
        Color::srgb(0.9, 0.7, 0.5),
    );
    let arm_r = spawn_limb(
        commands,
        root,
        LimbKind::ArmR,
        Vec2::new(14.0, 7.0),
        Vec2::new(8.0, 26.0),
        0.4,
        8.0,
        Color::srgb(0.9, 0.7, 0.5),
    );
    let leg_l = spawn_limb(
        commands,
        root,
        LimbKind::LegL,
        Vec2::new(-7.0, -33.0),
        Vec2::new(8.0, 28.0),
        0.5,
        9.0,
        Color::srgb(0.45, 0.4, 0.55),
    );
    let leg_r = spawn_limb(
        commands,
        root,
        LimbKind::LegR,
        Vec2::new(7.0, -33.0),
        Vec2::new(8.0, 28.0),
        0.5,
        9.0,
        Color::srgb(0.45, 0.4, 0.55),
    );

    #[cfg(feature = "physics")]
    {
        // Limits ported 1:1 from the original Godot PinJoint2D angular_limit values.
        joint(commands, torso, head, Vec2::new(0.0, 20.0), Vec2::new(0.0, -9.0), [-0.785398, 0.785398]); // ±45°
        joint(commands, torso, arm_l, Vec2::new(-10.0, 12.0), Vec2::new(0.0, 10.0), [-1.5708, 1.5708]); // ±90°
        joint(commands, torso, arm_r, Vec2::new(10.0, 12.0), Vec2::new(0.0, 10.0), [-1.5708, 1.5708]); // ±90°
        joint(commands, torso, leg_l, Vec2::new(-7.0, -18.0), Vec2::new(0.0, 12.0), [-0.0174533, 0.0349066]); // ~locked
        joint(commands, torso, leg_r, Vec2::new(7.0, -18.0), Vec2::new(0.0, 12.0), [-0.0349066, 0.0174533]); // ~locked
    }

    let bow_pivot = commands
        .spawn((
            GameCleanup,
            Transform::from_xyz(0.0, 10.0, 2.0),
            GlobalTransform::default(),
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
        spawn_world_health_bar(commands, root);
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
            CollisionGroups::new(
                Group::GROUP_2,
                Group::GROUP_1 | Group::GROUP_3,
            ),
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
    limits: [f32; 2],
) {
    let joint = RevoluteJointBuilder::new()
        .local_anchor1(anchor_a)
        .local_anchor2(anchor_b)
        .limits(limits);
    commands.entity(b).insert(ImpulseJoint::new(a, joint));
}

fn spawn_world_health_bar(commands: &mut Commands, root: Entity) {
    let width = 42.0;

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
                y_offset: 56.0,
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
        y_offset: 56.0,
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

        let pct = (warrior.health.max(0) as f32 / warrior.max_health.max(1) as f32)
            .clamp(0.0, 1.0);
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
) {
    for (entity, warrior) in &warriors {
        if !warrior.is_dead {
            continue;
        }

        commands.entity(entity).insert(RagdollApplied);
        commands.entity(entity).remove::<ActivePuppetMotor>();
        commands.entity(warrior.torso).remove::<LockedAxes>();

        if let Ok(mut impulse) = impulses.get_mut(warrior.torso) {
            let mut rng = rand::rng();

            impulse.impulse += Vec2::new(
                rng.random_range(-90.0..90.0),
                rng.random_range(40.0..140.0),
            );

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
        }
    }
}

#[cfg(feature = "physics")]
pub fn active_puppet_motor(
    time: Res<Time>,
    warriors: Query<(&WarriorRoot, &ActivePuppetMotor), Without<RagdollApplied>>,
    global_tf: Query<&GlobalTransform>,
    mut physics: Query<(&mut ExternalImpulse, &Velocity)>,
) {
    let dt = time.delta_secs().clamp(0.001, 0.05);

    for (warrior, motor) in &warriors {
        if warrior.is_dead {
            continue;
        }

        let Ok(torso_tf) = global_tf.get(warrior.torso) else {
            continue;
        };
        let Ok((mut impulse, velocity)) = physics.get_mut(warrior.torso) else {
            continue;
        };

        let torso_pos = torso_tf.translation().truncate();
        let torso_angle = global_z_angle(torso_tf);

        let y_error = motor.stand_y - torso_pos.y;
        let y_impulse = y_error * motor.hover_strength
            - velocity.linear.y * motor.hover_damping;

        impulse.impulse += Vec2::Y * y_impulse * dt;

        let angle_error = wrap_angle(torso_angle);
        let torque = -angle_error * motor.upright_strength
            - velocity.angular * motor.upright_damping;

        impulse.torque_impulse += torque * dt;
    }
}

#[cfg(not(feature = "physics"))]
pub fn active_puppet_motor() {}

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

/// Runs once when ArchetypeVisual is added; tints limbs + scales root.
pub fn apply_archetype_visuals(
    mut commands: Commands,
    q: Query<(Entity, &ArchetypeVisual), Added<ArchetypeVisual>>,
    children: Query<&Children>,
    mut sprites: Query<&mut Sprite>,
    mut transforms: Query<&mut Transform>,
    limbs: Query<&WarriorLimb>,
) {
    for (root, vis) in &q {
        if let Ok(mut tf) = transforms.get_mut(root) {
            tf.scale = Vec3::splat(vis.scale);
        }
        tint_tree(root, vis.tint, &children, &mut sprites, &limbs);
        commands.entity(root).remove::<ArchetypeVisual>();
    }
}

fn tint_tree(
    e: Entity,
    tint: Color,
    children: &Query<&Children>,
    sprites: &mut Query<&mut Sprite>,
    limbs: &Query<&WarriorLimb>,
) {
    if limbs.get(e).is_ok() {
        if let Ok(mut s) = sprites.get_mut(e) {
            s.color = tint;
        }
    }
    if let Ok(kids) = children.get(e) {
        for c in kids.iter() {
            tint_tree(c, tint, children, sprites, limbs);
        }
    }
}
