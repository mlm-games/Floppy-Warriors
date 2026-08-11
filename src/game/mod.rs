mod arena;
mod arrow;
mod components;
mod enemy_ai;
mod hud_sync;
pub mod meta;
mod player;
mod round_manager;
mod warrior;

use bevy::prelude::*;
use crate::app::{AppState, Paused};
use game_utils_bevy::transitions::Transition;

pub use components::*;
pub use round_manager::{ChooseReward, HitConfirmed, RoundManager, RunPhase};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoundManager>()
            .init_resource::<RestartFlag>()
            .add_message::<HitConfirmed>()
            .add_message::<ChooseReward>()
            .add_systems(
                OnEnter(AppState::InGame),
                (arena::spawn_arena, round_manager::begin_run).chain(),
            )
            .add_systems(OnExit(AppState::InGame), cleanup_game)
            .add_systems(
                Update,
                (
                    warrior::active_puppet_motor,
                    player::player_aim_and_bow,
                    enemy_ai::enemy_ai_system,
                    arrow::update_arrows,
                    warrior::apply_ragdoll_on_death,
                    round_manager::spawn_enemies_system,
                    round_manager::on_hits,
                    round_manager::apply_reward_system,
                    round_manager::finalize_game_over_bones,
                    hud_sync::sync_run_to_ui,
                    (warrior::sync_world_health_bars, warrior::sync_health_fills),
                    handle_restart_input,
                    process_restart,
                    death_fade,
                )
                    .run_if(in_state(AppState::InGame))
                    .run_if(|p: Res<Paused>| !p.0)
                    .run_if(|t: Res<Transition<AppState>>| !t.block_input),
            );
    }
}

#[derive(Resource, Default)]
struct RestartFlag(bool);

fn cleanup_game(mut commands: Commands, q: Query<Entity, With<GameCleanup>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn handle_restart_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    rm: Res<RoundManager>,
    mut flag: ResMut<RestartFlag>,
) {
    if rm.phase != RunPhase::GameOver {
        return;
    }
    if keys.just_pressed(KeyCode::KeyR) || mouse.just_pressed(MouseButton::Left) {
        flag.0 = true;
    }
}

fn process_restart(
    mut flag: ResMut<RestartFlag>,
    mut commands: Commands,
    cleanup: Query<Entity, With<GameCleanup>>,
    rm: ResMut<RoundManager>,
    save: Res<crate::save::SaveData>,
) {
    if !flag.0 {
        return;
    }

    flag.0 = false;

    for e in &cleanup {
        commands.entity(e).despawn();
    }

    arena::spawn_arena(commands.reborrow());
    round_manager::begin_run(rm, commands, save);
}

fn death_fade(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut round_manager::DyingFade)>,
    children_q: Query<&Children>,
    mut sprites: Query<&mut Sprite>,
) {
    for (entity, mut fade) in &mut q {
        fade.0 -= time.delta_secs();
        let alpha = (fade.0 / 2.0).clamp(0.0, 1.0);

        fade_recursive(entity, alpha, &children_q, &mut sprites);

        if fade.0 <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn fade_recursive(
    entity: Entity,
    alpha: f32,
    children_q: &Query<&Children>,
    sprites: &mut Query<&mut Sprite>,
) {
    if let Ok(mut sprite) = sprites.get_mut(entity) {
        sprite.color.set_alpha(alpha);
    }

    if let Ok(children) = children_q.get(entity) {
        for child in children.iter() {
            fade_recursive(child, alpha, children_q, sprites);
        }
    }
}
