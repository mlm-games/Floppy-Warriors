use bevy::prelude::*;
#[cfg(feature = "physics")]
use bevy_rapier2d::prelude::ExternalImpulse;
use game_utils_bevy::game_feel::{GameFeel, SlowMotion};
use game_utils_bevy::screen_effects::{ScreenEffects, Trauma};
use game_utils_bevy::vfx::VfxSpawner;
use rand::RngExt;

use super::audio_fx::{self, CombatSfx};
use super::components::*;
use super::round_manager::HitConfirmed;

const ARROW_GRAVITY: f32 = 420.0;
const STUCK_LIFETIME: f32 = 2.0;
const HITSTUN_SECS: f32 = 0.22;

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
    crit_chance: f32,
    crit_mult: f32,
) {
    let angle = velocity.y.atan2(velocity.x);

    commands.spawn((
        GameCleanup,
        Arrow {
            shooter,
            team,
            damage,
            damage_mult,
            headshot_mult,
            knockback_mult,
            crit_chance,
            crit_mult,
            velocity,
            has_hit: false,
            stuck_life: STUCK_LIFETIME,
            stuck_to: None,
            stuck_local_offset: Vec2::ZERO,
            stuck_angle_offset: 0.0,
        },
        Sprite {
            color: Color::srgb(0.47, 0.34, 0.22),
            custom_size: Some(Vec2::new(30.0, 6.0)),
            ..default()
        },
        Transform::from_translation(origin.extend(5.0))
            .with_rotation(Quat::from_rotation_z(angle)),
    ));
}

pub fn update_arrows(
    time: Res<Time>,
    mut commands: Commands,
    mut trauma: ResMut<Trauma>,
    asset_server: Res<AssetServer>,
    sfx: Res<CombatSfx>,
    mut slow_mo: ResMut<SlowMotion>,
    mut hits: MessageWriter<HitConfirmed>,
    mut arrows: Query<(Entity, &mut Arrow, &mut Transform)>,
    limbs: Query<(Entity, &WarriorLimb, &GlobalTransform)>,
    mut warriors: ParamSet<(Query<&WarriorRoot>, Query<&mut WarriorRoot>)>,
    ground: Query<(&GlobalTransform, &Sprite), With<Ground>>,
    player_tags: Query<(), With<PlayerTag>>,
    #[cfg(feature = "physics")] mut impulses: Query<&mut ExternalImpulse>,
) {
    let dt = time.delta_secs();

    for (arrow_e, mut arrow, mut tf) in &mut arrows {
        if arrow.has_hit {
            if let Some(parent) = arrow.stuck_to
                && let Ok((_, _, parent_tf)) = limbs.get(parent)
            {
                let parent_angle = global_z_angle(parent_tf);
                let parent_pos = parent_tf.translation().truncate();
                let world_offset = rotate_vec(arrow.stuck_local_offset, parent_angle);

                tf.translation = (parent_pos + world_offset).extend(tf.translation.z);
                tf.rotation = Quat::from_rotation_z(parent_angle + arrow.stuck_angle_offset);
            }

            arrow.stuck_life -= dt;
            if arrow.stuck_life <= 0.0 {
                commands.entity(arrow_e).despawn();
            }
            continue;
        }

        let prev = tf.translation.truncate();

        arrow.velocity.y -= ARROW_GRAVITY * dt;
        let next = prev + arrow.velocity * dt;
        let arrow_angle = arrow.velocity.y.atan2(arrow.velocity.x);
        tf.rotation = Quat::from_rotation_z(arrow_angle);

        let best_limb_hit = {
            let wq = warriors.p0();
            let mut best_limb_hit: Option<(f32, Entity, Entity, Entity, LimbKind, Vec2, f32, Vec2)> =
                None;

            for (limb_e, limb, limb_tf) in &limbs {
                let Ok(warrior) = wq.get(limb.root) else {
                    continue;
                };

                if warrior.is_dead || limb.root == arrow.shooter || warrior.team == arrow.team {
                    continue;
                }

                let center = limb_tf.translation().truncate();
                let radius = limb.hit_radius + 4.0;

                if let Some((t, hit_point)) = segment_circle_hit(prev, next, center, radius) {
                    let limb_angle = global_z_angle(limb_tf);
                    let replace = best_limb_hit
                        .as_ref()
                        .map(|(best_t, ..)| t < *best_t)
                        .unwrap_or(true);

                    if replace {
                        best_limb_hit = Some((
                            t,
                            limb_e,
                            limb.root,
                            warrior.torso,
                            limb.kind,
                            hit_point,
                            limb_angle,
                            center,
                        ));
                    }
                }
            }

            best_limb_hit
        };

        let mut best_ground_hit: Option<(f32, Vec2)> = None;
        if let Ok((ground_tf, ground_sprite)) = ground.single()
            && let Some(size) = ground_sprite.custom_size
        {
            let center = ground_tf.translation().truncate();
            best_ground_hit = segment_top_rect_hit(prev, next, center, size);
        }

        let limb_t = best_limb_hit.as_ref().map(|v| v.0);
        let ground_t = best_ground_hit.as_ref().map(|v| v.0);

        let limb_wins = match (limb_t, ground_t) {
            (Some(lt), Some(gt)) => lt <= gt,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => false,
        };

        if limb_wins {
            let Some((_, limb_e, root_e, _torso_e, kind, hit_point, limb_angle, limb_center)) =
                best_limb_hit
            else {
                continue;
            };

            let mult = kind.damage_mult()
                * if kind == LimbKind::Head {
                    arrow.headshot_mult
                } else {
                    1.0
                };

            let headshot = kind == LimbKind::Head;

            let target_damage_taken_mult = {
                warriors
                    .p0()
                    .get(root_e)
                    .map(|w| w.damage_taken_mult)
                    .unwrap_or(1.0)
            };

            let crit = rand::rng().random::<f32>() < arrow.crit_chance;

            let mut damage_f =
                arrow.damage * arrow.damage_mult * mult * target_damage_taken_mult;

            if crit {
                damage_f *= arrow.crit_mult;
            }

            let damage = damage_f.round().max(1.0) as i32;

            let mut killed = false;
            if let Ok(mut warrior) = warriors.p1().get_mut(root_e)
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

                        ScreenEffects::add_trauma(&mut trauma, 0.65);
                    } else {
                        warrior.is_dead = true;
                    }
                }
            }

            #[cfg(feature = "physics")]
            {
                let knock = arrow.velocity.normalize_or_zero();
                // Whole-body impulse on the torso (primary knockback driver).
                if let Ok(mut impulse) = impulses.get_mut(_torso_e) {
                    impulse.impulse += knock * damage as f32 * 8.0 * arrow.knockback_mult;
                }
                // Smaller local impulse on the hit limb so the limb lags the body.
                if let Ok(mut impulse) = impulses.get_mut(limb_e) {
                    impulse.impulse += knock * damage as f32 * 3.5 * arrow.knockback_mult;
                }
            }

            // Suppress the puppet motor briefly so the knockback reads instead
            // of being instantly corrected by hover/upright control.
            commands.entity(root_e).insert(HitStun {
                remaining: HITSTUN_SECS,
            });

            VfxSpawner::spawn_damage_number(
                &mut commands,
                damage,
                hit_point,
                if headshot {
                    Color::srgb(1.0, 0.85, 0.2)
                } else {
                    Color::srgb(1.0, 0.95, 0.95)
                },
            );
            VfxSpawner::spawn_burst(
                &mut commands,
                hit_point,
                if headshot { 18 } else { 8 },
                if headshot {
                    Color::srgb(1.0, 0.9, 0.3)
                } else {
                    Color::srgb(0.95, 0.3, 0.25)
                },
                (50.0, 160.0),
            );
            let hit_player = player_tags.contains(root_e);

            if !hit_player {
                if headshot && killed {
                    ScreenEffects::add_trauma(&mut trauma, 0.7);
                    GameFeel::slow_motion(&mut slow_mo, 0.25, 0.22);
                } else if killed {
                    ScreenEffects::add_trauma(&mut trauma, 0.45);
                    GameFeel::slow_motion(&mut slow_mo, 0.4, 0.08);
                } else if headshot {
                    ScreenEffects::add_trauma(&mut trauma, 0.35);
                    GameFeel::slow_motion(&mut slow_mo, 0.05, 0.06);
                } else {
                    ScreenEffects::add_trauma(&mut trauma, 0.15);
                    GameFeel::slow_motion(&mut slow_mo, 0.05, 0.05);
                }
            } else {
                ScreenEffects::add_trauma(&mut trauma, if headshot { 0.35 } else { 0.18 });
            }

            if headshot {
                audio_fx::play_sfx(&mut commands, &asset_server, &sfx.headshot, 0.6, 0.08);
            } else {
                audio_fx::play_sfx(&mut commands, &asset_server, &sfx.hit, 0.5, 0.1);
            }
            if killed {
                audio_fx::play_sfx(&mut commands, &asset_server, &sfx.kill, 0.55, 0.05);
            }

            hits.write(HitConfirmed {
                shooter: arrow.shooter,
                target: root_e,
                damage,
                killed,
                headshot,
            });

            arrow.has_hit = true;
            arrow.velocity = Vec2::ZERO;
            arrow.stuck_life = STUCK_LIFETIME;
            arrow.stuck_to = Some(limb_e);
            arrow.stuck_local_offset = rotate_vec(hit_point - limb_center, -limb_angle);
            arrow.stuck_angle_offset = arrow_angle - limb_angle;

            tf.translation = hit_point.extend(tf.translation.z);
            tf.rotation = Quat::from_rotation_z(arrow_angle);
            continue;
        }

        if let Some((_, hit_point)) = best_ground_hit {
            arrow.has_hit = true;
            arrow.velocity = Vec2::ZERO;
            arrow.stuck_life = STUCK_LIFETIME;
            arrow.stuck_to = None;

            tf.translation = hit_point.extend(tf.translation.z);
            tf.rotation = Quat::from_rotation_z(arrow_angle);
            continue;
        }

        tf.translation = next.extend(tf.translation.z);
    }
}

fn segment_circle_hit(a: Vec2, b: Vec2, center: Vec2, radius: f32) -> Option<(f32, Vec2)> {
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
    let top = center.y + half.y;

    if a.y >= top && b.y <= top {
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

fn global_z_angle(tf: &GlobalTransform) -> f32 {
    let right = (tf.compute_transform().rotation * Vec3::X).truncate();
    right.y.atan2(right.x)
}

fn rotate_vec(v: Vec2, angle: f32) -> Vec2 {
    let (s, c) = angle.sin_cos();
    Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}
