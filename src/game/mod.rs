mod arena;
mod arrow;
mod art;
mod audio_fx;
mod cleanup_bounds;
mod components;
mod debug_invariants;
mod enemy_ai;
mod hud_sync;
pub mod meta;
mod player;
mod round_manager;
mod title_demo;
mod warrior;

use crate::app::{AppState, Paused};
use crate::save::SaveData;
use bevy::prelude::*;
use game_utils_bevy::save::SaveManager;
use game_utils_bevy::transitions::Transition;

pub use components::*;
pub use round_manager::{ChooseReward, HitConfirmed, RoundManager, RunPhase};

#[derive(Resource, Default)]
pub struct OfflineBonesEarned(pub u32);

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoundManager>()
            .init_resource::<RestartFlag>()
            .init_resource::<OfflineBonesEarned>()
            .add_message::<HitConfirmed>()
            .add_message::<ChooseReward>()
            .add_plugins(title_demo::TitleDemoPlugin)
            .add_systems(Startup, (audio_fx::load_combat_sfx, load_warrior_textures))
            .add_systems(OnEnter(AppState::Title), grant_offline_bones)
            .add_systems(
                OnEnter(AppState::InGame),
                (arena::spawn_arena, round_manager::begin_run).chain(),
            )
            .add_systems(OnExit(AppState::InGame), cleanup_game)
            .add_systems(
                Update,
                (
                    warrior::active_puppet_motor,
                    warrior::tick_motor_state,
                    player::player_aim_and_bow,
                    enemy_ai::enemy_ai_system,
                    warrior::sync_bow_draw_visuals,
                    arrow::update_arrows,
                    warrior::apply_ragdoll_on_death,
                    warrior::apply_archetype_visuals,
                    round_manager::spawn_enemies_system,
                    round_manager::on_hits,
                    round_manager::apply_reward_system,
                    round_manager::finalize_game_over_bones,
                    hud_sync::sync_run_to_ui,
                    (warrior::sync_world_health_bars, warrior::sync_health_fills),
                    handle_restart_input,
                    process_restart,
                    death_fade,
                    cleanup_bounds::cleanup_far_entities,
                    debug_invariants::validate_round_manager,
                    debug_invariants::validate_warriors,
                )
                    .run_if(in_state(AppState::InGame))
                    .run_if(|p: Res<Paused>| !p.0)
                    .run_if(|t: Res<Transition<AppState>>| !t.block_input),
            );
    }
}

fn load_warrior_textures(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let art = art::bake_warrior_art(&mut images)
        .expect("failed to rasterize SVG art via renamite/repose");
    commands.insert_resource(art);
}

/// Passive bones while away. Runs on every title entry; self-limits by
/// bumping `last_played_unix` on grant, so re-entry yields zero.
fn grant_offline_bones(
    mut save: ResMut<SaveData>,
    manager: Res<SaveManager>,
    mut earned: ResMut<OfflineBonesEarned>,
) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut amount: u32 = 0;

    if save.total_runs > 0 && save.last_played_unix > 0 {
        let elapsed = now.saturating_sub(save.last_played_unix).min(6 * 3600);
        let per_min = 1 + save.meta_level("fortune");
        amount = (elapsed / 60) as u32 * per_min;
        if amount > 0 {
            save.bones += amount;
            save.last_played_unix = now;
            let _ = manager.save(&*save);
        }
    }

    earned.0 = amount;
}

#[derive(Resource, Default)]
pub struct RestartFlag(pub bool);

fn cleanup_game(mut commands: Commands, q: Query<Entity, With<GameCleanup>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn handle_restart_input(
    keys: Res<ButtonInput<KeyCode>>,
    rm: Res<RoundManager>,
    mut flag: ResMut<RestartFlag>,
) {
    if rm.phase != RunPhase::GameOver {
        return;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        flag.0 = true;
    }
}

fn process_restart(
    mut flag: ResMut<RestartFlag>,
    mut commands: Commands,
    cleanup: Query<Entity, With<GameCleanup>>,
    rm: ResMut<RoundManager>,
    save: Res<crate::save::SaveData>,
    textures: Res<art::WarriorArt>,
) {
    if !flag.0 {
        return;
    }

    flag.0 = false;

    for e in &cleanup {
        commands.entity(e).despawn();
    }

    arena::spawn_arena(commands.reborrow());
    round_manager::begin_run(rm, commands, save, textures);
}

fn death_fade(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut round_manager::DyingFade)>,
    children_q: Query<&Children>,
    mut sprites: Query<&mut Sprite>,
    bars: Query<(Entity, &WorldHealthBar)>,
    arrows: Query<(Entity, &Arrow)>,
    limbs: Query<&WarriorLimb>,
) {
    for (entity, mut fade) in &mut q {
        fade.0 -= time.delta_secs();
        let alpha = (fade.0 / 2.0).clamp(0.0, 1.0);

        fade_recursive(entity, alpha, &children_q, &mut sprites);

        if fade.0 <= 0.0 {
            // despawn() recursively removes all ChildOf descendants: every
            // limb body, its joints, the bow pivot + bow sprite.
            commands.entity(entity).despawn();
            // World-space health bar is not a child; tear it down explicitly.
            for (bar_e, bar) in &bars {
                if bar.root == entity {
                    commands.entity(bar_e).despawn();
                }
            }
            // Arrows stuck into this warrior's limbs die with it.
            for (arrow_e, arrow) in &arrows {
                if let Some(parent) = arrow.stuck_to
                    && limbs.get(parent).is_ok_and(|l| l.root == entity)
                {
                    commands.entity(arrow_e).despawn();
                }
            }
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
