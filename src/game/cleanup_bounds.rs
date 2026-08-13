use bevy::prelude::*;

use super::components::*;
use super::round_manager::DyingFade;

const MAX_X: f32 = 3000.0;
const MIN_X: f32 = -3000.0;
const MAX_Y: f32 = 2000.0;
const MIN_Y: f32 = -3000.0;

/// Endless-mode safety net: anything that escaped out of play bounds is
/// garbage (a knocked-away corpse, a glitched arrow) and should not linger
/// even if its normal lifetime path was missed.
pub fn cleanup_far_entities(
    mut commands: Commands,
    arrows: Query<(Entity, &Transform), With<Arrow>>,
    fading: Query<(Entity, &Transform), With<DyingFade>>,
) {
    for (e, tf) in &arrows {
        let p = tf.translation;
        if p.x < MIN_X || p.x > MAX_X || p.y < MIN_Y || p.y > MAX_Y {
            commands.entity(e).despawn();
        }
    }

    for (e, tf) in &fading {
        let p = tf.translation;
        if p.x < MIN_X || p.x > MAX_X || p.y < MIN_Y || p.y > MAX_Y {
            commands.entity(e).despawn();
        }
    }
}
