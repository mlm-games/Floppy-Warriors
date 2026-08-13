use bevy::prelude::*;
#[cfg(feature = "physics")]
use bevy_rapier2d::prelude::*;
use game_utils_bevy::vfx::VfxSpawner;
use rand::RngExt;

use super::art::WarriorArt;
use super::components::*;
use super::warrior::{SpawnWarrior, spawn_warrior};

pub struct TitleDemoPlugin;

#[derive(Component)]
pub struct TitleDemo;

#[derive(Component)]
pub struct DemoSide {
    pub pos: Vec2,
    pub tint: Color,
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
}

#[derive(Resource, Default)]
pub struct DemoRespawn {
    pub queued: Option<RespawnJob>,
}

pub struct RespawnJob {
    pub corpse: Entity,
    pub pos: Vec2,
    pub tint: Color,
    pub other: Entity,
    pub timer: f32,
    pub health: i32,
}

const DEMO_GRAVITY: f32 = 420.0;
const DEMO_ARROW_DAMAGE: i32 = 14;
const DEMO_HP: i32 = 46;

impl Plugin for TitleDemoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DemoRespawn>()
            .add_systems(OnEnter(crate::app::AppState::Title), title_demo_setup)
            .add_systems(OnExit(crate::app::AppState::Title), title_demo_teardown)
            .add_systems(
                Update,
                (
                    demos_apply_archetype_visuals,
                    demo_ai,
                    update_demo_arrows,
                    demo_respawn,
                    demos_puppet_motor,
                    demos_tick_motor,
                    demos_ragdoll,
                )
                    .run_if(in_state(crate::app::AppState::Title)),
            );
    }
}

fn title_demo_setup(mut commands: Commands, art: Res<WarriorArt>) {
    // Landscape backdrop (from the committed bg.svg).
    commands.spawn((
        TitleDemo,
        Sprite {
            image: art.bg.clone(),
            color: Color::WHITE,
            custom_size: Some(Vec2::new(2400.0, 1370.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -50.0),
    ));
    // Ground (visual + static collider so ragdolls land on it)
    let mut ground = commands.spawn((
        TitleDemo,
        Sprite {
            color: Color::srgb(0.22, 0.32, 0.24),
            custom_size: Some(Vec2::new(2400.0, 90.0)),
            ..default()
        },
        Transform::from_xyz(0.0, -205.0, -1.0),
    ));
    #[cfg(feature = "physics")]
    ground.insert((
        RigidBody::Fixed,
        Collider::cuboid(1200.0, 45.0),
        CollisionGroups::new(Group::GROUP_3, Group::ALL),
    ));

    let a_pos = Vec2::new(-280.0, -112.0);
    let b_pos = Vec2::new(280.0, -112.0);

    let a = spawn_demo_warrior(
        &mut commands,
        &art,
        a_pos,
        Color::srgb(0.36, 0.6, 1.0),
        DEMO_HP,
    );
    let b = spawn_demo_warrior(
        &mut commands,
        &art,
        b_pos,
        Color::srgb(1.0, 0.42, 0.38),
        DEMO_HP,
    );

    let side_a = DemoSide {
        pos: a_pos,
        tint: Color::srgb(0.36, 0.6, 1.0),
        other: b,
    };
    let side_b = DemoSide {
        pos: b_pos,
        tint: Color::srgb(1.0, 0.42, 0.38),
        other: a,
    };

    commands.entity(a).insert((
        side_a,
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
    commands.entity(b).insert((
        side_b,
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

fn spawn_demo_warrior(
    commands: &mut Commands,
    textures: &WarriorArt,
    pos: Vec2,
    tint: Color,
    hp: i32,
) -> Entity {
    let root = spawn_warrior(
        commands,
        textures,
        SpawnWarrior {
            translation: pos,
            team: Team::Player,
            mods: super::components::CombatMods::default(),
            base_hp: hp,
            scale: 1.0,
        },
    );
    commands
        .entity(root)
        .insert((TitleDemo, ArchetypeVisual { tint }));
    root
}

fn title_demo_teardown(mut commands: Commands, q: Query<Entity, With<TitleDemo>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

/// Tint applied via the same logic as in-game archetypes (state-agnostic).
pub fn demos_apply_archetype_visuals(
    mut commands: Commands,
    q: Query<(Entity, &ArchetypeVisual), Added<ArchetypeVisual>>,
    children: Query<&Children>,
    mut sprites: Query<&mut Sprite>,
    skip: Query<(), With<SkipTint>>,
) {
    for (root, vis) in &q {
        super::warrior::tint_warrior_tree(root, vis.tint, &children, &mut sprites, &skip);
        commands.entity(root).remove::<ArchetypeVisual>();
    }
}

#[cfg(feature = "physics")]
pub fn demos_puppet_motor(
    time: Res<Time>,
    warriors: Query<
        (
            &WarriorRoot,
            &super::components::ActivePuppetMotor,
            Option<&super::components::HitStun>,
            Option<&super::components::Recovery>,
        ),
        Without<super::components::RagdollApplied>,
    >,
    global_tf: Query<&GlobalTransform>,
    physics: Query<(&mut ExternalImpulse, &Velocity)>,
) {
    super::warrior::active_puppet_motor(time, warriors, global_tf, physics);
}

pub fn demos_tick_motor(
    time: Res<Time>,
    q: Query<&mut super::components::HitStun>,
    rq: Query<&mut super::components::Recovery>,
) {
    super::warrior::tick_motor_state(time, q, rq);
}

#[cfg(feature = "physics")]
pub fn demos_ragdoll(
    commands: Commands,
    warriors: Query<(Entity, &WarriorRoot), Without<super::components::RagdollApplied>>,
    impulses: Query<&mut ExternalImpulse>,
) {
    super::warrior::apply_ragdoll_on_death(commands, warriors, impulses);
}

#[cfg(not(feature = "physics"))]
pub fn demos_ragdoll(
    mut commands: Commands,
    warriors: Query<(Entity, &WarriorRoot), Without<super::components::RagdollApplied>>,
) {
    super::warrior::apply_ragdoll_on_death(commands, warriors);
}

fn demo_ai(
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    sfx: Res<super::audio_fx::CombatSfx>,
    mut commands: Commands,
    mut ai_q: Query<(Entity, &mut BowState, &mut DemoAi, &DemoSide)>,
    warriors: Query<&WarriorRoot>,
    torso_tf: Query<&GlobalTransform>,
    mut bow_tf: Query<&mut Transform>,
) {
    let dt = time.delta_secs();

    for (entity, mut bow, mut ai, side) in &mut ai_q {
        let Ok(warrior) = warriors.get(entity) else {
            continue;
        };
        if warrior.is_dead {
            continue;
        }

        let Ok(origin_tf) = torso_tf.get(warrior.torso) else {
            continue;
        };
        let origin = origin_tf.translation().truncate();

        // Target the other demo warrior's torso.
        let Ok(target_warrior) = warriors.get(side.other) else {
            continue;
        };
        let Ok(target_tf) = torso_tf.get(target_warrior.torso) else {
            continue;
        };
        let target_pos = target_tf.translation().truncate();

        let aim = target_pos + ai.aim_offset;
        let angle = (aim - origin).y.atan2((aim - origin).x);

        let torso_angle = {
            let right = (origin_tf.compute_transform().rotation * Vec3::X).truncate();
            right.y.atan2(right.x)
        };
        let local_angle = angle - torso_angle;

        if let Ok(mut local_bow_tf) = bow_tf.get_mut(warrior.bow_pivot) {
            local_bow_tf.rotation = Quat::from_rotation_z(local_angle);
        }

        if ai.drawing {
            let speed = 170.0 * warrior.draw_speed_mult.max(0.1);
            bow.drawing = true;
            bow.draw_power = (bow.draw_power + speed * dt).min(100.0);
            ai.draw_timer -= dt;

            if ai.draw_timer <= 0.0 {
                let spawn_origin = origin + Vec2::from_angle(angle) * 40.0;
                let force = rand::rng().random_range(680.0..880.0);
                commands
                    .spawn((
                        TitleDemo,
                        DemoArrow {
                            shooter: entity,
                            velocity: Vec2::from_angle(angle) * force,
                        },
                        Transform::from_translation(spawn_origin.extend(5.0))
                            .with_rotation(Quat::from_rotation_z(angle)),
                    ))
                    .with_children(|b| super::arrow::arrow_visuals(b, 30.0));
                super::audio_fx::play_sfx(
                    &mut commands,
                    &asset_server,
                    &sfx.bow_release,
                    0.22,
                    0.08,
                );

                bow.drawing = false;
                bow.draw_power = 0.0;
                ai.drawing = false;
                ai.decision_timer = rand::rng().random_range(ai.decision_min..ai.decision_max);
            }
        } else {
            ai.decision_timer -= dt;
            if ai.decision_timer <= 0.0 {
                ai.aim_offset = Vec2::new(
                    rand::rng().random_range(-ai.aim_error..ai.aim_error),
                    rand::rng().random_range(-ai.aim_error..ai.aim_error),
                );
                ai.drawing = true;
                bow.drawing = true;
                bow.draw_power = 0.0;
                ai.draw_timer = rand::rng().random_range(0.5..0.9);
            }
        }
    }
}

fn update_demo_arrows(
    time: Res<Time>,
    mut commands: Commands,
    mut arrows: Query<(Entity, &mut DemoArrow, &mut Transform)>,
    limbs: Query<(Entity, &WarriorLimb, &GlobalTransform)>,
    mut warriors: Query<&mut WarriorRoot>,
    #[cfg(feature = "physics")] mut impulses: Query<&mut ExternalImpulse>,
) {
    let dt = time.delta_secs();

    for (arrow_e, mut arrow, mut tf) in &mut arrows {
        let prev = tf.translation.truncate();

        arrow.velocity.y -= DEMO_GRAVITY * dt;
        let next = prev + arrow.velocity * dt;
        tf.rotation = Quat::from_rotation_z(arrow.velocity.y.atan2(arrow.velocity.x));

        let mut hit: Option<(Entity, Entity, f32)> = None; // root, torso, t

        for (limb_e, limb, limb_tf) in &limbs {
            if limb.root == arrow.shooter {
                continue;
            }
            let Ok(w) = warriors.get(limb.root) else {
                continue;
            };
            if w.is_dead {
                continue;
            }

            let center = limb_tf.translation().truncate();
            let radius = limb.hit_radius + 4.0;
            if let Some(t) = segment_circle_t(prev, next, center, radius) {
                let replace = hit.as_ref().map(|(.., bt)| t < *bt).unwrap_or(true);
                if replace {
                    hit = Some((limb.root, limb_e, t));
                }
            }
        }

        if let Some((root, limb_e, t)) = hit {
            let pos = prev + (next - prev) * t;
            {
                let mut w = warriors.get_mut(root).unwrap();
                w.health = (w.health - DEMO_ARROW_DAMAGE).max(0);
                if w.health <= 0 {
                    w.is_dead = true;
                }
            }

            #[cfg(feature = "physics")]
            {
                let knock = arrow.velocity.normalize_or_zero();
                if let Ok(mut torso_imp) =
                    impulses.get_mut(warrior_torso(root, &warriors).unwrap_or(limb_e))
                {
                    torso_imp.impulse += knock * 22.0;
                }
                if let Ok(mut limb_imp) = impulses.get_mut(limb_e) {
                    limb_imp.impulse += knock * 10.0;
                }
            }

            VfxSpawner::spawn_burst(
                &mut commands,
                pos,
                7,
                Color::srgb(0.95, 0.85, 0.3),
                (40.0, 110.0),
            );
            commands.entity(arrow_e).despawn();
            continue;
        }

        // Ground / off-screen despawn.
        if next.y < -196.0 || next.x.abs() > 1000.0 || next.y > 300.0 {
            commands.entity(arrow_e).despawn();
            continue;
        }

        tf.translation = next.extend(5.0);
    }
}

fn warrior_torso(root: Entity, warriors: &Query<&mut WarriorRoot>) -> Option<Entity> {
    warriors.get(root).ok().map(|w| w.torso)
}

fn demo_respawn(
    time: Res<Time>,
    mut commands: Commands,
    textures: Res<WarriorArt>,
    mut res: ResMut<DemoRespawn>,
    mut q: Query<(Entity, &WarriorRoot, &DemoSide, &DemoAi)>,
) {
    // If nothing is queued and a demo warrior died, queue its respawn.
    if res.queued.is_none() {
        for (e, w, side, _ai) in &mut q {
            if w.is_dead {
                res.queued = Some(RespawnJob {
                    corpse: e,
                    pos: side.pos,
                    tint: side.tint,
                    other: side.other,
                    timer: 2.6,
                    health: DEMO_HP,
                });
                break;
            }
        }
        return;
    }

    let job = res.queued.as_mut().unwrap();
    job.timer -= time.delta_secs();
    if job.timer > 0.0 {
        return;
    }

    let job = res.queued.take().unwrap();
    // Remove the old corpse (and any arrows stuck to its limbs via ChildOf).
    commands
        .entity(job.corpse)
        .insert(super::components::RagdollApplied);
    let new_root = spawn_demo_warrior(&mut commands, &textures, job.pos, job.tint, job.health);
    commands.entity(new_root).insert((
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
    commands.entity(job.corpse).despawn();
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
