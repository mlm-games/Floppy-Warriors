# Floppy Warriors

> Janky ragdoll archery roguelite. Draw, flop, headshot. Die, spend Bones, go again. Endless after the first clear.

A Bevy 2D game built on top of the [mlm-games ecosystem](https://github.com/mlm-games/game-utils) plugins (transitions, screen effects, vfx, save, i18n, audio).

## Gameplay

- Aim with the mouse, hold **Left Click** (or **Space**) to draw your bow, release to fire.
- **Shift** (either) is an airdodge.
- **Esc** pauses.
- Enemies come in 6 archetypes: **Grunt, Fast, Tank, Sniper, Splitter, Boss** - each with distinct visuals, health, fire rate, and behavior.
- Headshots hurt harder, crits double damage, and killing shots trigger slow-mo.
- Between rounds pick one of three weighted rewards from a draft deck (stacks, gating, crits, lifesteal, revive, glass cannon, last stand).
- Spend **Bones** in the Bone Shop on meta upgrades. Offline time also drips Bones while you're away.

## Controls

| Input | Action |
|-------|--------|
| Mouse | Aim |
| Hold Left Click / Space | Draw bow |
| Release | Fire |
| Shift (L or R) | Airdodge |
| Esc | Pause / Settings |
| R (on game over) | Retry |

## Running

```bash
# Full physics build (default)
cargo run --features physics

# Physics-free build
cargo run --no-default-features

# Web (WASM)
# cargo build --release --target wasm32-unknown-unknown
```

## Features

- Active-ragdoll puppet with force-driven standing (no hard lock) and a full flop on death
- Manual projectile arrows that stick to limbs and the ground
- Hitstop / headshot slow-mo, trauma shake, damage numbers, particle bursts
- Offline bones drip, reward deck with memory, endless post-clear loop
- 7 locales (en, es, fr, de, ja, zh, pt), channel-based audio, persistent RON save
- Packaging for AUR, Chocolatey, Flatpak, Snap

## Structure

```
src/
├── app.rs               # AppPlugin, states, UI bridge, settings
├── game/
│   ├── arena.rs         # Backdrop + ground
│   ├── arrow.rs         # Manual projectile motion + hit tests
│   ├── audio_fx.rs      # SFX handles + load-state-gated playback
│   ├── cleanup_bounds.rs# Safety net for arrows / corpses out of play
│   ├── components.rs    # Warrior, limbs, arrows, mods, archetypes
│   ├── debug_invariants.rs # Debug-build run-loop assertions
│   ├── enemy_ai.rs      # Enemy decision loop + archetype config
│   ├── hud_sync.rs      # HUD bridge (typed reward cards)
│   ├── meta.rs          # Bone Shop catalog + meta scaling
│   ├── mod.rs           # GamePlugin, offline bones, cleanup
│   ├── player.rs        # Mouse aim + bow + airdodge
│   ├── round_manager.rs # Rounds, rewards, endless, scoring
│   └── warrior.rs       # Puppet spawn, joints, active ragdoll, fire
├── menus/               # Title, pause, settings, credits, Bone Shop (Repose)
├── screens/             # Splash, loading, title state machine
├── save.rs              # SaveData + meta levels + migration
└── asset_tracking.rs    # Preload tracking
```

## Roadmap (post-v1)

Music loop, a few more SFX variants, controller bindings, Steam page assets, explosive reward.

## License

GPL-3.0. SFX should come from CC0 packs (e.g. Kenney Impact / UI Audio) for commercial use.
