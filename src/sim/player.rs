use bevy_ecs::prelude::*;
use glam::Vec2;
use repame_rapier2d::ImpulseRequest;

use super::comps::*;
use super::rounds::{RunPhase, RunState};
use super::state::SimStep;

pub fn player_system(world: &mut World) {
    let dt: f32;
    let combat: bool;
    {
        let step = match world.get_resource::<SimStep>() {
            Some(v) => v,
            None => return,
        };
        dt = step.dt;
        combat = world.resource::<RunState>().phase == RunPhase::Combat;
    }
    if !combat {
        return;
    }
    let Some(player_e) = world.resource::<RunState>().player else {
        return;
    };
    let input = world.resource::<super::input::PlayerInput>().clone();
    let Some(warrior) = world
        .get::<WarriorRoot>(player_e)
        .map(|w| (w.is_dead, w.torso, w.draw_speed_mult))
    else {
        return;
    };
    if warrior.0 {
        return;
    }
    let origin = world
        .get::<repame_rapier2d::BodySnapshot2d>(warrior.1)
        .map(|s| s.pos)
        .unwrap_or(Vec2::ZERO);
    let torso_angle = world
        .get::<repame_rapier2d::BodySnapshot2d>(warrior.1)
        .map(|s| s.angle)
        .unwrap_or(0.0);
    let target = if input.stick.length_squared() > 0.04 {
        origin + input.stick.normalize() * 120.0
    } else {
        input.aim_world.unwrap_or(origin + Vec2::X * 100.0)
    };
    let angle = (target - origin).y.atan2((target - origin).x);
    let local_angle = angle - torso_angle;
    if let Some(mut aim) = world.get_mut::<BowAim>(player_e) {
        aim.angle = local_angle;
    }
    if input.draw_pressed {
        if let Some(mut bow) = world.get_mut::<BowState>(player_e) {
            if !bow.drawing {
                bow.drawing = true;
                bow.draw_power = 0.0;
                super::audio::push_cue(
                    world,
                    super::audio::AudioCue {
                        name: "bow_draw",
                        volume: 0.3,
                        variance: 0.05,
                    },
                );
            }
        }
    }
    {
        let draw_speed = warrior.2;
        if let Some(mut bow) = world.get_mut::<BowState>(player_e) {
            if bow.drawing {
                let speed = 160.0 * draw_speed.max(0.1);
                bow.draw_power = (bow.draw_power + speed * dt).min(100.0);
            }
        }
    }
    if input.draw_released {
        let drawing = world
            .get::<BowState>(player_e)
            .map(|b| b.drawing)
            .unwrap_or(false);
        if drawing {
            let spawn_origin = origin + Vec2::from_angle(angle) * 36.0;
            let mut cues = Vec::new();
            super::arrow::fire_from_bow(world, player_e, spawn_origin, angle, &mut cues);
            {
                let q = world.resource::<super::audio::CueQueue>().clone();
                q.0.lock().unwrap().extend(cues);
            }
            if let Some(mut bow) = world.get_mut::<BowState>(player_e) {
                bow.drawing = false;
                bow.draw_power = 0.0;
            }
        }
    } else if input.lmb_held {
        if let Some(bow) = world.get::<BowState>(player_e) {
            if !bow.drawing {
                if let Some(mut bow) = world.get_mut::<BowState>(player_e) {
                    bow.drawing = true;
                    bow.draw_power = 0.0;
                }
                super::audio::push_cue(
                    world,
                    super::audio::AudioCue {
                        name: "bow_draw",
                        volume: 0.3,
                        variance: 0.05,
                    },
                );
            }
        }
    }
    let airdodge = world
        .get::<Airdodge>(player_e)
        .map(|a| (a.impulse, a.cooldown, a.remaining));
    let Some((impulse, cooldown, remaining)) = airdodge else {
        return;
    };
    {
        let remaining = (remaining - dt).max(0.0);
        if let Some(mut a) = world.get_mut::<Airdodge>(player_e) {
            a.remaining = remaining;
        }
        if input.dodge_pressed && remaining <= 0.0 {
            if let Some(mut a) = world.get_mut::<Airdodge>(player_e) {
                a.remaining = cooldown;
            }
            world
                .entity_mut(player_e)
                .insert(Recovery { remaining: 0.35 });
            if let Some(mut impulse_req) = world.get_mut::<ImpulseRequest>(warrior.1) {
                impulse_req.linear += Vec2::Y * impulse;
            }
        }
    }
}

#[cfg(test)]
mod input_tests {
    use super::*;
    use crate::sim::input::PlayerInput;
    use crate::sim::rounds::RunState;

    fn combat_world() -> World {
        let mut world = World::new();
        world.insert_resource(SimStep { dt: 1.0 / 60.0 });
        world.insert_resource(RunState {
            phase: RunPhase::Combat,
            ..Default::default()
        });
        world.insert_resource(PlayerInput::default());
        world.insert_resource(crate::sim::audio::CueQueue::default());
        world.insert_resource(crate::sim::state::GameOverFlushed::default());
        world
    }

    fn spawn_player(world: &mut World) -> Entity {
        let torso = world
            .spawn((
                repame_rapier2d::BodySnapshot2d {
                    pos: Vec2::ZERO,
                    angle: 0.0,
                    ..Default::default()
                },
                ImpulseRequest::default(),
            ))
            .id();
        let root = world
            .spawn((
                WarriorRoot {
                    team: Team::Player,
                    health: 100,
                    max_health: 100,
                    is_dead: false,
                    torso,
                    draw_speed_mult: 1.0,
                    ..warrior_test_defaults()
                },
                BowState {
                    drawing: false,
                    draw_power: 0.0,
                },
                BowAim { angle: 0.0 },
                Airdodge {
                    impulse: 420.0,
                    cooldown: 1.0,
                    remaining: 0.0,
                },
            ))
            .id();
        world.resource_mut::<RunState>().player = Some(root);
        root
    }

    fn warrior_test_defaults() -> WarriorRoot {
        WarriorRoot {
            team: Team::Player,
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
    fn draw_press_starts_bow_once() {
        let mut world = combat_world();
        let player = spawn_player(&mut world);
        world.resource_mut::<PlayerInput>().draw_pressed = true;
        player_system(&mut world);
        world.resource_mut::<PlayerInput>().draw_pressed = true;
        player_system(&mut world);
        let bow = world.get::<BowState>(player).unwrap();
        assert!(bow.drawing);
        let cues = crate::sim::audio::drain_cues(&mut world);
        assert_eq!(cues.iter().filter(|c| c.name == "bow_draw").count(), 1);
    }

    #[test]
    fn lmb_hold_sustains_draw_release_fires() {
        let mut world = combat_world();
        let player = spawn_player(&mut world);
        world.resource_mut::<PlayerInput>().lmb_held = true;
        player_system(&mut world);
        assert!(world.get::<BowState>(player).unwrap().drawing);
        world.resource_mut::<PlayerInput>().lmb_held = false;
        world.resource_mut::<PlayerInput>().draw_released = true;
        player_system(&mut world);
        assert!(!world.get::<BowState>(player).unwrap().drawing);
        assert!(
            world
                .query::<&crate::sim::comps::Arrow>()
                .iter(&world)
                .next()
                .is_some()
        );
    }

    #[test]
    fn release_without_draw_spawns_nothing() {
        let mut world = combat_world();
        spawn_player(&mut world);
        world.resource_mut::<PlayerInput>().draw_released = true;
        player_system(&mut world);
        assert!(
            world
                .query::<&crate::sim::comps::Arrow>()
                .iter(&world)
                .next()
                .is_none()
        );
    }
}
