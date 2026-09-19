use std::collections::HashSet;

use bevy_ecs::prelude::*;
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

#[derive(Resource, Default)]
pub struct Paused(pub bool);

#[derive(Resource, Default)]
pub struct PendingUnpause(pub f32);

pub use crate::ui::UiAction;

pub struct App {
    pub sim: repame_sim::Sim,
    schedule: Schedule,
    pub state: AppState,
    pub overlay: OverlayMenu,
    pub paused: bool,
    pending_unpause: f32,
    splash_timer: f32,
    loading_timer: f32,
    assets: Option<crate::render::assets::RenderAssets>,
    uploads: Vec<repame_sprite::AtlasUpload>,
    batch_desc: Option<repame_sprite::BatchDesc>,
    pub audio: repame_audio::Audio,
    save_path: Option<std::path::PathBuf>,
    save_dirty: bool,
    staging: std::rc::Rc<std::cell::RefCell<repame_shell::Staging>>,
    shortcut_edges: repame_shell::SharedEdges,
    pause_edge: bool,
    restart_edge: bool,
    interact_edge: bool,
    held: HashSet<PhysicalKey>,
    edges: Vec<PhysicalKey>,
    window_focused: bool,
    lmb_held: bool,
    rmb_held: bool,
    lmb_released: bool,
    space_was_held: bool,
    cam: repame_sprite::Camera2d,
    geom: repame_sprite::GeomHandle,
    view_width: f32,
    view_viewport_dp: [f32; 2],
    view_density: f32,
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
        repame_rapier2d::init_world(&mut sim.world, glam::Vec2::new(0.0, -980.0), 20.0);
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
            assets: None,
            uploads: Vec::new(),
            batch_desc: None,
            audio: repame_audio::Audio::noop(),
            save_path: None,
            save_dirty: false,
            staging,
            shortcut_edges: repame_shell::shared_edges(),
            pause_edge: false,
            restart_edge: false,
            interact_edge: false,
            held: HashSet::new(),
            edges: Vec::new(),
            window_focused: true,
            lmb_held: false,
            rmb_held: false,
            lmb_released: false,
            space_was_held: false,
            cam: repame_sprite::Camera2d::default(),
            geom: repame_sprite::GeomHandle::new(),
            view_width: 1280.0,
            view_viewport_dp: [1280.0, 720.0],
            view_density: 1.0,
        }
    }

    pub fn load_save(&mut self) {
        let path = repame_shell::save_file_path("FLOPPY_SAVE", "floppy-warriors/save.ron");
        if let Ok(text) = repame_shell::load_json(&path) {
            if let Ok(mut save) = ron::from_str::<crate::sim::save::SaveData>(&text) {
                save.migrate();
                self.apply_locale(&save.settings.language);
                self.sim.world.insert_resource(save);
            }
        }
        self.sim
            .world
            .resource_mut::<crate::sim::save::SaveData>()
            .migrate();
        self.save_path = Some(path);
    }

    fn save_now(&mut self) {
        if !self.save_dirty {
            return;
        }
        self.save_dirty = false;
        let Some(path) = self.save_path.clone() else {
            return;
        };
        let mut save = self
            .sim
            .world
            .resource::<crate::sim::save::SaveData>()
            .clone();
        save.version = crate::sim::save::SAVE_VERSION;
        if let Ok(text) = ron::ser::to_string(&save) {
            let _ = repame_shell::store_json(&text, &path);
        }
    }

    pub fn mark_save_dirty(&mut self) {
        self.save_dirty = true;
    }

    fn apply_locale(&mut self, _lang: &str) {}

    fn ensure_assets(&mut self) {
        if self.assets.is_some() {
            return;
        }
        let dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let assets_dir = dir.join("assets");
        let mut assets = crate::render::assets::bake_svg_assets(&assets_dir);
        self.batch_desc = Some(assets.batch_desc());
        self.uploads = assets.take_uploads();
        self.assets = Some(assets);
    }

    fn grant_offline_bones(&mut self) -> u32 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
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
        self.state = AppState::Loading;
        self.loading_timer = 0.5;
        self.ensure_assets();
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
        let limbs: Vec<Entity> = self
            .sim
            .world
            .query::<(Entity, &crate::sim::comps::WarriorLimb)>()
            .iter(&self.sim.world)
            .map(|(e, _)| e)
            .collect();
        for e in limbs {
            self.sim.world.despawn(e);
        }
        let arrows: Vec<Entity> = self
            .sim
            .world
            .query::<(Entity, &crate::sim::comps::Arrow)>()
            .iter(&self.sim.world)
            .map(|(e, _)| e)
            .collect();
        for e in arrows {
            self.sim.world.despawn(e);
        }
        let roots: Vec<Entity> = self
            .sim
            .world
            .query::<(Entity, &crate::sim::comps::WarriorRoot)>()
            .iter(&self.sim.world)
            .map(|(e, _)| e)
            .collect();
        for e in roots {
            self.sim.world.despawn(e);
        }
        let joints: Vec<Entity> = self
            .sim
            .world
            .query::<(Entity, &repame_rapier2d::RapierRevolute)>()
            .iter(&self.sim.world)
            .map(|(e, _)| e)
            .collect();
        for e in joints {
            self.sim.world.despawn(e);
        }
        self.sim.world.insert_resource(RunState::default());
        self.sim
            .world
            .insert_resource(crate::sim::rounds::HitEvents::default());
    }

    fn feed_polled(&mut self, sched: &repose_core::runtime::Scheduler) {
        if !sched.window_focused {
            self.held.clear();
            self.lmb_held = false;
            self.rmb_held = false;
            self.window_focused = false;
            self.edges.clear();
        } else {
            self.window_focused = true;
            repame_input::reconcile_held(&mut self.held, &sched.held_keys, |k| Some(k), |_| None);
            if !sched.mouse_primary {
                self.lmb_held = false;
            }
            if !sched.mouse_secondary {
                self.rmb_held = false;
            }
        }
        let mut staging = self.staging.borrow_mut();
        staging.feed_polled(sched);
    }

    fn feed_input(&mut self, sched: &repose_core::runtime::Scheduler) {
        let staging = self.staging.clone();
        let mut staging = staging.borrow_mut();
        let staging_edges = std::mem::take(&mut staging.edges);
        let staging_clicks = staging.take_clicks();
        let staging_lmb = staging.lmb_held;
        let staging_rmb_down = staging.take_rmb_down();
        drop(staging);
        for code in staging_edges {
            if self.held.insert(code) {
                self.edges.push(code);
            }
        }
        let just: HashSet<PhysicalKey> = self.edges.drain(..).collect();
        let in_game_combat = self.state == AppState::InGame
            && !self.paused
            && self.overlay == OverlayMenu::None
            && self.sim.world.resource::<RunState>().phase == RunPhase::Combat;
        let rmb_down = staging_rmb_down;
        let lmb_was_held = self.lmb_held;
        if self.state == AppState::InGame {
            for key in &just {
                if *key == repose_core::input::PhysicalKey::Escape {
                    self.pause_edge = true;
                }
                if *key == repose_core::input::PhysicalKey::KeyR
                    && self.sim.world.resource::<RunState>().phase == RunPhase::GameOver
                {
                    self.sim
                        .world
                        .resource_mut::<crate::sim::input::RestartRequest>()
                        .0 = true;
                }
            }
            if self.pause_edge {
                self.pause_edge = false;
                self.toggle_pause();
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
                self.begin_run();
                return;
            }
        }
        let mouse_down_edge = !staging_clicks.is_empty();
        let lmb_down = staging_lmb || sched.mouse_primary;
        if !lmb_down {
            self.lmb_released = lmb_was_held && in_game_combat;
        }
        let lmb_released = std::mem::take(&mut self.lmb_released);
        let space_currently_held = sched
            .held_keys
            .contains(&repose_core::input::PhysicalKey::Space);
        let space_edge = just.contains(&repose_core::input::PhysicalKey::Space);
        let space_released = self.space_was_held && !space_currently_held && in_game_combat;
        self.space_was_held = space_currently_held;
        let aim = self.staging.borrow().aim;
        let aim_world = aim.baked();
        let stick = if in_game_combat {
            keyboard_stick(&self.held)
        } else {
            Vec2::ZERO
        };
        let draw_pressed_edge = space_edge;
        self.sim.world.insert_resource(PlayerInput {
            aim_world,
            stick,
            draw_pressed: (mouse_down_edge || draw_pressed_edge) && in_game_combat,
            draw_released: (lmb_released || space_released) && in_game_combat,
            dodge_pressed: (just.contains(&repose_core::input::PhysicalKey::ShiftLeft)
                || just.contains(&repose_core::input::PhysicalKey::ShiftRight))
                && in_game_combat,
            lmb_held: lmb_down && in_game_combat,
        });
        self.lmb_held = lmb_down;
        let _ = rmb_down;
        if self.interact_edge {
            self.interact_edge = false;
        }
        if self.restart_edge {
            self.restart_edge = false;
        }
    }

    fn toggle_pause(&mut self) {
        if self.overlay == OverlayMenu::Pause {
            self.overlay = OverlayMenu::None;
            self.pending_unpause = 0.2;
        } else if self.paused {
            self.overlay = OverlayMenu::None;
            self.pending_unpause = 0.2;
        } else {
            self.paused = true;
            self.overlay = OverlayMenu::Pause;
        }
    }

    fn advance(&mut self, dt: std::time::Duration) {
        if self.state == AppState::Splash {
            self.splash_timer -= dt.as_secs_f32();
            if self.splash_timer <= 0.0 {
                self.state = AppState::Title;
            }
            return;
        }
        if self.state == AppState::Loading {
            self.loading_timer -= dt.as_secs_f32();
            if self.loading_timer <= 0.0 {
                self.begin_run();
            }
            return;
        }
        if self.pending_unpause > 0.0 {
            self.pending_unpause -= dt.as_secs_f32();
            if self.pending_unpause <= 0.0 {
                self.pending_unpause = 0.0;
                self.paused = false;
            }
        }
        if self.paused || self.overlay == OverlayMenu::Pause {
            return;
        }
        let step_dt = self.sim.step.as_secs_f32();
        self.sim
            .world
            .resource_mut::<crate::sim::state::SimStep>()
            .dt = step_dt;
        self.sim.tick();
        if self.state == AppState::Title {
            crate::sim::title_demo::demo_ai(&mut self.sim.world);
            crate::sim::title_demo::update_demo_arrows(&mut self.sim.world);
            crate::sim::title_demo::demo_respawn(&mut self.sim.world);
            crate::sim::warrior::tick_motor_state_by_world(&mut self.sim.world);
            crate::sim::fx::step_fx(&mut self.sim.world, step_dt);
            let cues = crate::sim::audio::drain_cues(&mut self.sim.world);
            for cue in cues {
                self.audio.play(cue.name);
            }
            return;
        }
        if self.state != AppState::InGame {
            return;
        }
        self.schedule.run(&mut self.sim.world);
        let cues = crate::sim::audio::drain_cues(&mut self.sim.world);
        for cue in cues {
            self.audio.play(cue.name);
        }
        if self.sim.world.resource::<RunState>().phase == RunPhase::GameOver {
            self.mark_save_dirty();
        }
        self.save_now();
    }

    fn apply_ui_action(&mut self, action: UiAction) {
        match action {
            UiAction::StartGame => self.goto_loading(),
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
                self.begin_run();
            }
            UiAction::QuitApp => std::process::exit(0),
            UiAction::QuitToTitle => {
                self.paused = false;
                self.overlay = OverlayMenu::None;
                self.pending_unpause = 0.0;
                self.goto_title();
            }
            UiAction::SetMasterVol(v) => {
                self.sim
                    .world
                    .resource_mut::<crate::sim::audio::AudioLevels>()
                    .master = v.clamp(0.0, 1.0);
                self.audio
                    .set_channel(repame_audio::AudioChannel::Master, v.clamp(0.0, 1.0));
            }
            UiAction::SetSfxVol(v) => {
                self.sim
                    .world
                    .resource_mut::<crate::sim::audio::AudioLevels>()
                    .sfx = v.clamp(0.0, 1.0);
                self.audio
                    .set_channel(repame_audio::AudioChannel::Sfx, v.clamp(0.0, 1.0));
            }
            UiAction::SetMusicVol(v) => {
                self.sim
                    .world
                    .resource_mut::<crate::sim::audio::AudioLevels>()
                    .music = v.clamp(0.0, 1.0);
                self.audio
                    .set_channel(repame_audio::AudioChannel::Music, v.clamp(0.0, 1.0));
            }
            UiAction::SaveSettings => {
                let levels = *self.sim.world.resource::<crate::sim::audio::AudioLevels>();
                {
                    let mut save = self.sim.world.resource_mut::<crate::sim::save::SaveData>();
                    save.settings.master_volume = levels.master;
                    save.settings.sfx_volume = levels.sfx;
                    save.settings.music_volume = levels.music;
                    save.last_played_unix = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
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

    pub fn view(
        &mut self,
        sched: &mut repose_core::runtime::Scheduler,
        _ctx: &repose_core::RenderContext,
        dt: std::time::Duration,
    ) -> repose_core::View {
        repose_core::request_frame();
        if sched.size.0 > 0 {
            self.view_width = sched.size.0 as f32;
        }
        self.feed_polled(sched);
        self.feed_input(sched);
        let actions = crate::ui::drain_actions(&mut self.sim.world);
        for action in actions {
            self.apply_ui_action(action);
        }
        self.advance(dt);
        self.save_now();
        let (w, h) = sched.size;
        let viewport_px = if w == 0 || h == 0 {
            [1280.0, 720.0]
        } else {
            [w as f32, h as f32]
        };
        let density = repose_core::locals::effective_density_scale().max(1e-6);
        let viewport_dp = [viewport_px[0] / density, viewport_px[1] / density];
        self.view_viewport_dp = viewport_dp;
        self.view_density = density;
        self.ensure_assets();
        let world_size = [2400.0, 1440.0];
        let center = self.camera_target();
        self.cam = repame_sprite::Camera2d {
            center,
            offset: glam::Vec2::ZERO,
            units_per_pixel: 1.0,
            zoom: 1.0,
            roll: 0.0,
        };
        let assets = self.assets.as_ref().expect("assets ensured");
        let input = crate::render::assets::frame_input(
            &mut self.sim.world,
            assets,
            self.cam,
            world_size,
            viewport_dp,
        );
        let uploads = std::mem::take(&mut self.uploads);
        let desc = self.batch_desc.clone().unwrap_or_default();
        let geom = self.geom.clone();
        let staging = self.staging.clone();
        let viewport = repame_sprite::Viewport2dGpuWithHud(input, geom, uploads, desc, move |ev| {
            use repame_sprite::PickEvent;
            let mut staging = staging.borrow_mut();
            match ev {
                PickEvent::Press {
                    world,
                    screen,
                    button,
                } => {
                    staging.pick_down(button);
                    if button == repose_core::input::PointerButton::Primary {
                        let d = repose_core::locals::effective_density_scale().max(1e-6);
                        staging.stage_click(world, screen, d);
                    }
                    staging.stage_hover(world, screen);
                }
                PickEvent::Click { button, .. } => {
                    staging.pick_up(button);
                }
                PickEvent::Hover { world, screen, .. } => {
                    staging.stage_hover(world, screen);
                }
                PickEvent::TouchDown { id, screen } => {
                    staging.touch_down(id, glam::Vec2::new(screen[0], screen[1]));
                }
                PickEvent::TouchMove { id, screen } => {
                    staging.touch_move(id, glam::Vec2::new(screen[0], screen[1]));
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
        let mut ui_state = crate::ui::SharedUi::default();
        crate::ui::sync_shared_ui(self, &mut ui_state);
        let actions = self
            .sim
            .world
            .get_resource::<crate::ui::UiActions>()
            .cloned()
            .unwrap_or_default();
        let overlay_rc = repose_core::remember(repose_ui::overlay::OverlayHandle::new);
        let overlay = (*overlay_rc).clone();
        let root = crate::ui::compose_root(overlay.clone(), ui_state, actions.0.clone());
        let root = {
            use repose_ui::ViewExt as _;
            repose_ui::ZStack(repose_core::prelude::Modifier::new().fill_max_size())
                .child((viewport, root))
        };
        overlay.host(repose_core::prelude::Modifier::new().fill_max_size(), root)
    }

    fn camera_target(&mut self) -> glam::Vec2 {
        if self.state != AppState::InGame {
            return glam::Vec2::new(0.0, 40.0);
        }
        let player = self.sim.world.resource::<RunState>().player;
        if let Some(p) = player
            && let Some(w) = self.sim.world.get::<crate::sim::comps::WarriorRoot>(p)
            && let Some(snap) = self
                .sim
                .world
                .get::<repame_rapier2d::BodySnapshot2d>(w.torso)
        {
            return snap.pos;
        }
        glam::Vec2::new(0.0, 40.0)
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

pub fn run_desktop() -> anyhow::Result<()> {
    let mut app = App::new();
    app.load_save();
    let offline = app.grant_offline_bones();
    let _ = offline;
    let mut poller = repame_shell::GamepadPoller::new();
    let mut bank = repame_shell::PadBank::default();
    repame_shell::run_desktop("Floppy Warriors", (1280, 720), move |sched, ctx| {
        bank.feed(poller.poll());
        for pad in bank.drain() {
            app.staging.borrow_mut().stage_gamepad(pad);
        }
        let view = app.view(sched, ctx, std::time::Duration::from_secs_f32(1.0 / 60.0));
        app.audio.update(1.0 / 60.0);
        view
    })
}
