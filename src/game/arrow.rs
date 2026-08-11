use bevy::prelude::*;
#[cfg(feature = "physics")]
use bevy_rapier2d::prelude::*;
use game_utils_bevy::screen_effects::{ScreenEffects, Trauma};
use game_utils_bevy::vfx::VfxSpawner;
use super::components::*;
use super::round_manager::HitConfirmed;

pub fn spawn_arrow(
    commands: &mut Commands,
    origin: Vec2,
    velocity: Vec2,
    shooter: Entity,
    team: Team,
    damage: f32,
    damage_mult: f32,
    headshot_mult: f32,
    knockback_mult: f32,
) {
    let angle = velocity.y.atan2(velocity.x);
    let mut e = commands.spawn((
        GameCleanup,
        Arrow {
            shooter,
            team,
            damage,
            damage_mult,
            headshot_mult,
            knockback_mult,
            has_hit: false,
            stuck_life: 2.0,
        },
        Sprite {
            color: Color::srgb(0.47, 0.34, 0.22),
            custom_size: Some(Vec2::new(30.0, 6.0)),
            ..default()
        },
        Transform::from_translation(origin.extend(5.0))
            .with_rotation(Quat::from_rotation_z(angle)),
    ));
    #[cfg(feature = "physics")]
    {
        e.insert((
            RigidBody::Dynamic,
            Collider::cuboid(15.0, 3.0),
            AdditionalMassProperties::Mass(0.5),
            Velocity::linear(velocity),
            GravityScale(0.2),
            Ccd::enabled(),
            CollisionGroups::new(
                Group::GROUP_4,
                Group::GROUP_1 | Group::GROUP_2 | Group::GROUP_3,
            ),
            // Sensor-ish continuous hits via collision events:
            CollidingEntities::default(),
        ));
    }
    #[cfg(not(feature = "physics"))]
    {
        e.insert(LinearVelFallback(velocity));
    }
}

#[cfg(not(feature = "physics"))]
#[derive(Component)]
pub struct LinearVelFallback(pub Vec2);

#[cfg(not(feature = "physics"))]
pub fn move_arrows_fallback(
    time: Res<Time>,
    mut q: Query<(&mut Transform, &mut LinearVelFallback, &Arrow)>,
) {
    for (mut tf, mut v, a) in &mut q {
        if a.has_hit {
            continue;
        }
        v.0.y -= 200.0 * time.delta_secs(); // light gravity
        tf.translation += (v.0 * time.delta_secs()).extend(0.0);
        tf.rotation = Quat::from_rotation_z(v.0.y.atan2(v.0.x));
    }
}

#[cfg(feature = "physics")]
pub fn arrow_align_velocity(
    mut q: Query<(&mut Transform, &Velocity, &Arrow), With<Arrow>>,
) {
    for (mut tf, vel, a) in &mut q {
        if a.has_hit || vel.linear.length_squared() < 1.0 {
            continue;
        }
        let a = vel.linear.y.atan2(vel.linear.x);
        tf.rotation = Quat::from_rotation_z(a);
    }
}

/// Collision resolution — avian CollidingEntities.
pub fn resolve_arrow_hits(
    mut commands: Commands,
    mut trauma: ResMut<Trauma>,
    mut hits: MessageWriter<HitConfirmed>,
    mut arrows: Query<(Entity, &mut Arrow, &Transform, Option<&CollidingEntities>)>,
    limbs: Query<(Entity, &WarriorLimb, &Transform)>,
    mut warriors: Query<&mut WarriorRoot>,
    ground: Query<Entity, With<Ground>>,
    #[cfg(feature = "physics")] mut impulses: Query<&mut ExternalImpulse>,
) {
    for (ae, mut arrow, at, colliding) in &mut arrows {
        if arrow.has_hit {
            continue;
        }
        let mut hit_limb: Option<(Entity, LimbKind, Entity)> = None;
        #[cfg(feature = "physics")]
        {
            if let Some(col) = colliding {
                for other in col.iter() {
                    if ground.get(other).is_ok() {
                        arrow.has_hit = true;
                        freeze_arrow(&mut commands, ae);
                        break;
                    }
                    if let Ok((_, limb, _)) = limbs.get(other) {
                        if let Ok(w) = warriors.get(limb.root) {
                            if w.is_dead || limb.root == arrow.shooter || w.team == arrow.team {
                                continue;
                            }
                            hit_limb = Some((other, limb.kind, limb.root));
                            break;
                        }
                    }
                }
            }
        }
        #[cfg(not(feature = "physics"))]
        {
            let ap = at.translation.truncate();
            for (le, limb, lt) in &limbs {
                if let Ok(w) = warriors.get(limb.root) {
                    if w.is_dead || limb.root == arrow.shooter || w.team == arrow.team {
                        continue;
                    }
                    if ap.distance(lt.translation.truncate()) < 18.0 {
                        hit_limb = Some((le, limb.kind, limb.root));
                        break;
                    }
                }
            }
        }
        let Some((limb_e, kind, root)) = hit_limb else {
            continue;
        };
        arrow.has_hit = true;
        let is_head = kind == LimbKind::Head;
        let mult = kind.damage_mult() * if is_head { arrow.headshot_mult } else { 1.0 };
        let dmg = (arrow.damage * arrow.damage_mult * mult).round().max(1.0) as i32;
        let mut killed = false;
        if let Ok(mut w) = warriors.get_mut(root) {
            if !w.is_dead {
                w.health = (w.health - dmg).max(0);
                killed = w.health <= 0;
                if killed {
                    w.is_dead = true;
                }
            }
        }

        // Knockback
        #[cfg(feature = "physics")]
        {
            let dir = at.rotation * Vec3::X;
            if let Ok(mut imp) = impulses.get_mut(limb_e) {
                imp.impulse +=
                    dir.truncate().normalize_or_zero() * dmg as f32 * 7.0 * arrow.knockback_mult;
            }
        }

        let pos = at.translation.truncate();
        VfxSpawner::spawn_damage_number(
            &mut commands,
            dmg,
            pos,
            if is_head {
                Color::srgb(1.0, 0.85, 0.2)
            } else {
                Color::srgb(1.0, 0.95, 0.95)
            },
        );
        VfxSpawner::spawn_burst(
            &mut commands,
            pos,
            if is_head { 18 } else { 8 },
            if is_head {
                Color::srgb(1.0, 0.9, 0.3)
            } else {
                Color::srgb(0.95, 0.3, 0.25)
            },
            (50.0, 160.0),
        );
        ScreenEffects::add_trauma(&mut trauma, if is_head { 0.45 } else if killed { 0.35 } else { 0.18 });
        hits.write(HitConfirmed {
            shooter: arrow.shooter,
            target: root,
            limb: kind,
            damage: dmg,
            killed,
            headshot: is_head,
        });
        freeze_arrow(&mut commands, ae);
    }
}

fn freeze_arrow(commands: &mut Commands, e: Entity) {
    #[cfg(feature = "physics")]
    {
        commands.entity(e).insert(RigidBody::Fixed);
        commands.entity(e).remove::<Velocity>();
    }
}

pub fn despawn_stuck_arrows(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Arrow)>,
) {
    for (e, mut a) in &mut q {
        if !a.has_hit {
            continue;
        }
        a.stuck_life -= time.delta_secs();
        if a.stuck_life <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}