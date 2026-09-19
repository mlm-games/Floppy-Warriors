use bevy_ecs::prelude::*;
use bevy_ecs::system::RunSystemOnce;
use glam::Vec2;
use repame_fx::{EffectDef, Gradient, Jittered};

pub fn spawn_damage_number(world: &mut World, damage: i32, pos: Vec2, headshot: bool) {
    let _ = world.run_system_once(move |mut commands: Commands| {
        repame_fx::spawn_number(
            &mut commands,
            pos.x,
            pos.y,
            format!("{damage}"),
            if headshot {
                [1.0, 0.85, 0.2, 1.0]
            } else {
                [1.0, 0.95, 0.95, 1.0]
            },
        );
    });
}

pub fn spawn_burst(world: &mut World, pos: Vec2, headshot: bool) {
    let def = EffectDef {
        speed_pps: Jittered {
            base: 105.0,
            range: 55.0,
        },
        lifetime_ticks: Jittered {
            base: 35.0,
            range: 15.0,
        },
        size_px: Jittered {
            base: 5.0,
            range: 2.0,
        },
        gravity_pps2: 300.0,
        drag_per_sec: 1.5,
        gradient: if headshot {
            Gradient::fade_out([1.0, 0.9, 0.3, 1.0])
        } else {
            Gradient::fade_out([0.95, 0.3, 0.25, 1.0])
        },
        ..Default::default()
    };
    let _ = world.run_system_once(move |mut commands: Commands| {
        let mut rng = rand::rng();
        repame_fx::burst(
            &mut commands,
            pos.x,
            pos.y,
            &def,
            if headshot { 18 } else { 8 },
            &mut rng,
        );
    });
}

pub fn step_fx(world: &mut World, dt: f32) {
    let ticks = (dt * 100.0).round() as i32;
    if let Some(mut trauma) = world.get_resource_mut::<repame_fx::Trauma>() {
        trauma.decay(ticks);
    }
    if let Some(mut flash) = world.get_resource_mut::<repame_fx::Flash>() {
        flash.tick(ticks);
    }
    let _ = world.run_system_once(
        move |mut commands: Commands, mut particles: Query<(Entity, &mut repame_fx::Particle)>| {
            repame_fx::step_particles(&mut commands, &mut particles, ticks);
        },
    );
    let _ = world.run_system_once(
        move |mut commands: Commands,
              mut numbers: Query<(Entity, &mut repame_fx::DamageNumber)>| {
            repame_fx::step_numbers(&mut commands, &mut numbers, ticks);
        },
    );
    world.resource_mut::<super::rounds::SlowMo>().tick(dt);
}
