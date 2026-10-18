use std::collections::HashSet;

use bevy_ecs::prelude::*;
use game_utils::save_store::LoadStatus;
use game_utils::storage::Storage as _;
use glam::Vec2;
use repose_core::input::PhysicalKey;

use crate::sim::input::PlayerInput;
use crate::sim::rounds::{RewardPick, RunPhase, RunState};

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppState {
    #[default]
    Splash,
    Loading,
    Title,
    InGame,
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayMenu {
    #[default]
    None,
    Settings,
    Credits,
    Pause,
    BoneShop,
}

pub use crate::ui::UiAction;

/// World units per dp. The art, ragdoll metrics and arena constants are all
/// authored at one world unit per dp, so a HiDPI surface changes the pixel
/// count, never the world-to-dp scale.
const WORLD_PER_DP: f32 = 1.0;

/// Seconds per screen fade, matching the Godot original's `fadeToBlack` length.
const FADE_SECS: f32 = 0.2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingScreen {
    Title,
    Loading,
    Run,
}

pub struct App {
    pub sim: repame_sim::Sim,
    schedule: Schedule,
    pub state: AppState,
    pub overlay: OverlayMenu,
    pub paused: bool,
    pending_unpause: f32,
    splash_timer: f32,
    loading_timer: f32,
    bake: Option<crate::render::assets::AssetBake>,
    pub(crate) fade: f32,
    pending: Option<PendingScreen>,
    pub(crate) assets: Option<crate::render::assets::RenderAssets>,
    uploads: std::sync::Arc<[repame_view2d::AtlasUpload]>,
    batch_desc: Option<repame_view2d::BatchDesc>,
    pub audio: crate::sim::audio::AudioHost,
    save: game_utils::save::SaveManager,
    save_dirty: bool,
    pub(crate) offline_bones: u32,
    ui: crate::ui::SharedUi,
    pub(crate) last_boss_round: u32,
    staging: std::rc::Rc<std::cell::RefCell<repame_shell::Staging>>,
    shortcut_edges: repame_shell::SharedEdges,
    pause_edge: bool,
    restart_edge: bool,
    interact_edge: bool,
    held: HashSet<PhysicalKey>,
    edges: Vec<PhysicalKey>,
    window_focused: bool,
    lmb_held: bool,
    lmb_was_down: bool,
    rmb_held: bool,
    space_was_held: bool,
    z_was_held: bool,
    touch_was_held: bool,
    pad: repame_input::GamepadState,
    pad_was_down: bool,
    cam: repame_view2d::Camera2d,
    raw_elapsed: f32,
    view_world_size: [f32; 2],
}

impl App {
    pub fn new() -> Self {
        let mut sim = repame_sim::Sim::with_default_step();
        repame_fx::init_resources(&mut sim.world);
        sim.world
            .insert_resource(crate::sim::save::SaveData::default());
        sim.world
            .insert_resource(crate::sim::audio::CueQueue::default());
        sim.world
            .insert_resource(crate::sim::audio::AudioLevels::default());
        sim.world
            .insert_resource(crate::sim::input::PlayerInput::default());
        sim.world
            .insert_resource(crate::sim::input::RestartRequest::default());
        sim.world.insert_resource(RunState::default());
        sim.world
            .insert_resource(crate::sim::rounds::HitEvents::default());
        sim.world
            .insert_resource(crate::sim::rounds::RewardPick::default());
        sim.world
            .insert_resource(crate::sim::rounds::SlowMo::default());
        sim.world
            .insert_resource(crate::sim::rounds::GroundLine::default());
        sim.world
            .insert_resource(crate::sim::state::SimStep::default());
        sim.world
            .insert_resource(crate::sim::state::GameOverFlushed::default());
        sim.world
            .insert_resource(crate::sim::title_demo::DemoRespawn::default());
        sim.world.insert_resource(crate::ui::UiActions::default());
        sim.world.init_resource::<repame_fx::Trauma>();
        sim.world.init_resource::<repame_fx::Flash>();
        sim.world.init_resource::<crate::sim::arena::ViewExtent>();
        repame_rapier2d::init_world(&mut sim.world, glam::Vec2::new(0.0, 980.0), 20.0);
        repame_rapier2d::register_rapier2d_systems(&mut sim);
        crate::sim::i18n::register(&mut sim.world);
        let schedule = crate::sim::state::build_schedule();
        let staging = repame_shell::Staging::shared();
        Self {
            sim,
            schedule,
            state: AppState::Splash,
            overlay: OverlayMenu::None,
            paused: false,
            pending_unpause: 0.0,
            splash_timer: 1.5,
            loading_timer: 0.5,
            bake: None,
            fade: 0.0,
            pending: None,
            assets: None,
            uploads: std::sync::Arc::default(),
            batch_desc: None,
            audio: crate::sim::audio::AudioHost::new(),
            save: game_utils::save::SaveManager::new(
                "org.mlm.floppy",
                "mlm-games",
                "floppy-warriors",
                "save.ron",
                crate::sim::save::SAVE_VERSION,
            ),
            save_dirty: false,
            offline_bones: 0,
            ui: crate::ui::SharedUi::default(),
            last_boss_round: 0,
            staging,
            shortcut_edges: repame_shell::shared_edges(),
            pause_edge: false,
            restart_edge: false,
            interact_edge: false,
            held: HashSet::new(),
            edges: Vec::new(),
            window_focused: true,
            lmb_held: false,
            lmb_was_down: false,
            rmb_held: false,
            space_was_held: false,
            z_was_held: false,
            touch_was_held: false,
            pad: repame_input::GamepadState::default(),
            pad_was_down: false,
            cam: repame_view2d::Camera2d::default(),
            raw_elapsed: 0.0,
            view_world_size: crate::sim::arena::WORLD_VIEW,
        }
    }

    pub fn load_save(&mut self) {
        let (mut save, status) = self.save.load_with_status::<crate::sim::save::SaveData>();
        if status == LoadStatus::Missing {
            let legacy = game_utils::save::SaveManager::new(
                "godot",
                "Godot",
                "app_userdata/Floppy Warriors",
                "floppy_meta.json",
                1,
            );
            if let Some(imported) = legacy
                .storage()
                .read(&legacy.path())
                .ok()
                .flatten()
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .and_then(|text| crate::sim::save::SaveData::from_godot_json(&text).ok())
            {
                log::info!("imported Godot floppy_meta.json");
                save = imported;
                self.save_dirty = true;
            }
        }
        save.migrate();
        self.apply_locale(&save.settings.language);
        let levels = crate::sim::audio::AudioLevels {
            master: save.settings.master_volume,
            sfx: save.settings.sfx_volume,
            music: save.settings.music_volume,
        };
        self.sim.world.insert_resource(save);
        self.sim.world.insert_resource(levels);
        self.audio
            .set_volume(repame_audio::AudioChannel::Master, levels.master);
        self.audio
            .set_volume(repame_audio::AudioChannel::Sfx, levels.sfx);
        self.audio
            .set_volume(repame_audio::AudioChannel::Music, levels.music);
    }

    fn save_now(&mut self) {
        if !self.save_dirty {
            return;
        }
        self.save_dirty = false;
        let mut save = self
            .sim
            .world
            .resource::<crate::sim::save::SaveData>()
            .clone();
        if let Err(error) = self.save.save_versioned(&mut save) {
            log::error!("save failed: {error}");
            self.save_dirty = true;
        }
    }

    pub fn mark_save_dirty(&mut self) {
        self.save_dirty = true;
    }

    fn stamp_last_played(&mut self) {
        let now = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map(|value| value.as_secs())
            .unwrap_or(0);
        self.sim
            .world
            .resource_mut::<crate::sim::save::SaveData>()
            .last_played_unix = now;
        self.mark_save_dirty();
    }

    fn apply_locale(&mut self, lang: &str) {
        if let Some(mut i18n) = self
            .sim
            .world
            .get_resource_mut::<crate::sim::i18n::I18nStrings>()
        {
            i18n.0.set_locale(lang);
        }
    }

    pub fn load_progress(&self) -> f32 {
        match (&self.assets, &self.bake) {
            (Some(_), _) => 1.0,
            (None, Some(bake)) => bake.progress(),
            (None, None) => 0.0,
        }
    }

    /// Bakes one embedded asset per frame so the loading screen keeps animating.
    fn advance_bake(&mut self) {
        if self.assets.is_some() {
            self.bake = None;
            return;
        }
        let mut bake = self
            .bake
            .take()
            .unwrap_or_else(crate::render::assets::AssetBake::new);
        if bake.advance() {
            let mut assets = bake.finish();
            self.batch_desc = Some(assets.batch_desc());
            self.uploads = assets.take_uploads().into();
            self.assets = Some(assets);
        } else {
            self.bake = Some(bake);
        }
    }

    fn request(&mut self, screen: PendingScreen) {
        if self.pending.is_none() {
            self.pending = Some(screen);
        }
    }

    /// Fades to black, swaps the screen while covered, then fades back in.
    /// Returns true while the transition owns the frame.
    fn tick_transition(&mut self, dt: f32) -> bool {
        let Some(screen) = self.pending else {
            if self.fade > 0.0 {
                self.fade = (self.fade - dt / FADE_SECS).max(0.0);
            }
            return false;
        };
        self.fade = (self.fade + dt / FADE_SECS).min(1.0);
        if self.fade < 1.0 {
            return true;
        }
        self.pending = None;
        match screen {
            PendingScreen::Title => self.goto_title(),
            PendingScreen::Loading => self.goto_loading(),
            PendingScreen::Run => self.begin_run(),
        }
        true
    }

    fn grant_offline_bones(&mut self) -> u32 {
        let now = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let mut amount = 0u32;
        {
            let mut save = self.sim.world.resource_mut::<crate::sim::save::SaveData>();
            if save.total_runs > 0 && save.last_played_unix > 0 {
                let elapsed = now.saturating_sub(save.last_played_unix).min(6 * 3600);
                let per_min = 1 + save.meta_level("fortune");
                amount = (elapsed / 60) as u32 * per_min;
                if amount > 0 {
                    save.bones += amount;
                    save.last_played_unix = now;
                }
            }
        }
        if amount > 0 {
            self.mark_save_dirty();
        }
        amount
    }

    fn goto_title(&mut self) {
        self.clear_sim_entities();
        crate::sim::arena::spawn_ground(&mut self.sim.world);
        crate::sim::title_demo::title_demo_setup(&mut self.sim.world);
        self.state = AppState::Title;
        self.overlay = OverlayMenu::None;
        self.paused = false;
    }

    fn goto_loading(&mut self) {
        if self.assets.is_some() {
            self.request(PendingScreen::Run);
            return;
        }
        self.state = AppState::Loading;
        self.loading_timer = 0.5;
        if self.bake.is_none() {
            self.bake = Some(crate::render::assets::AssetBake::new());
        }
    }

    fn begin_run(&mut self) {
        self.clear_sim_entities();
        crate::sim::arena::spawn_ground(&mut self.sim.world);
        crate::sim::rounds::begin_run(&mut self.sim.world);
        self.state = AppState::InGame;
        self.overlay = OverlayMenu::None;
        self.paused = false;
    }

    fn clear_sim_entities(&mut self) {
        crate::sim::title_demo::title_demo_teardown(&mut self.sim.world);
        let roots: Vec<Entity> = self
            .sim
            .world
            .query_filtered::<Entity, With<crate::sim::comps::WarriorRoot>>()
            .iter(&self.sim.world)
            .collect();
        for root in roots {
            crate::sim::warrior::despawn_warrior(&mut self.sim.world, root);
        }

        let leftovers: Vec<Entity> = [
            self.sim
                .world
                .query_filtered::<Entity, With<crate::sim::comps::WarriorLimb>>()
                .iter(&self.sim.world)
                .collect::<Vec<_>>(),
            self.sim
                .world
                .query_filtered::<Entity, With<crate::sim::comps::Arrow>>()
                .iter(&self.sim.world)
                .collect(),
            self.sim
                .world
                .query_filtered::<Entity, With<repame_rapier2d::RapierRevolute>>()
                .iter(&self.sim.world)
                .collect(),
            self.sim
                .world
                .query_filtered::<Entity, With<crate::sim::arena::GroundTag>>()
                .iter(&self.sim.world)
                .collect(),
            self.sim
                .world
                .query_filtered::<Entity, With<repame_fx::Particle>>()
                .iter(&self.sim.world)
                .collect(),
            self.sim
                .world
                .query_filtered::<Entity, With<repame_fx::DamageNumber>>()
                .iter(&self.sim.world)
                .collect(),
        ]
        .into_iter()
        .flatten()
        .collect();
        for entity in leftovers {
            self.sim.world.despawn(entity);
        }

        self.sim.world.insert_resource(RunState::default());
        self.sim
            .world
            .insert_resource(crate::sim::rounds::HitEvents::default());
        self.sim
            .world
            .insert_resource(crate::sim::rounds::RewardPick::default());
        self.sim
            .world
            .insert_resource(crate::sim::rounds::SlowMo::default());
        self.sim
            .world
            .insert_resource(crate::sim::state::GameOverFlushed::default());
        self.sim.world.insert_resource(repame_fx::Trauma::default());
        self.sim.world.insert_resource(repame_fx::Flash::default());
        self.sim
            .world
            .insert_resource(crate::sim::input::PlayerInput::default());
        self.sim
            .world
            .insert_resource(crate::sim::input::RestartRequest::default());
        self.sim
            .world
            .insert_resource(crate::ui::UiActions::default());

        self.held.clear();
        self.edges.clear();
        self.pause_edge = false;
        self.restart_edge = false;
        self.interact_edge = false;
        self.lmb_held = false;
        self.lmb_was_down = false;
        self.rmb_held = false;
        self.space_was_held = false;
        self.z_was_held = false;
        self.touch_was_held = false;
        self.pad = repame_input::GamepadState::default();
        self.pad_was_down = false;
        self.pending_unpause = 0.0;
        self.last_boss_round = 0;
        self.ui.boss_banner_timer = 0.0;
        let mut staging = self.staging.borrow_mut();
        staging.set_window_focused(false);
        staging.set_window_focused(true);
    }

    fn feed_polled(&mut self, sched: &repose_core::runtime::Scheduler) {
        let before = self.held.clone();
        let held = {
            let mut staging = self.staging.borrow_mut();
            staging.feed_polled(sched);
            staging.held.clone()
        };
        self.held = held;
        self.window_focused = sched.window_focused;
        if self.window_focused {
            self.edges.extend(self.held.difference(&before).copied());
        } else {
            self.edges.clear();
            self.cancel_combat_input();
        }
        if !sched.mouse_primary {
            self.lmb_held = false;
        }
        if !sched.mouse_secondary {
            self.rmb_held = false;
        }
    }

    fn feed_input(&mut self, sched: &repose_core::runtime::Scheduler) {
        let staging = self.staging.clone();
        let mut staging = staging.borrow_mut();
        let staging_edges = staging.take_edges();
        let mouse_edges = staging.take_mouse_edges();
        let staging_lmb = staging.lmb_held;
        let staging_rmb_down = staging.take_rmb_down();
        let pads = staging.take_pads();
        let touches = staging.touch_contacts();
        let _clicks = staging.take_clicks();
        drop(staging);
        self.pad = pads.first().copied().unwrap_or_default();
        for code in staging_edges {
            if self.held.insert(code) {
                self.edges.push(code);
            }
        }
        let just: HashSet<PhysicalKey> = self.edges.drain(..).collect();
        let in_game_combat = self.state == AppState::InGame
            && self.window_focused
            && !self.paused
            && self.overlay == OverlayMenu::None
            && self.sim.world.resource::<RunState>().phase == RunPhase::Combat;
        let rmb_down = staging_rmb_down;
        if self.state == AppState::InGame {
            if self.pause_edge {
                self.pause_edge = false;
                self.toggle_pause();
            }
            let restart_pressed =
                self.restart_edge || just.contains(&repose_core::input::PhysicalKey::KeyR);
            self.restart_edge = false;
            if restart_pressed && self.sim.world.resource::<RunState>().phase == RunPhase::GameOver
            {
                self.sim
                    .world
                    .resource_mut::<crate::sim::input::RestartRequest>()
                    .0 = true;
            }
            let restart = self
                .sim
                .world
                .resource::<crate::sim::input::RestartRequest>()
                .0;
            if restart {
                self.sim
                    .world
                    .resource_mut::<crate::sim::input::RestartRequest>()
                    .0 = false;
                self.request(PendingScreen::Run);
                return;
            }
        }
        if self.interact_edge {
            self.interact_edge = false;
            self.apply_confirm();
        }
        let touch_down = !touches.is_empty();
        let touch_pressed = touches.iter().any(|contact| contact.just_pressed);
        let touch_released = self.touch_was_held && !touch_down && in_game_combat;
        self.touch_was_held = touch_down;
        let pad_down = self.pad.right_trigger_held;
        let pad_pressed = self.pad.right_trigger_pressed;
        let pad_released = self.pad_was_down && !pad_down && in_game_combat;
        self.pad_was_down = pad_down;
        let pointer_down = staging_lmb || sched.mouse_primary;
        let lmb_down = pointer_down || touch_down || pad_down;
        let primary_pressed = mouse_edges.iter().any(|(button, pressed)| {
            *button == repose_core::input::PointerButton::Primary && *pressed
        });
        let primary_released = mouse_edges.iter().any(|(button, pressed)| {
            *button == repose_core::input::PointerButton::Primary && !*pressed
        });
        let lmb_pressed = (primary_pressed || (lmb_down && !self.lmb_was_down)) && in_game_combat;
        let lmb_released = (primary_released || (!lmb_down && self.lmb_was_down)) && in_game_combat;
        self.lmb_was_down = lmb_down;
        let space_currently_held = self.held.contains(&repose_core::input::PhysicalKey::Space);
        let space_edge = just.contains(&repose_core::input::PhysicalKey::Space);
        let space_released = self.space_was_held && !space_currently_held && in_game_combat;
        self.space_was_held = space_currently_held;
        let z_currently_held = self.held.contains(&repose_core::input::PhysicalKey::KeyZ);
        let z_edge = just.contains(&repose_core::input::PhysicalKey::KeyZ);
        let z_released = self.z_was_held && !z_currently_held && in_game_combat;
        self.z_was_held = z_currently_held;
        let aim_world = self.sim.world.resource::<PlayerInput>().aim_world;
        let stick = if in_game_combat {
            let keyboard = keyboard_stick(&self.held);
            if keyboard == Vec2::ZERO {
                repame_input::apply_stick(self.pad.right_stick)
            } else {
                keyboard
            }
        } else {
            Vec2::ZERO
        };
        self.sim.world.insert_resource(PlayerInput {
            aim_world,
            stick,
            draw_pressed: (lmb_pressed || space_edge || z_edge || touch_pressed || pad_pressed)
                && in_game_combat,
            draw_released: (lmb_released
                || space_released
                || z_released
                || touch_released
                || pad_released)
                && in_game_combat,
            dodge_pressed: (just.contains(&repose_core::input::PhysicalKey::ShiftLeft)
                || just.contains(&repose_core::input::PhysicalKey::ShiftRight)
                || rmb_down
                || self.pad.left_trigger_pressed)
                && in_game_combat,
            lmb_held: lmb_down && in_game_combat,
        });
        self.lmb_held = lmb_down;
    }

    fn apply_confirm(&mut self) {
        match self.overlay {
            OverlayMenu::None => match self.state {
                AppState::Title => self.request(PendingScreen::Loading),
                AppState::InGame => {
                    let phase = self.sim.world.resource::<RunState>().phase;
                    if phase == RunPhase::GameOver {
                        self.request(PendingScreen::Run);
                    }
                }
                AppState::Splash | AppState::Loading => {}
            },
            OverlayMenu::Pause => self.apply_ui_action(UiAction::Resume),
            OverlayMenu::Settings | OverlayMenu::Credits | OverlayMenu::BoneShop => {
                self.apply_ui_action(UiAction::CloseOverlay)
            }
        }
    }

    pub fn on_pad_disconnected(&mut self) {
        self.cancel_combat_input();
    }

    fn cancel_combat_input(&mut self) {
        if let Some(player) = self.sim.world.resource::<RunState>().player
            && let Some(mut bow) = self
                .sim
                .world
                .get_mut::<crate::sim::comps::BowState>(player)
        {
            bow.drawing = false;
            bow.draw_power = 0.0;
        }
        let mut input = self.sim.world.resource_mut::<PlayerInput>();
        input.draw_pressed = false;
        input.draw_released = false;
        input.dodge_pressed = false;
        input.lmb_held = false;
        self.lmb_held = false;
        self.lmb_was_down = false;
        self.space_was_held = false;
        self.z_was_held = false;
        self.touch_was_held = false;
        self.pad = repame_input::GamepadState::default();
        self.pad_was_down = false;
    }

    fn toggle_pause(&mut self) {
        if self.paused || self.overlay == OverlayMenu::Pause {
            self.overlay = OverlayMenu::None;
            self.pending_unpause = 0.2;
        } else {
            self.cancel_combat_input();
            self.paused = true;
            self.overlay = OverlayMenu::Pause;
        }
    }

    pub fn step_fixed(&mut self) -> u32 {
        let step = self.sim.step;
        self.advance(step)
    }

    fn advance(&mut self, dt: std::time::Duration) -> u32 {
        if self.tick_transition(dt.as_secs_f32()) {
            return 0;
        }
        if self.state == AppState::Splash {
            self.splash_timer -= dt.as_secs_f32();
            if self.splash_timer <= 0.0 {
                self.request(PendingScreen::Title);
            }
            return 0;
        }
        if self.state == AppState::Loading {
            self.advance_bake();
            self.loading_timer -= dt.as_secs_f32();
            if self.loading_timer <= 0.0 && self.assets.is_some() {
                self.request(PendingScreen::Run);
            }
            return 0;
        }
        if self.pending_unpause > 0.0 {
            self.pending_unpause -= dt.as_secs_f32();
            if self.pending_unpause <= 0.0 {
                self.pending_unpause = 0.0;
                self.paused = false;
            }
        }
        if self.paused || self.overlay == OverlayMenu::Pause {
            return 0;
        }

        let state = self.state;
        let time_scale = self
            .sim
            .world
            .resource::<crate::sim::rounds::SlowMo>()
            .time_scale();
        self.sim.time_scale = time_scale;
        let raw_step_dt = self.sim.step.as_secs_f32();
        let step_dt = raw_step_dt * time_scale;
        let schedule = &mut self.schedule;
        let raw_elapsed = &mut self.raw_elapsed;
        let ran = self.sim.step_with(dt, |world| {
            *raw_elapsed += raw_step_dt;
            *world.resource_mut::<crate::sim::state::SimStep>() = crate::sim::state::SimStep {
                dt: step_dt,
                raw: raw_step_dt,
            };
            match state {
                AppState::Title => {
                    crate::sim::title_demo::demo_ai(world);
                    crate::sim::title_demo::update_demo_arrows(world);
                    crate::sim::title_demo::demo_respawn(world);
                    crate::sim::warrior::tick_motor_state_by_world(world);
                    crate::sim::fx::step_fx(world, raw_step_dt);
                }
                AppState::InGame => {
                    schedule.run(world);
                    crate::sim::fx::step_visual_fx(world, raw_step_dt);
                }
                AppState::Splash | AppState::Loading => {}
            }
        });

        if matches!(state, AppState::Title | AppState::InGame) {
            let cues = crate::sim::audio::drain_cues(&mut self.sim.world);
            for cue in cues {
                self.audio.play(&cue);
            }
        }
        if state == AppState::InGame
            && self.sim.world.resource::<RunState>().phase == RunPhase::GameOver
        {
            self.mark_save_dirty();
        }
        if state == AppState::InGame {
            self.save_now();
        }
        ran
    }

    fn apply_ui_action(&mut self, action: UiAction) {
        match action {
            UiAction::StartGame => self.request(PendingScreen::Loading),
            UiAction::OpenSettings => self.overlay = OverlayMenu::Settings,
            UiAction::OpenCredits => self.overlay = OverlayMenu::Credits,
            UiAction::CloseOverlay => {
                if self.paused {
                    self.overlay = OverlayMenu::Pause;
                } else {
                    self.overlay = OverlayMenu::None;
                }
            }
            UiAction::Resume => {
                self.overlay = OverlayMenu::None;
                self.pending_unpause = 0.2;
            }
            UiAction::Restart => {
                self.paused = false;
                self.overlay = OverlayMenu::None;
                self.pending_unpause = 0.0;
                self.request(PendingScreen::Run);
            }
            UiAction::QuitApp => {
                self.stamp_last_played();
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.save_now();
                    std::process::exit(0);
                }
                #[cfg(target_arch = "wasm32")]
                self.request(PendingScreen::Title);
            }
            UiAction::QuitToTitle => {
                self.stamp_last_played();
                self.paused = false;
                self.overlay = OverlayMenu::None;
                self.pending_unpause = 0.0;
                self.request(PendingScreen::Title);
            }
            UiAction::SetMasterVol(v) => {
                self.sim
                    .world
                    .resource_mut::<crate::sim::audio::AudioLevels>()
                    .master = v.clamp(0.0, 1.0);
                self.audio
                    .set_volume(repame_audio::AudioChannel::Master, v.clamp(0.0, 1.0));
            }
            UiAction::SetSfxVol(v) => {
                self.sim
                    .world
                    .resource_mut::<crate::sim::audio::AudioLevels>()
                    .sfx = v.clamp(0.0, 1.0);
                self.audio
                    .set_volume(repame_audio::AudioChannel::Sfx, v.clamp(0.0, 1.0));
            }
            UiAction::SetMusicVol(v) => {
                self.sim
                    .world
                    .resource_mut::<crate::sim::audio::AudioLevels>()
                    .music = v.clamp(0.0, 1.0);
                self.audio
                    .set_volume(repame_audio::AudioChannel::Music, v.clamp(0.0, 1.0));
            }
            UiAction::SaveSettings => {
                let levels = *self.sim.world.resource::<crate::sim::audio::AudioLevels>();
                {
                    let mut save = self.sim.world.resource_mut::<crate::sim::save::SaveData>();
                    save.settings.master_volume = levels.master;
                    save.settings.sfx_volume = levels.sfx;
                    save.settings.music_volume = levels.music;
                    save.last_played_unix = web_time::SystemTime::now()
                        .duration_since(web_time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                }
                self.mark_save_dirty();
                self.save_now();
                if self.paused {
                    self.overlay = OverlayMenu::Pause;
                } else {
                    self.overlay = OverlayMenu::None;
                }
            }
            UiAction::SetLanguage(lang) => {
                self.apply_locale(&lang);
                self.sim
                    .world
                    .resource_mut::<crate::sim::save::SaveData>()
                    .settings
                    .language = lang;
                self.mark_save_dirty();
            }
            UiAction::OpenBoneShop => self.overlay = OverlayMenu::BoneShop,
            UiAction::CloseBoneShop => self.overlay = OverlayMenu::None,
            UiAction::BuyMeta(id) => {
                let bought = {
                    let mut save = self.sim.world.resource_mut::<crate::sim::save::SaveData>();
                    crate::sim::meta::buy(&mut save, &id)
                };
                if bought {
                    self.mark_save_dirty();
                    self.save_now();
                }
            }
            UiAction::ChooseReward(i) => {
                let id = self
                    .sim
                    .world
                    .resource::<RunState>()
                    .reward_choices
                    .get(i)
                    .map(|r| r.id.to_string());
                if let Some(id) = id {
                    self.sim.world.resource_mut::<RewardPick>().0 = Some(id);
                }
            }
        }
    }

    fn prepare_camera(&mut self, size: [u32; 2]) -> ([f32; 2], f32) {
        let viewport_px = if size[0] == 0 || size[1] == 0 {
            [1280.0, 720.0]
        } else {
            [size[0] as f32, size[1] as f32]
        };
        let density = repose_core::locals::effective_density_scale().max(1e-6);
        let viewport_dp = [viewport_px[0] / density, viewport_px[1] / density];
        self.view_world_size = crate::sim::arena::WORLD_VIEW;
        let (shake_x, shake_y, shake_roll) = self
            .sim
            .world
            .get_resource::<repame_fx::Trauma>()
            .map(|trauma| trauma.camera_offset(self.raw_elapsed, WORLD_PER_DP, 1.0))
            .unwrap_or_default();
        self.cam = repame_view2d::Camera2d {
            center: crate::sim::arena::CAMERA_HOME,
            offset: glam::Vec2::new(shake_x, shake_y),
            units_per_pixel: WORLD_PER_DP,
            zoom: 1.0,
            roll: shake_roll,
            fit: repame_view2d::FitMode::Keep,
        };
        let (fit_scale, _, _) =
            repame_view2d::effective_fit(viewport_dp, self.view_world_size, &self.cam);
        self.sim
            .world
            .insert_resource(crate::sim::arena::ViewExtent {
                half_width: crate::sim::arena::WORLD_VIEW[0] * fit_scale * 0.5,
                half_height: crate::sim::arena::WORLD_VIEW[1] * fit_scale * 0.5,
            });
        (viewport_dp, density)
    }

    fn refresh_aim(&mut self, viewport_dp: [f32; 2], density: f32) {
        let (aim, touch_dp) = {
            let staging = self.staging.borrow();
            (
                staging.aim,
                staging
                    .touch_active
                    .values()
                    .next()
                    .map(|(_, position)| *position),
            )
        };
        let mut aim_cam = self.cam;
        aim_cam.offset = glam::Vec2::ZERO;
        aim_cam.roll = 0.0;
        let world_size = self.view_world_size;
        let aim_world = match touch_dp {
            Some(dp) => Some(aim_cam.dp_to_world_pt(viewport_dp, world_size, [dp.x, dp.y])),
            None => aim.live_px().map(|px| {
                repame_view2d::unproject_px(px, viewport_dp, world_size, density, &aim_cam)
            }),
        };
        self.sim.world.resource_mut::<PlayerInput>().aim_world = aim_world;
    }

    pub fn view(
        &mut self,
        sched: &mut repose_core::runtime::Scheduler,
        _ctx: &repose_core::RenderContext,
        dt: std::time::Duration,
    ) -> repose_core::View {
        repose_core::request_frame();
        repame_shell::install_game_shortcuts(
            &self.shortcut_edges,
            "floppy.pause",
            "floppy.restart",
            "floppy.confirm",
        );
        let (pause, restart, confirm) = repame_shell::take_shortcut_edges(&self.shortcut_edges);
        self.pause_edge |= pause;
        self.restart_edge |= restart;
        self.interact_edge |= confirm;
        self.feed_polled(sched);
        let (viewport_dp, density) = self.prepare_camera([sched.size.0, sched.size.1]);
        self.refresh_aim(viewport_dp, density);
        self.feed_input(sched);
        let actions = crate::ui::drain_actions(&mut self.sim.world);
        for action in actions {
            self.apply_ui_action(action);
        }
        self.advance(dt);
        self.save_now();
        let (viewport_dp, _) = self.prepare_camera([sched.size.0, sched.size.1]);
        self.advance_bake();
        let input = crate::render::assets::frame_input(
            &mut self.sim.world,
            self.assets.as_ref(),
            self.cam,
            self.view_world_size,
            viewport_dp,
        );
        let uploads = std::sync::Arc::clone(&self.uploads);
        let desc = self.batch_desc.unwrap_or(repame_view2d::BatchDesc {
            layer_size: crate::render::assets::ATLAS_SIZE,
            layers: 1,
            ..Default::default()
        });
        let staging = self.staging.clone();
        let viewport = crate::view2d::viewport(input, uploads, desc, move |ev| {
            use repame_view2d::PickEvent;
            let density = repose_core::locals::effective_density_scale().max(1e-6);
            let to_dp = |screen: [f32; 2]| glam::Vec2::new(screen[0], screen[1]) / density;
            let mut staging = staging.borrow_mut();
            match ev {
                PickEvent::Press {
                    world,
                    screen,
                    button,
                } => {
                    staging.pick_down(button);
                    if button == repose_core::input::PointerButton::Primary {
                        staging.stage_click(world, screen, density);
                    }
                    staging.stage_hover(world, screen);
                }
                PickEvent::Click { button, .. } => {
                    staging.pick_up(button);
                }
                PickEvent::Release { button, .. } => {
                    staging.pick_up(button);
                }
                PickEvent::Hover { world, screen, .. } => {
                    staging.stage_hover(world, screen);
                }
                PickEvent::TouchDown { id, screen } => {
                    staging.touch_down(id, to_dp(screen));
                }
                PickEvent::TouchMove { id, screen } => {
                    staging.touch_move(id, to_dp(screen));
                }
                PickEvent::TouchUp { id } => {
                    staging.touch_up(id);
                }
            }
        });
        let viewport = {
            use repose_ui::ViewExt as _;
            repose_ui::ZStack(repose_core::prelude::Modifier::new().fill_max_size()).child(viewport)
        };
        let mut ui_state = std::mem::take(&mut self.ui);
        crate::ui::sync_shared_ui(self, &mut ui_state, dt.as_secs_f32());
        self.ui = ui_state;
        let actions = self
            .sim
            .world
            .get_resource::<crate::ui::UiActions>()
            .cloned()
            .unwrap_or_default();
        let overlay_rc = repose_core::remember(repose_ui::overlay::OverlayHandle::new);
        let overlay = (*overlay_rc).clone();
        let root = crate::ui::compose_root(self.ui.clone(), actions.0.clone());
        let root = {
            use repose_ui::ViewExt as _;
            repose_ui::ZStack(repose_core::prelude::Modifier::new().fill_max_size())
                .child((viewport, root))
        };
        let focus = repose_core::remember(repose_core::FocusRequester::new);
        let focus_positioned = (*focus).clone();
        let focus_staging = self.staging.clone();
        let key_staging = self.staging.clone();
        let modifier = repose_core::prelude::Modifier::new()
            .fill_max_size()
            .focusable(true)
            .focus_requester((*focus).clone())
            .on_globally_positioned(move |_| focus_positioned.request_focus())
            .on_focus_changed(move |focused| {
                focus_staging.borrow_mut().set_window_focused(focused);
            })
            .on_key_event(move |event| {
                key_staging.borrow_mut().handle_key(&event);
                false
            });
        overlay.host(modifier, root)
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

fn keyboard_stick(held: &HashSet<PhysicalKey>) -> Vec2 {
    use repose_core::input::PhysicalKey as K;
    let x = (held.contains(&K::ArrowRight) as i32 - held.contains(&K::ArrowLeft) as i32) as f32;
    let y = (held.contains(&K::ArrowUp) as i32 - held.contains(&K::ArrowDown) as i32) as f32;
    Vec2::new(x, y)
}

struct AppRuntime {
    app: App,
    previous: web_time::Instant,
    #[cfg(not(target_arch = "wasm32"))]
    poller: repame_shell::GamepadPoller,
    #[cfg(not(target_arch = "wasm32"))]
    pads: repame_shell::PadBank,
    #[cfg(not(target_arch = "wasm32"))]
    pad_was_live: bool,
}

impl AppRuntime {
    fn new() -> Self {
        let mut app = App::new();
        app.load_save();
        app.offline_bones = app.grant_offline_bones();
        Self {
            app,
            previous: web_time::Instant::now(),
            #[cfg(not(target_arch = "wasm32"))]
            poller: repame_shell::GamepadPoller::new(),
            #[cfg(not(target_arch = "wasm32"))]
            pads: repame_shell::PadBank::default(),
            #[cfg(not(target_arch = "wasm32"))]
            pad_was_live: false,
        }
    }

    fn view(
        &mut self,
        sched: &mut repose_core::runtime::Scheduler,
        ctx: &repose_core::RenderContext,
    ) -> repose_core::View {
        let now = web_time::Instant::now();
        let dt = now
            .duration_since(self.previous)
            .min(web_time::Duration::from_millis(250));
        self.previous = now;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.pads.feed(self.poller.poll());
            let live = self.pads.live();
            if self.pad_was_live && !live {
                self.app.on_pad_disconnected();
            }
            self.pad_was_live = live;
            for pad in self.pads.drain() {
                self.app.staging.borrow_mut().stage_gamepad(pad);
            }
        }
        let view = self.app.view(sched, ctx, dt);
        self.app.audio.update(dt.as_secs_f32());
        view
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
pub fn run_desktop() -> anyhow::Result<()> {
    let mut runtime = AppRuntime::new();
    repame_shell::run_desktop("Floppy Warriors", (1280, 720), move |sched, ctx| {
        runtime.view(sched, ctx)
    })
}

#[cfg(target_arch = "wasm32")]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    let mut runtime = AppRuntime::new();
    repame_shell::run_web(move |sched, ctx| runtime.view(sched, ctx))
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn android_main(app: winit::platform::android::activity::AndroidApp) {
    if let Some(dir) = app.internal_data_path() {
        game_utils::set_android_data_dir(dir);
    }
    if let Err(error) = run_android(app) {
        log::error!("floppy-warriors failed: {error:?}");
    }
}

#[cfg(target_os = "android")]
fn run_android(app: winit::platform::android::activity::AndroidApp) -> anyhow::Result<()> {
    let mut runtime = AppRuntime::new();
    repame_shell::run_android(app, move |sched, ctx| runtime.view(sched, ctx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_step_runs_repame_and_gameplay_schedules() {
        let mut app = App::new();
        app.state = AppState::InGame;
        {
            let mut slowmo = app.sim.world.resource_mut::<crate::sim::rounds::SlowMo>();
            slowmo.start(0.5, 1.0);
        }

        let ran = app.advance(web_time::Duration::from_millis(51));
        let step_dt = app.sim.step.as_secs_f32();
        let step_secs = app.sim.step.as_secs_f64();
        let elapsed = app.sim.world.resource::<repame_sim::SimTime>().elapsed_secs;
        let slowmo = app
            .sim
            .world
            .resource::<crate::sim::rounds::SlowMo>()
            .remaining;

        assert_eq!(ran, 3);
        assert!((elapsed - step_secs * 3.0 * 0.5).abs() < 1e-12);
        assert!((slowmo - (1.0 - step_dt * 3.0)).abs() < 1e-6);
    }

    #[test]
    fn golden_loading_fades_and_bakes_incrementally() {
        let mut app = App::new();
        let mut sched = repose_core::runtime::Scheduler::new();
        sched.size = (1280, 720);
        sched.window_focused = true;
        let ctx = repose_core::RenderContext::new();
        let step = web_time::Duration::from_millis(16);

        let mut fade_steps = Vec::new();
        let mut progress = Vec::new();
        let mut started = false;
        for _ in 0..400 {
            let _ = app.view(&mut sched, &ctx, step);
            if app.state == AppState::Title && !started {
                app.apply_ui_action(UiAction::StartGame);
                started = true;
            }
            if app.fade > 0.0 && app.fade < 1.0 {
                fade_steps.push(app.fade);
            }
            progress.push(app.load_progress());
            if app.state == AppState::InGame && app.fade == 0.0 {
                break;
            }
        }

        assert!(
            fade_steps.len() >= 8,
            "screen changes must fade over several frames, saw {}",
            fade_steps.len()
        );
        let peak = fade_steps.iter().copied().fold(0.0f32, f32::max);
        let peak_at = fade_steps
            .iter()
            .position(|f| (*f - peak).abs() < 1e-6)
            .unwrap_or(0);
        assert!(peak >= 0.95, "fade must reach black, saw {fade_steps:?}");
        assert!(
            peak_at >= 4,
            "fade must ramp up over several frames, saw {fade_steps:?}"
        );
        assert!(
            fade_steps.len() - peak_at >= 4,
            "fade must ramp back in, saw {fade_steps:?}"
        );

        let mut partial: Vec<f32> = progress.iter().copied().filter(|p| *p > 0.0).collect();
        partial.dedup();
        assert!(
            partial.len() >= 3,
            "assets must bake a few per frame, not one blocking jump: {partial:?}"
        );
        assert_eq!(
            *progress.last().unwrap_or(&0.0),
            1.0,
            "bake must finish before the run starts"
        );
        assert!(
            progress.iter().all(|p| (0.0..=1.0).contains(p)),
            "progress must stay in 0..=1: {progress:?}"
        );
    }

    #[test]
    fn headless_frames_walk_splash_title_into_a_live_combat() {
        fn frames(app: &mut App, sched: &mut repose_core::runtime::Scheduler, count: usize) {
            let ctx = repose_core::RenderContext::new();
            let dt = web_time::Duration::from_millis(16);
            for _ in 0..count {
                let _ = app.view(sched, &ctx, dt);
            }
        }

        let mut app = App::new();
        let mut sched = repose_core::runtime::Scheduler::new();
        sched.size = (1280, 720);
        sched.window_focused = true;

        frames(&mut app, &mut sched, 120);
        assert_eq!(app.state, AppState::Title);
        let demo_warriors = app
            .sim
            .world
            .query_filtered::<Entity, With<crate::sim::title_demo::DemoSide>>()
            .iter(&app.sim.world)
            .count();
        assert_eq!(demo_warriors, 2);

        app.apply_ui_action(UiAction::StartGame);
        frames(&mut app, &mut sched, 120);
        assert_eq!(app.state, AppState::InGame);
        assert_eq!(app.sim.world.resource::<RunState>().phase, RunPhase::Combat);
        frames(&mut app, &mut sched, 90);
        assert_eq!(app.sim.world.resource::<RunState>().enemies_spawned, 1);
        assert!(app.sim.world.resource::<RunState>().active_enemy.is_some());
    }

    #[test]
    fn golden_camera_keeps_one_world_unit_per_dp() {
        let mut app = App::new();
        let (viewport_dp, _) = app.prepare_camera([1280, 720]);
        let center = app.cam.center;
        let world_size = crate::sim::arena::WORLD_VIEW;

        let canvas_center = app.cam.dp_to_world_pt(
            viewport_dp,
            world_size,
            [viewport_dp[0] * 0.5, viewport_dp[1] * 0.5],
        );
        assert!(
            (canvas_center - center).length() < 1e-3,
            "canvas centre maps to the camera centre: {canvas_center:?} vs {center:?}"
        );

        for offset in [37.0f32, 240.0, 601.5] {
            let probed = app.cam.dp_to_world_pt(
                viewport_dp,
                world_size,
                [
                    viewport_dp[0] * 0.5 + offset,
                    viewport_dp[1] * 0.5 - offset * 0.5,
                ],
            );
            let expected = center + Vec2::new(offset, -offset * 0.5);
            assert!(
                (probed - expected).length() < 1e-3,
                "{offset} dp must equal {offset} world units: {probed:?} vs {expected:?}"
            );
        }
    }

    #[test]
    fn golden_camera_holds_godots_static_framing() {
        use crate::sim::arena::{CAMERA_HOME, GROUND_TOP, WORLD_VIEW};

        let mut app = App::new();
        let (viewport_dp, _) = app.prepare_camera([1280, 720]);
        assert_eq!(
            app.cam.center, CAMERA_HOME,
            "camera stays home instead of following the player"
        );
        assert_eq!(
            app.view_world_size, WORLD_VIEW,
            "render, aim and picks share one world rect"
        );
        let extent = *app.sim.world.resource::<crate::sim::arena::ViewExtent>();
        assert_eq!(
            (extent.half_width, extent.half_height),
            (WORLD_VIEW[0] * 0.5, WORLD_VIEW[1] * 0.5),
            "published extent is the fitted world rect, {extent:?}"
        );

        let (scale, ox, oy) = repame_view2d::effective_fit(viewport_dp, WORLD_VIEW, &app.cam);
        assert!(
            (scale - 1.0).abs() < 1e-4,
            "the 1200x720 world view keeps 1:1 units, scale {scale}"
        );
        assert!(
            (ox - 40.0).abs() < 1e-4 && oy.abs() < 1e-4,
            "letterboxed like the original: {ox} {oy}"
        );

        let top = CAMERA_HOME.y - WORLD_VIEW[1] * scale * 0.5;
        let surface = (GROUND_TOP - top) / (WORLD_VIEW[1] * scale);
        assert!(
            (surface - 528.0 / 720.0).abs() < 1e-4,
            "ground line sits where Godot puts it, {surface} down the view"
        );
    }

    #[test]
    fn golden_ground_tiles_sample_the_plain_dirt_block() {
        use crate::render::assets::frame_input;
        use crate::sim::arena::{TILESET_COLS, TILESET_ROWS};

        let mut app = App::new();
        while app.assets.is_none() {
            app.advance_bake();
        }
        let assets = app.assets.as_ref().expect("assets loaded");
        let input = frame_input(
            &mut app.sim.world,
            Some(assets),
            app.cam,
            app.view_world_size,
            [1280.0, 720.0],
        );
        let uv = assets.tileset_uv().expect("tileset");
        let cell_w = (uv.max[0] - uv.min[0]) / TILESET_COLS as f32;
        let cell_h = (uv.max[1] - uv.min[1]) / TILESET_ROWS as f32;
        let tiles: Vec<_> = input
            .sprites
            .iter()
            .filter(|s| (-2.0..-0.5).contains(&s.z))
            .collect();
        assert!(!tiles.is_empty(), "ground tiles are drawn");
        for tile in &tiles {
            let col = ((tile.uv_min[0] - uv.min[0]) / cell_w).round();
            let row = ((tile.uv_min[1] - uv.min[1]) / cell_h).round();
            assert!(
                (4.0..8.0).contains(&col) && (4.0..8.0).contains(&row),
                "ground tile samples atlas col {col} row {row}, but the plain dirt \
                 block is cols 4..8 of rows 4..8 (cols 0..3 hold the outlined rock art)"
            );
        }
        let view_bottom = crate::sim::arena::CAMERA_HOME.y + app.view_world_size[1] * 0.5;
        let dirt_bottom = tiles.iter().fold(f32::MIN, |acc, t| acc.max(t.center.y))
            + crate::sim::arena::TILE_WORLD * 0.5;
        assert!(
            dirt_bottom >= view_bottom,
            "dirt must reach the bottom of the view, dirt ends at {dirt_bottom} but the \
             view ends at {view_bottom}"
        );
    }

    #[test]
    fn golden_player_stands_upright_on_the_ground() {
        use crate::sim::arena::{GROUND_TOP, STAND_Y};
        use crate::sim::comps::{WarriorLimb, WarriorRoot};
        use repame_rapier2d::BodySnapshot2d;

        fn frames(app: &mut App, sched: &mut repose_core::runtime::Scheduler, count: usize) {
            let ctx = repose_core::RenderContext::new();
            let dt = web_time::Duration::from_millis(16);
            for _ in 0..count {
                let _ = app.view(sched, &ctx, dt);
            }
        }

        let mut app = App::new();
        let mut sched = repose_core::runtime::Scheduler::new();
        sched.size = (1280, 720);
        sched.window_focused = true;
        frames(&mut app, &mut sched, 120);
        app.apply_ui_action(UiAction::StartGame);
        frames(&mut app, &mut sched, 300);

        let player = app.sim.world.resource::<RunState>().player.unwrap();
        let torso = app.sim.world.get::<WarriorRoot>(player).unwrap().torso;
        let torso_snapshot = *app.sim.world.get::<BodySnapshot2d>(torso).unwrap();
        let lowest_foot = app
            .sim
            .world
            .query::<(&WarriorLimb, &BodySnapshot2d)>()
            .iter(&app.sim.world)
            .filter(|(limb, _)| {
                limb.root == player && limb.kind == crate::sim::comps::LimbKind::LegL
            })
            .map(|(_, snap)| snap.pos.y + 17.5)
            .fold(f32::INFINITY, f32::min);

        let head_pos = app
            .sim
            .world
            .query::<(
                &crate::sim::comps::WarriorLimb,
                &repame_rapier2d::BodySnapshot2d,
            )>()
            .iter(&app.sim.world)
            .find(|(limb, _)| limb.root == player && limb.kind == crate::sim::comps::LimbKind::Head)
            .map(|(_, snap)| snap.pos)
            .unwrap();
        assert!(
            head_pos.y < torso_snapshot.pos.y - 20.0,
            "head sits above the torso: head {} vs torso {}",
            head_pos.y,
            torso_snapshot.pos.y
        );
        assert!(
            (torso_snapshot.pos.y - STAND_Y).abs() < 30.0,
            "torso settles at the stand line: {} vs {STAND_Y}",
            torso_snapshot.pos.y
        );
        assert!(
            (lowest_foot - GROUND_TOP).abs() < 20.0,
            "feet rest on the ground surface: {lowest_foot} vs {GROUND_TOP}"
        );
        assert!(
            torso_snapshot.angle.abs() < 0.7,
            "torso stays upright: {} rad",
            torso_snapshot.angle
        );
    }

    #[test]
    fn golden_bow_draw_release_shoots_and_pause_freezes_input() {
        use crate::sim::comps::{Arrow, BowState};

        fn frames(app: &mut App, sched: &mut repose_core::runtime::Scheduler, count: usize) {
            let ctx = repose_core::RenderContext::new();
            let dt = web_time::Duration::from_millis(16);
            for _ in 0..count {
                let _ = app.view(sched, &ctx, dt);
            }
        }

        fn arrow_count(app: &mut App) -> usize {
            app.sim
                .world
                .query_filtered::<Entity, With<Arrow>>()
                .iter(&app.sim.world)
                .count()
        }

        let mut app = App::new();
        let mut sched = repose_core::runtime::Scheduler::new();
        sched.size = (1280, 720);
        sched.window_focused = true;
        frames(&mut app, &mut sched, 120);
        app.apply_ui_action(UiAction::StartGame);
        frames(&mut app, &mut sched, 120);
        for _ in 0..300 {
            if app.sim.world.resource::<RunState>().active_enemy.is_some() {
                break;
            }
            frames(&mut app, &mut sched, 1);
        }
        let player = app
            .sim
            .world
            .resource::<RunState>()
            .player
            .expect("run has a player");
        let density = repose_core::locals::effective_density_scale().max(1e-6);

        app.staging
            .borrow_mut()
            .stage_hover(glam::Vec2::ZERO, [900.0 * density, 300.0 * density]);
        app.staging
            .borrow_mut()
            .pick_down(repose_core::input::PointerButton::Primary);
        frames(&mut app, &mut sched, 6);
        let bow = app.sim.world.get::<BowState>(player).unwrap();
        assert!(bow.drawing, "holding the button draws the bow");
        assert!(bow.draw_power > 0.0, "draw power accumulates");

        app.toggle_pause();
        app.staging
            .borrow_mut()
            .pick_up(repose_core::input::PointerButton::Primary);
        frames(&mut app, &mut sched, 4);
        assert_eq!(arrow_count(&mut app), 0, "a paused release must not shoot");
        assert!(!app.sim.world.get::<BowState>(player).unwrap().drawing);

        let elapsed = app.sim.world.resource::<repame_sim::SimTime>().elapsed_secs;
        frames(&mut app, &mut sched, 4);
        assert_eq!(
            app.sim.world.resource::<repame_sim::SimTime>().elapsed_secs,
            elapsed,
            "pause freezes the sim clock"
        );

        app.apply_ui_action(UiAction::Resume);
        frames(&mut app, &mut sched, 20);
        assert!(!app.paused);

        app.staging
            .borrow_mut()
            .pick_down(repose_core::input::PointerButton::Primary);
        frames(&mut app, &mut sched, 6);
        app.staging
            .borrow_mut()
            .pick_up(repose_core::input::PointerButton::Primary);
        frames(&mut app, &mut sched, 1);
        assert!(arrow_count(&mut app) > 0, "releasing the button shoots");
        assert!(!app.sim.world.get::<BowState>(player).unwrap().drawing);
    }
}
