use bevy_ecs::prelude::*;
use glam::Vec2;
use repame_rapier2d::BodySnapshot2d;

use super::comps::*;
use super::rounds::RunPhase;
use super::rounds::RunState;

#[derive(Resource, Clone, Copy, Debug)]
pub struct SimStep {
    /// Gameplay delta: fixed step scaled by slow motion.
    pub dt: f32,
    /// Unscaled fixed step, for effect recovery that must not stretch.
    pub raw: f32,
}

impl Default for SimStep {
    fn default() -> Self {
        Self {
            dt: 1.0 / 60.0,
            raw: 1.0 / 60.0,
        }
    }
}

#[derive(Component)]
pub struct DyingFade(pub f32);

#[derive(Resource, Default)]
pub struct GameOverFlushed(pub bool);

pub fn build_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.add_systems(
        (
            super::player::player_system,
            super::enemy_ai::enemy_ai_system,
            super::warrior::active_puppet_motor,
            super::warrior::tick_motor_state,
            super::arrow::update_arrows,
            super::warrior::apply_ragdoll_on_death,
            super::rounds::spawn_enemies_system,
            super::rounds::on_hits,
            super::rounds::apply_reward_system,
            super::rounds::finalize_game_over_bones,
            tick_slowmo,
            tick_dying_fade,
            super::arena::cleanup_far_entities,
            validate_round_manager,
            validate_warriors,
        )
            .chain(),
    );
    schedule
}

fn tick_slowmo(world: &mut World) {
    let dt = world.resource::<SimStep>().raw;
    if world.get_resource::<super::rounds::SlowMo>().is_none() {
        return;
    }
    world.resource_mut::<super::rounds::SlowMo>().tick(dt);
}

fn tick_dying_fade(world: &mut World) {
    let dt = world.resource::<SimStep>().dt;
    let fading: Vec<(Entity, f32)> = world
        .query::<(Entity, &DyingFade)>()
        .iter(world)
        .map(|(entity, fade)| (entity, fade.0))
        .collect();
    for (entity, remaining) in fading {
        let next = remaining - dt;
        if next <= 0.0 {
            super::warrior::despawn_warrior(world, entity);
        } else if let Some(mut fade) = world.get_mut::<DyingFade>(entity) {
            fade.0 = next;
        }
    }
}

fn validate_round_manager(rm: Res<RunState>, warriors: Query<&WarriorRoot>) {
    if !rm.is_changed() {
        return;
    }
    debug_assert!(
        rm.enemies_remaining <= rm.enemies_total,
        "enemies_remaining {} > enemies_total {}",
        rm.enemies_remaining,
        rm.enemies_total
    );
    debug_assert!(
        !(rm.phase == RunPhase::Reward && rm.reward_choices.is_empty()),
        "Entered reward phase with no reward choices"
    );
    if let Some(player) = rm.player {
        debug_assert!(
            warriors.get(player).is_ok(),
            "RunState.player points to missing entity"
        );
    }
    if let Some(enemy) = rm.active_enemy {
        debug_assert!(
            warriors.get(enemy).is_ok(),
            "RunState.active_enemy points to missing entity"
        );
    }
    if rm.phase == RunPhase::GameOver {
        debug_assert!(
            rm.player.is_some(),
            "GameOver reached with no player tracked"
        );
    }
}

fn validate_warriors(q: Query<(Entity, &WarriorRoot)>) {
    for (e, w) in &q {
        debug_assert!(w.max_health >= 1, "Warrior {e:?} max_health < 1");
        debug_assert!(
            w.health <= w.max_health,
            "Warrior {e:?} health > max_health"
        );
        debug_assert!(w.arrow_count >= 1, "Warrior {e:?} arrow_count < 1");
        debug_assert!(w.crit_chance <= 1.0, "Warrior {e:?} crit chance > 1.0");
        debug_assert!(
            w.damage_taken_mult > 0.0,
            "Warrior {e:?} damage_taken_mult <= 0"
        );
    }
}

pub fn diag_anomaly_lines(world: &mut World) -> Vec<String> {
    let mut out = Vec::new();
    let roots: Vec<(Entity, bool, f32, f32, bool)> = world
        .query::<(
            Entity,
            &WarriorRoot,
            Option<&HitStun>,
            Option<&Recovery>,
            Has<RagdollApplied>,
        )>()
        .iter(world)
        .map(|(e, w, stun, rec, rag)| {
            (
                e,
                w.is_dead,
                stun.map_or(0.0, |s| s.remaining),
                rec.map_or(0.0, |r| r.remaining),
                rag,
            )
        })
        .collect();
    let limbs: Vec<(Entity, LimbKind, Vec2, Vec2, f32)> = world
        .query::<(Entity, &WarriorLimb, &BodySnapshot2d)>()
        .iter(world)
        .map(|(e, l, s)| (e, l.kind, s.pos, s.linvel, s.angvel))
        .collect();
    for (limb_e, kind, pos, linvel, angvel) in limbs {
        let sp = linvel.length();
        if sp > 600.0 || angvel.abs() > 8.0 {
            let root = world.get::<WarriorLimb>(limb_e).map(|l| l.root);
            let (is_dead, stun, rec, rag) = roots
                .iter()
                .find(|(e, ..)| Some(*e) == root)
                .map(|(_, d, s, r, g)| (*d, *s, *r, *g))
                .unwrap_or((false, 0.0, 0.0, false));
            out.push(format!(
                "root={root:?} limb={kind:?} speed={sp:.0} angvel={angvel:.1} pos=({:.0},{:.0}) dead={is_dead} stun={stun:.1} rec={rec:.1} rag={rag}",
                pos.x, pos.y
            ));
        }
    }
    out
}
