use std::sync::{Arc, Mutex};

use bevy_ecs::prelude::*;

use crate::sim::rounds::RunState;

use crate::app::{AppState, OverlayMenu};

#[derive(Clone, Debug)]
pub struct BoneShopItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub level: u32,
    pub max_level: u32,
    pub cost: u32,
    pub maxed: bool,
}

#[derive(Clone, Debug, Default)]
pub struct RewardCardUi {
    pub id: String,
    pub title: String,
    pub description: String,
    pub rarity: u8,
    pub current_stacks: u32,
    pub max_stacks: u32,
    pub category: String,
    pub accent_rgb: [u8; 3],
}

pub fn rarity_accent(rarity: u8) -> [u8; 3] {
    match rarity {
        5 => [245, 190, 70],
        4 => [190, 130, 240],
        3 => [90, 150, 240],
        2 => [120, 200, 130],
        _ => [150, 160, 175],
    }
}

pub fn rarity_name(rarity: u8) -> &'static str {
    match rarity {
        5 => "Legendary",
        4 => "Epic",
        3 => "Rare",
        2 => "Uncommon",
        _ => "Common",
    }
}

#[derive(Clone)]
pub struct SharedUi {
    pub phase: AppState,
    pub paused: bool,
    pub loading_progress: f32,
    pub overlay: OverlayMenu,
    pub master_vol: f32,
    pub sfx_vol: f32,
    pub music_vol: f32,
    pub high_score: u32,
    pub score: u32,
    pub transition_alpha: f32,
    pub flash_alpha: f32,
    pub language: String,
    pub saved_language: String,
    pub available_languages: Vec<String>,
    pub translations: std::collections::HashMap<String, String>,
    pub bones: u32,
    pub best_round: u32,
    pub total_runs: u32,
    pub total_victories: u32,
    pub run_round: u32,
    pub run_score: u32,
    pub run_phase: u8,
    pub player_hp: i32,
    pub player_max_hp: i32,
    pub enemy_hp: i32,
    pub enemy_max_hp: i32,
    pub reward_cards: Vec<RewardCardUi>,
    pub bones_earned: u32,
    pub victory: bool,
    pub end_reason: String,
    pub boss_banner_timer: f32,
    pub status_line: String,
    pub draw_power: f32,
    pub drawing: bool,
    pub bone_shop_items: Vec<BoneShopItem>,
    pub offline_bones: u32,
}

impl Default for SharedUi {
    fn default() -> Self {
        Self {
            phase: AppState::Splash,
            paused: false,
            loading_progress: 1.0,
            overlay: OverlayMenu::None,
            master_vol: 1.0,
            sfx_vol: 1.0,
            music_vol: 0.8,
            high_score: 0,
            score: 0,
            transition_alpha: 0.0,
            flash_alpha: 0.0,
            language: "en".to_string(),
            saved_language: "en".to_string(),
            available_languages: vec!["en".to_string()],
            translations: Default::default(),
            bones: 0,
            best_round: 0,
            total_runs: 0,
            total_victories: 0,
            run_round: 0,
            run_score: 0,
            run_phase: 0,
            player_hp: 0,
            player_max_hp: 0,
            enemy_hp: 0,
            enemy_max_hp: 0,
            reward_cards: Vec::new(),
            bones_earned: 0,
            victory: false,
            end_reason: String::new(),
            boss_banner_timer: 0.0,
            status_line: String::new(),
            draw_power: 0.0,
            drawing: false,
            bone_shop_items: Vec::new(),
            offline_bones: 0,
        }
    }
}

#[derive(Resource, Clone, Default)]
pub struct UiActions(pub Arc<Mutex<Vec<crate::app::UiAction>>>);

pub fn drain_actions(world: &mut World) -> Vec<crate::app::UiAction> {
    if let Some(actions) = world.get_resource::<UiActions>() {
        std::mem::take(&mut *actions.0.lock().unwrap())
    } else {
        Vec::new()
    }
}

pub fn push_action(world: &mut World, action: crate::app::UiAction) {
    if let Some(actions) = world.get_resource::<UiActions>() {
        actions.0.lock().unwrap().push(action);
    }
}

pub fn sync_shared_ui(app: &mut crate::app::App, ui: &mut SharedUi) {
    let world = &mut app.sim.world;
    ui.phase = app.state;
    ui.paused = app.paused;
    ui.overlay = app.overlay;
    let save = world.resource::<crate::sim::save::SaveData>().clone();
    let rm = world.resource::<crate::sim::rounds::RunState>().clone();
    ui.high_score = save.high_score;
    ui.bones = save.bones;
    ui.best_round = save.best_round;
    ui.total_runs = save.total_runs;
    ui.total_victories = save.total_victories;
    ui.score = rm.score;
    ui.bone_shop_items = crate::sim::meta::CATALOG
        .iter()
        .map(|d| {
            let lvl = save.meta_level(d.id);
            BoneShopItem {
                id: d.id.to_string(),
                name: d.name.to_string(),
                description: d.description.to_string(),
                level: lvl,
                max_level: d.max_level,
                cost: if lvl >= d.max_level {
                    0
                } else {
                    crate::sim::meta::cost_for(&save, d.id)
                },
                maxed: lvl >= d.max_level,
            }
        })
        .collect();
    let levels = world.resource::<crate::sim::audio::AudioLevels>();
    if ui.overlay != OverlayMenu::Settings {
        ui.master_vol = levels.master;
        ui.sfx_vol = levels.sfx;
        ui.music_vol = levels.music;
    }
    ui.transition_alpha = 0.0;
    if let Some(flash) = world.get_resource::<repame_fx::Flash>() {
        ui.flash_alpha = flash.rgba().map(|c| c[3]).unwrap_or(0.0);
    }
    if let Some(i18n) = world.get_resource::<crate::sim::i18n::I18nStrings>() {
        ui.language = i18n.0.current.clone();
        ui.available_languages = i18n.0.available.clone();
        ui.translations = game_utils::i18n::get_current_translations(&i18n.0);
    }
    ui.run_round = rm.round;
    ui.run_score = rm.score;
    ui.run_phase = match rm.phase {
        crate::sim::rounds::RunPhase::Combat => 0,
        crate::sim::rounds::RunPhase::Reward => 1,
        crate::sim::rounds::RunPhase::GameOver => 2,
    };
    ui.bones_earned = rm.bones_earned;
    ui.victory = rm.victory;
    ui.reward_cards = rm
        .reward_choices
        .iter()
        .map(|r| {
            let stacks = rm.upgrade_counts.get(r.id).copied().unwrap_or(0);
            RewardCardUi {
                id: r.id.to_string(),
                title: r.title.to_string(),
                description: r.description.to_string(),
                rarity: r.rarity,
                current_stacks: stacks,
                max_stacks: r.max_stacks,
                category: r.category.to_string(),
                accent_rgb: rarity_accent(r.rarity),
            }
        })
        .collect();
    let warriors: Vec<(Option<bool>, i32, i32)> = world
        .query::<(
            Option<&crate::sim::comps::PlayerTag>,
            &crate::sim::comps::WarriorRoot,
        )>()
        .iter(world)
        .map(|(p, w)| (p.map(|_| true), w.health, w.max_health))
        .collect();
    if let Some((_, hp, max)) = warriors.iter().find(|(p, _, _)| p.is_some()) {
        ui.player_hp = *hp;
        ui.player_max_hp = *max;
    }
    let active = world
        .resource::<crate::sim::rounds::RunState>()
        .active_enemy;
    if let Some(e) = active
        && let Some(w) = world.get::<crate::sim::comps::WarriorRoot>(e)
    {
        ui.enemy_hp = w.health;
        ui.enemy_max_hp = w.max_health;
    } else {
        ui.enemy_hp = 0;
        ui.enemy_max_hp = 0;
    }
    if let Some(p) = world.resource::<crate::sim::rounds::RunState>().player
        && let Some(bow) = world.get::<crate::sim::comps::BowState>(p)
    {
        ui.drawing = bow.drawing;
        ui.draw_power = if bow.drawing { bow.draw_power } else { 0.0 };
    }
    if ui.boss_banner_timer > 0.0 {
        ui.boss_banner_timer = (ui.boss_banner_timer - 1.0 / 60.0).max(0.0);
    }
    if rm.round != 0
        && rm.round.is_multiple_of(5)
        && rm.phase == crate::sim::rounds::RunPhase::Combat
    {
        ui.boss_banner_timer = 2.5;
    }
    ui.end_reason = if rm.phase == crate::sim::rounds::RunPhase::GameOver {
        if rm.victory {
            format!("Cleared round {}", rm.round)
        } else {
            format!("Fell on round {}", rm.round)
        }
    } else {
        String::new()
    };
    ui.status_line = String::new();
}
