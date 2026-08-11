use bevy::prelude::*;
use game_utils_bevy::audio::AudioM;

#[derive(Resource)]
pub struct CombatSfx {
    pub bow_draw: Handle<AudioSource>,
    pub bow_release: Handle<AudioSource>,
    pub hit: Handle<AudioSource>,
    pub headshot: Handle<AudioSource>,
    pub kill: Handle<AudioSource>,
    pub death: Handle<AudioSource>,
    pub reward: Handle<AudioSource>,
}

pub fn load_combat_sfx(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(CombatSfx {
        bow_draw: asset_server.load("audio/sfx/bow_draw.ogg"),
        bow_release: asset_server.load("audio/sfx/bow_release.ogg"),
        hit: asset_server.load("audio/sfx/arrow_hit.ogg"),
        headshot: asset_server.load("audio/sfx/arrow_headshot.ogg"),
        kill: asset_server.load("audio/sfx/kill.ogg"),
        death: asset_server.load("audio/sfx/death.ogg"),
        reward: asset_server.load("audio/sfx/reward.ogg"),
    });
}

/// Plays a one-shot SFX with pitch variance, but only once the asset is
/// actually loaded. This keeps the game silent-and-safe if a pack hasn't been
/// dropped into `assets/audio/sfx/` yet instead of spamming load errors.
pub fn play_sfx(
    commands: &mut Commands,
    asset_server: &AssetServer,
    handle: &Handle<AudioSource>,
    volume: f32,
    pitch_var: f32,
) {
    if asset_server
        .get_load_state(handle.id())
        .is_some_and(|s| s.is_loaded())
    {
        AudioM::play_sfx_varied(commands, handle.clone(), volume, pitch_var);
    }
}
