use bevy::prelude::*;
#[cfg(feature = "physics")]
use bevy_rapier2d::prelude::*;

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
fn joint(commands: &mut Commands, a: Entity, b: Entity, anchor_a: Vec2, anchor_b: Vec2) {
    let joint = RevoluteJointBuilder::new()
        .local_anchor1(anchor_a)
        .local_anchor2(anchor_b);
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

    let right = (bow_tf.compute_transform().rotation * Vec3::X)
        .truncate()
        .normalize_or_zero();

    if right.length_squared() <= 0.0001 {
        return;
    }

    let base_angle = right.y.atan2(right.x);
    let origin = bow_tf.translation().truncate() + right * 24.0;
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
            warrior.damage_mult,
            warrior.headshot_mult,
            warrior.knockback_mult,
        );
    }
}
