use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use std::rc::Rc;

use repose_core::View;
use repose_core::prelude::{
    AlignItems, AlignSelf, AnimationSpec, Color as RColor, Easing, JustifyContent, Modifier,
    remember,
};
use repose_material::material3::{
    ButtonConfig, DropdownMenu, DropdownMenuConfig, DropdownMenuEntry, DropdownMenuItem,
    FilledTonalButton, MenuState,
};
use repose_ui::anim_ext::{
    AnimatedVisibility, AnimatedVisibilityConfig, EnterTransition, ExitTransition,
};
use repose_ui::overlay::OverlayHandle;
use repose_ui::{Column, FlowRow, Row, Spacer, Text as RText, TextStyle, ViewExt, ZStack};

use crate::app::{AppState, OverlayMenu, RewardCardUi, SharedUi, rarity_name};

fn t(translations: &HashMap<String, String>, key: &str, fallback: &str) -> String {
    translations
        .get(key)
        .cloned()
        .unwrap_or_else(|| fallback.to_string())
}

#[derive(Clone, Debug)]
pub enum UiAction {
    StartGame,
    OpenSettings,
    OpenCredits,
    CloseOverlay,
    Resume,
    QuitToTitle,
    QuitApp,
    SetMasterVol(f32),
    SetSfxVol(f32),
    SetMusicVol(f32),
    SaveSettings,
    SetLanguage(String),
    OpenBoneShop,
    CloseBoneShop,
    BuyMeta(String),
    ChooseReward(usize),
}

#[derive(bevy::prelude::Resource, Clone)]
pub struct UiBridge {
    pub shared: Arc<Mutex<SharedUi>>,
    pub actions: Arc<Mutex<Vec<UiAction>>>,
}

fn spacer(h: f32) -> View {
    Column(Modifier::new().height(h).width(1.0))
}

fn popup_anim_config(key: &str) -> AnimatedVisibilityConfig {
    AnimatedVisibilityConfig {
        key: key.into(),
        spec: AnimationSpec::tween(Duration::from_millis(200), Easing::EaseOut),
        enter: EnterTransition::ScaleIn { initial: 0.95 },
        exit: ExitTransition::ScaleOut { target: 0.95 },
    }
}

fn reward_anim_config() -> AnimatedVisibilityConfig {
    AnimatedVisibilityConfig {
        key: "reward_overlay".into(),
        spec: AnimationSpec::tween(Duration::from_millis(220), Easing::EaseOut),
        enter: EnterTransition::FadeIn.and(EnterTransition::SlideIn {
            offset_x: 0.0,
            offset_y: 24.0,
        }),
        exit: ExitTransition::FadeOut.and(ExitTransition::SlideOut {
            offset_x: 0.0,
            offset_y: 16.0,
        }),
    }
}

pub fn compose_root(
    overlay: OverlayHandle,
    st: SharedUi,
    actions: Arc<Mutex<Vec<UiAction>>>,
) -> View {
    let root = ZStack(Modifier::new().fill_max_size());
    let settings_view = settings_ui(overlay, &st, actions.clone());

    let content = match st.phase {
        AppState::Splash => splash_ui(),
        AppState::Loading => loading_ui(&st),
        AppState::Title => ZStack(Modifier::new().fill_max_size()).child((
            title_ui(&st, actions.clone()),
            AnimatedVisibility(
                st.overlay == OverlayMenu::Settings,
                settings_view.clone(),
                popup_anim_config("title_settings"),
            ),
            AnimatedVisibility(
                st.overlay == OverlayMenu::Credits,
                credits_ui(&st, actions.clone()),
                popup_anim_config("title_credits"),
            ),
            AnimatedVisibility(
                st.overlay == OverlayMenu::BoneShop,
                bone_shop_ui(&st, actions.clone()),
                popup_anim_config("title_boneshop"),
            ),
        )),
        AppState::InGame => {
            let hud = ingame_hud(&st);
            let reward = if st.run_phase == 1 {
                reward_ui(&st, actions.clone())
            } else {
                Column(Modifier::new().width(0.0).height(0.0))
            };
            let game_over = if st.run_phase == 2 {
                game_over_ui(&st, actions.clone())
            } else {
                Column(Modifier::new().width(0.0).height(0.0))
            };
            ZStack(Modifier::new().fill_max_size()).child((
                hud,
                boss_banner_ui(&st),
                AnimatedVisibility(st.run_phase == 1, reward, reward_anim_config()),
                AnimatedVisibility(st.run_phase == 2, game_over, popup_anim_config("game_over")),
                AnimatedVisibility(
                    st.overlay == OverlayMenu::Pause,
                    pause_overlay(&st, actions.clone()),
                    popup_anim_config("pause"),
                ),
                AnimatedVisibility(
                    st.overlay == OverlayMenu::Settings,
                    settings_view.clone(),
                    popup_anim_config("ingame_settings"),
                ),
                AnimatedVisibility(
                    st.overlay == OverlayMenu::Credits,
                    credits_ui(&st, actions.clone()),
                    popup_anim_config("ingame_credits"),
                ),
            ))
        }
    };

    if st.transition_alpha > 0.001 || st.flash_alpha > 0.001 {
        let fade_a = (st.transition_alpha.clamp(0.0, 1.0) * 255.0) as u8;
        let flash_a = (st.flash_alpha.clamp(0.0, 1.0) * 255.0) as u8;
        root.child((
            content,
            Column(
                Modifier::new()
                    .fill_max_size()
                    .background(RColor::from_rgba(0, 0, 0, fade_a)),
            ),
            Column(
                Modifier::new()
                    .fill_max_size()
                    .background(RColor::from_rgba(flash_a, flash_a, flash_a, flash_a)),
            ),
        ))
    } else {
        root.child(content)
    }
}

fn splash_ui() -> View {
    Column(
        Modifier::new()
            .fill_max_size()
            .justify_content(JustifyContent::CENTER)
            .align_items(AlignItems::CENTER)
            .background(col(8, 8, 12)),
    )
    .child(RText("Floppy Warriors").size(48.0).color(RColor::WHITE))
}

fn loading_ui(st: &SharedUi) -> View {
    let pct = st.loading_progress.clamp(0.0, 1.0);
    Column(
        Modifier::new()
            .fill_max_size()
            .justify_content(JustifyContent::CENTER)
            .align_items(AlignItems::CENTER)
            .background(col(8, 8, 12)),
    )
    .child(
        RText(t(&st.translations, "loading", "Loading..."))
            .size(32.0)
            .color(RColor::WHITE),
    )
    .child(spacer(16.0))
    .child(
        RText(format!("{:.0}%", pct * 100.0))
            .size(18.0)
            .color(RColor::WHITE),
    )
    .child(spacer(12.0))
    .child(
        Column(
            Modifier::new()
                .width(320.0)
                .height(12.0)
                .background(col(30, 30, 38))
                .clip_rounded(6.0),
        )
        .child(Column(
            Modifier::new()
                .width((320.0 * pct).max(1.0))
                .height(12.0)
                .background(col(96, 165, 250))
                .clip_rounded(6.0)
                .align_self(AlignSelf::FLEX_START),
        )),
    )
}

fn title_ui(st: &SharedUi, actions: Arc<Mutex<Vec<UiAction>>>) -> View {
    let a1 = actions.clone();
    let a2 = actions.clone();
    let a3 = actions.clone();
    let a4 = actions.clone();
    let a5 = actions.clone();
    let tr = &st.translations;

    let mut children: Vec<View> = vec![
        RText(t(tr, "app-title", "Floppy Warriors"))
            .size(56.0)
            .color(RColor::WHITE),
        spacer(12.0),
        RText(format!(
            "{}: {}   {}: {}",
            t(tr, "bones", "Bones"),
            st.bones,
            t(tr, "best-round", "Best Round"),
            st.best_round,
        ))
        .size(18.0)
        .color(col(200, 200, 200)),
    ];

    if st.offline_bones > 0 {
        children.push(
            RText(format!(
                "{}  (+{})",
                t(tr, "offline-bones", "While you were away"),
                st.offline_bones,
            ))
            .size(15.0)
            .color(col(230, 200, 120)),
        );
    }

    children.push(spacer(24.0));
    children.push(mk_button(
        &t(tr, "start-game", "Play!"),
        col(60, 120, 200),
        move || push(&a1, UiAction::StartGame),
    ));
    children.push(mk_button(
        &t(tr, "bone-shop", "Bone Shop"),
        col(150, 130, 60),
        move || push(&a2, UiAction::OpenBoneShop),
    ));
    children.push(mk_button(
        &t(tr, "settings", "Settings"),
        col(70, 70, 90),
        move || push(&a3, UiAction::OpenSettings),
    ));
    children.push(mk_button(
        &t(tr, "credits", "Credits"),
        col(70, 70, 90),
        move || push(&a4, UiAction::OpenCredits),
    ));
    children.push(mk_button(
        &t(tr, "quit", "Quit"),
        col(180, 60, 60),
        move || push(&a5, UiAction::QuitApp),
    ));

    Column(
        Modifier::new()
            .fill_max_size()
            .justify_content(JustifyContent::CENTER)
            .align_items(AlignItems::CENTER)
            .background(col(8, 8, 12)),
    )
    .child(children)
}

fn bone_shop_ui(st: &SharedUi, actions: Arc<Mutex<Vec<UiAction>>>) -> View {
    let a_close = actions.clone();
    let tr = &st.translations;
    let bones = st.bones;

    let rows: Vec<View> = st
        .bone_shop_items
        .iter()
        .map(|item| {
            let a_buy = actions.clone();
            let id = item.id.clone();
            let afford = bones >= item.cost;
            let cost_label = if item.maxed {
                "MAX".to_string()
            } else {
                format!("{}: {}", t(tr, "bones", "Bones"), item.cost)
            };
            let label = format!("{}  ({} {})", cost_label, t(tr, "level", "Lv"), item.level);
            Row(Modifier::new().gap(12.0).align_items(AlignItems::CENTER)).child((
                Column(Modifier::new().width(300.0)).child((
                    RText(&item.name).size(18.0).color(RColor::WHITE),
                    RText(&item.description)
                        .size(13.0)
                        .color(col(190, 190, 200)),
                )),
                RText(label).size(15.0).color(if afford && !item.maxed {
                    col(120, 220, 140)
                } else {
                    col(140, 140, 150)
                }),
                FilledTonalButton(
                    Modifier::new().width(110.0).height(40.0),
                    move || push(&a_buy, UiAction::BuyMeta(id.clone())),
                    ButtonConfig::default(),
                    move || RText(if item.maxed { "MAX" } else { "Buy" }).size(16.0),
                ),
            ))
        })
        .collect();

    let inner = Column(
        Modifier::new()
            .width(620.0)
            .padding(24.0)
            .background(col(20, 20, 28))
            .clip_rounded(12.0)
            .align_items(AlignItems::CENTER),
    )
    .child([
        RText(t(tr, "bone-shop", "Bone Shop"))
            .size(36.0)
            .color(RColor::WHITE),
        spacer(6.0),
        RText(format!("{}: {}", t(tr, "bones", "Bones"), bones))
            .size(18.0)
            .color(col(230, 200, 120)),
        spacer(12.0),
        RText(format!(
            "{}: {} | {}: {} | {}: {}",
            t(tr, "best-round", "Best Round"),
            st.best_round,
            t(tr, "runs", "Runs"),
            st.total_runs,
            t(tr, "wins", "Wins"),
            st.total_victories,
        ))
        .size(14.0)
        .color(col(180, 180, 190)),
        spacer(16.0),
        Column(Modifier::new().gap(8.0).align_items(AlignItems::FLEX_START)).child(rows),
        spacer(20.0),
        mk_button(&t(tr, "back", "Back"), col(70, 70, 90), move || {
            push(&a_close, UiAction::CloseBoneShop)
        }),
    ]);

    Column(
        Modifier::new()
            .fill_max_size()
            .justify_content(JustifyContent::CENTER)
            .align_items(AlignItems::CENTER)
            .background(RColor::from_rgba(0, 0, 0, 180)),
    )
    .child(inner)
}

fn pause_overlay(st: &SharedUi, actions: Arc<Mutex<Vec<UiAction>>>) -> View {
    let a1 = actions.clone();
    let a2 = actions.clone();
    let a3 = actions.clone();
    let tr = &st.translations;

    Column(
        Modifier::new()
            .fill_max_size()
            .justify_content(JustifyContent::CENTER)
            .align_items(AlignItems::CENTER)
            .background(RColor::from_rgba(0, 0, 0, 180)),
    )
    .child(pause_panel(tr, a1, a2, a3))
}

fn pause_panel(
    tr: &HashMap<String, String>,
    a1: Arc<Mutex<Vec<UiAction>>>,
    a2: Arc<Mutex<Vec<UiAction>>>,
    a3: Arc<Mutex<Vec<UiAction>>>,
) -> View {
    Column(
        Modifier::new()
            .width(320.0)
            .padding(24.0)
            .background(col(20, 20, 28))
            .clip_rounded(12.0)
            .align_items(AlignItems::CENTER),
    )
    .child((
        RText(t(tr, "paused", "Paused"))
            .size(36.0)
            .color(RColor::WHITE),
        spacer(16.0),
        mk_button(&t(tr, "resume", "Resume"), col(60, 140, 90), move || {
            push(&a1, UiAction::Resume)
        }),
        mk_button(&t(tr, "settings", "Settings"), col(70, 70, 90), move || {
            push(&a2, UiAction::OpenSettings)
        }),
        mk_button(
            &t(tr, "quit-to-title", "Quit to Title"),
            col(180, 60, 60),
            move || push(&a3, UiAction::QuitToTitle),
        ),
    ))
}

fn reward_ui(st: &SharedUi, actions: Arc<Mutex<Vec<UiAction>>>) -> View {
    let tr = &st.translations;

    let cards: Vec<View> = if st.reward_cards.is_empty() {
        vec![RText("No upgrades left — free heal applied.")
            .size(16.0)
            .color(col(200, 200, 210))]
    } else {
        st.reward_cards
            .iter()
            .enumerate()
            .map(|(i, card)| reward_card(card.clone(), i, actions.clone()))
            .collect()
    };

    // Full-screen overlay — NO graphics_layer / shadow / repaint_boundary
    ZStack(
        Modifier::new()
            .fill_max_size()
            .render_z_index(900.0)
            .input_blocker(),
    )
    .child((
        // Dark readable backdrop
        Column(
            Modifier::new()
                .fill_max_size()
                .background(RColor::from_rgba(0, 0, 0, 178)),
        ),
        // Center panel
        Column(
            Modifier::new()
                .fill_max_size()
                .justify_content(JustifyContent::CENTER)
                .align_items(AlignItems::CENTER)
                .padding(24.0),
        )
        .child(
            Column(
                Modifier::new()
                    .key(key_of(&format!("reward-panel-round-{}", st.run_round)))
                    .width(930.0)
                    .max_width(960.0)
                    .padding(24.0)
                    .background(RColor::from_rgba(15, 17, 24, 245))
                    .border(1.0, RColor::from_rgba(255, 210, 120, 70), 22.0)
                    .clip_rounded(22.0) // OK on surface
                    .align_items(AlignItems::CENTER)
                    .gap(18.0),
            )
            .child((
                reward_header(st, t(tr, "choose-reward", "Choose a Reward")),
                FlowRow(
                    Modifier::new()
                        .key(key_of(&format!("reward-card-row-{}", st.run_round)))
                        .gap(16.0)
                        .align_items(AlignItems::CENTER)
                        .justify_content(JustifyContent::CENTER),
                )
                .child(cards),
                reward_footer_hint(),
            )),
        ),
    ))
}

fn reward_header(st: &SharedUi, title: String) -> View {
    Column(
        Modifier::new()
            .fill_max_width()
            .align_items(AlignItems::CENTER)
            .gap(6.0),
    )
    .child((
        RText(title).size(34.0).color(col(255, 218, 132)),
        Row(Modifier::new().gap(8.0).align_items(AlignItems::CENTER)).child((
            reward_chip(
                format!("ROUND {}", st.run_round),
                RColor::from_rgba(255, 210, 120, 34),
                col(255, 220, 150),
            ),
            reward_chip(
                format!("SCORE {}", st.run_score),
                RColor::from_rgba(120, 170, 255, 30),
                col(150, 190, 255),
            ),
        )),
    ))
}

fn reward_footer_hint() -> View {
    RText("Pick one upgrade. Stacks have caps, so rare choices matter the most.")
        .size(13.0)
        .color(col(150, 155, 168))
}

fn reward_card(card: RewardCardUi, index: usize, actions: Arc<Mutex<Vec<UiAction>>>) -> View {
    let accent = rgb(card.accent_rgb);
    let soft_accent = rgb_alpha(card.accent_rgb, 34);
    let dim_accent = rgb_alpha(card.accent_rgb, 90);
    let dark = RColor::from_rgba(24, 27, 36, 255);

    let rarity = rarity_name(card.rarity).to_uppercase();
    let stack_text = stack_label(card.current_stacks, card.max_stacks);
    let id_text = format!("#{}", card.id);
    let category_text = card.category.to_uppercase();
    let glyph = reward_glyph(&card);
    let title = card.title.clone();
    let description = card.description.clone();
    let choose_actions = actions.clone();

    // Card: clip_rounded OK — NO graphics_layer / shadow / repaint_boundary
    Column(
        Modifier::new()
            .key(key_of(&format!("reward-card:{}:{}", card.id, index)))
            .width(270.0)
            .height(336.0)
            .background(dark)
            .border(1.0, dim_accent, 18.0)
            .clip_rounded(18.0)
            .clickable()
            .on_pointer_up(move |_| push(&choose_actions, UiAction::ChooseReward(index))),
    )
    .child((
        // Accent top rail
        Column(
            Modifier::new()
                .fill_max_width()
                .height(8.0)
                .background(accent),
        ),
        Column(Modifier::new().fill_max_size().padding(16.0).gap(12.0)).child((
            // Top metadata row
            Row(Modifier::new()
                .fill_max_width()
                .align_items(AlignItems::CENTER)
                .gap(8.0))
            .child((
                reward_chip(rarity, soft_accent, accent),
                reward_chip(
                    category_text,
                    RColor::from_rgba(255, 255, 255, 18),
                    col(190, 195, 210),
                ),
                Spacer(),
                RText(id_text)
                    .size(10.0)
                    .color(col(92, 98, 112))
                    .single_line(),
            )),
            // Rarity glyph badge
            Column(
                Modifier::new()
                    .align_self(AlignSelf::CENTER)
                    .size(62.0, 62.0)
                    .background(soft_accent)
                    .border(1.0, dim_accent, 22.0)
                    .clip_rounded(22.0)
                    .justify_content(JustifyContent::CENTER)
                    .align_items(AlignItems::CENTER),
            )
            .child(RText(glyph).size(30.0).color(accent)),
            // Title, fixed height so cards line up
            Column(
                Modifier::new()
                    .fill_max_width()
                    .height(56.0)
                    .justify_content(JustifyContent::CENTER)
                    .align_items(AlignItems::CENTER),
            )
            .child(
                RText(card.title.clone())
                    .size(22.0)
                    .color(RColor::WHITE)
                    .max_lines(2)
                    .overflow_ellipsize(),
            ),
            // Description, bounded so a long one can't grow the card
            Column(
                Modifier::new()
                    .fill_max_width()
                    .height(72.0)
                    .padding(10.0)
                    .background(RColor::from_rgba(255, 255, 255, 10))
                    .clip_rounded(12.0),
            )
            .child(
                RText(card.description.clone())
                    .size(13.0)
                    .color(col(205, 210, 222))
                    .max_lines(4)
                    .overflow_ellipsize(),
            ),
            Spacer(),
            // Stack status
            Row(Modifier::new()
                .fill_max_width()
                .align_items(AlignItems::CENTER)
                .gap(8.0))
            .child((
                stack_pips(card.current_stacks, card.max_stacks, accent),
                Spacer(),
                reward_chip(stack_text, soft_accent, accent),
            )),
            // SELECT CTA
            Column(
                Modifier::new()
                    .fill_max_width()
                    .height(34.0)
                    .background(accent)
                    .clip_rounded(12.0)
                    .justify_content(JustifyContent::CENTER)
                    .align_items(AlignItems::CENTER),
            )
            .child(RText("SELECT").size(14.0).color(col(18, 18, 22))),
        )),
    ))
}

fn key_of(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

fn reward_chip(label: impl Into<String>, bg: RColor, fg: RColor) -> View {
    Column(
        Modifier::new()
            .padding_values(repose_core::PaddingValues {
                left: 9.0,
                right: 9.0,
                top: 5.0,
                bottom: 5.0,
            })
            .background(bg)
            .clip_rounded(999.0)
            .justify_content(JustifyContent::CENTER)
            .align_items(AlignItems::CENTER),
    )
    .child(
        RText(label.into())
            .size(11.0)
            .color(fg)
            .single_line()
            .overflow_ellipsize(),
    )
}

fn stack_pips(current: u32, max: u32, accent: RColor) -> View {
    let visible_max = max.min(6);
    let filled_up_to = current.min(visible_max);
    let mut pips: Vec<View> = Vec::new();

    for i in 0..visible_max {
        pips.push(Column(
            Modifier::new()
                .size(10.0, 10.0)
                .background(if i < filled_up_to {
                    accent
                } else {
                    RColor::from_rgba(255, 255, 255, 28)
                })
                .border(1.0, RColor::from_rgba(255, 255, 255, 36), 5.0)
                .clip_rounded(5.0),
        ));
    }

    if max > visible_max {
        pips.push(RText("+").size(11.0).color(col(150, 155, 168)));
    }

    Row(Modifier::new().gap(5.0).align_items(AlignItems::CENTER)).child(pips)
}

fn stack_label(current: u32, max: u32) -> String {
    if max >= 999 {
        format!("x{}", current)
    } else if current == 0 {
        format!("0/{}", max)
    } else {
        format!("{}/{}", current, max)
    }
}

fn reward_glyph(card: &RewardCardUi) -> &'static str {
    match card.category.as_str() {
        "damage" => "\u{27da}",
        "crit" => "\u{2726}",
        "survival" | "defense" => "\u{2665}",
        "mobility" => "\u{21e7}",
        "economy" => "\u{25c6}",
        "utility" => "\u{25c8}",
        _ => match card.rarity {
            4 | 5 => "\u{2739}",
            3 => "\u{2726}",
            _ => "\u{2022}",
        },
    }
}

fn rgb(rgb: [u8; 3]) -> RColor {
    RColor::from_rgba(rgb[0], rgb[1], rgb[2], 255)
}

fn rgb_alpha(rgb: [u8; 3], a: u8) -> RColor {
    RColor::from_rgba(rgb[0], rgb[1], rgb[2], a)
}

fn boss_banner_ui(st: &SharedUi) -> View {
    if st.boss_banner_timer <= 0.0 || st.run_phase != 0 {
        return Column(Modifier::new());
    }
    let a = ((st.boss_banner_timer / 1.0).clamp(0.0, 1.0) * 255.0) as u8;
    Column(
        Modifier::new()
            .fill_max_size()
            .justify_content(JustifyContent::FLEX_START)
            .align_items(AlignItems::CENTER)
            .padding(26.0),
    )
    .child(
        Column(
            Modifier::new()
                .padding(12.0)
                .background(RColor::from_rgba(18, 18, 28, a))
                .clip_rounded(10.0),
        )
        .child(
            RText("BOSS INCOMING")
                .size(34.0)
                .color(RColor::from_rgba(200, 120, 255, a)),
        ),
    )
}

fn game_over_ui(st: &SharedUi, actions: Arc<Mutex<Vec<UiAction>>>) -> View {
    let a = actions.clone();
    let tr = &st.translations;
    let title = if st.victory {
        t(tr, "run-complete", "RUN COMPLETE")
    } else {
        t(tr, "you-lose", "YOU LOSE")
    };
    let title_col = if st.victory {
        col(120, 220, 140)
    } else {
        col(220, 90, 90)
    };
    let subtitle = if st.victory {
        t(tr, "victory-bonus", "15 rounds cleared!")
    } else {
        format!("{} {}", t(tr, "round", "Round"), st.run_round)
    };

    Column(
        Modifier::new()
            .fill_max_size()
            .justify_content(JustifyContent::CENTER)
            .align_items(AlignItems::CENTER)
            .background(RColor::from_rgba(0, 0, 0, 170)),
    )
    .child(
        Column(
            Modifier::new()
                .width(460.0)
                .padding(28.0)
                .background(col(22, 22, 30))
                .clip_rounded(14.0)
                .align_items(AlignItems::CENTER),
        )
        .child([
            RText(title).size(42.0).color(title_col),
            spacer(8.0),
            RText(subtitle).size(16.0).color(col(200, 200, 210)),
            spacer(2.0),
            RText(&st.end_reason).size(13.0).color(col(160, 165, 175)),
            spacer(14.0),
            RText(format!(
                "{}: {}   {}: {}",
                t(tr, "score", "Score"),
                st.run_score,
                t(tr, "round", "Round"),
                st.run_round,
            ))
            .size(18.0)
            .color(RColor::WHITE),
            RText(format!("{}: +{}", t(tr, "bones", "Bones"), st.bones_earned))
                .size(22.0)
                .color(col(230, 200, 120)),
            spacer(18.0),
            RText(t(tr, "retry-hint", "R / Click = Retry"))
                .size(15.0)
                .color(col(170, 170, 180)),
            spacer(12.0),
            mk_button(
                &t(tr, "quit-to-title", "Quit to Title"),
                col(180, 60, 60),
                move || push(&a, UiAction::QuitToTitle),
            ),
        ]),
    )
}

fn settings_ui(overlay: OverlayHandle, st: &SharedUi, actions: Arc<Mutex<Vec<UiAction>>>) -> View {
    let a_m_down = actions.clone();
    let a_m_up = actions.clone();
    let a_s_down = actions.clone();
    let a_s_up = actions.clone();
    let a_mu_down = actions.clone();
    let a_mu_up = actions.clone();
    let a_save = actions.clone();
    let a_back = actions.clone();
    let master = st.master_vol;
    let sfx = st.sfx_vol;
    let music = st.music_vol;
    let tr = &st.translations;
    let lang = &st.language;
    let langs = &st.available_languages;
    let overlay_clone = overlay.clone();
    let actions_clone = actions.clone();

    let menu_state: Rc<MenuState> = remember(MenuState::new);
    let lang_items: Vec<DropdownMenuEntry> = langs
        .iter()
        .map(|l| {
            let a = actions_clone.clone();
            let code = l.clone();
            let mut item = DropdownMenuItem::new(l.clone(), move || {
                push(&a, UiAction::SetLanguage(code.clone()))
            });
            if l == lang {
                item = item.disabled();
            }
            DropdownMenuEntry::Item(item)
        })
        .collect();
    let menu_trigger = menu_state.clone();
    let lang_label = st.language.clone();
    let trigger = FilledTonalButton(
        Modifier::new().width(100.0).height(40.0),
        move || menu_trigger.open(),
        ButtonConfig::default(),
        move || RText(lang_label.clone()).size(20.0),
    );

    let lang_dropdown = DropdownMenu(
        menu_state,
        overlay_clone,
        Modifier::new(),
        trigger,
        lang_items,
        DropdownMenuConfig {
            min_width: 100.0,
            ..Default::default()
        },
    );

    let inner = Column(
        Modifier::new()
            .width(360.0)
            .padding(24.0)
            .background(col(20, 20, 28))
            .clip_rounded(12.0)
            .align_items(AlignItems::CENTER),
    )
    .child(
        RText(t(tr, "settings", "Settings"))
            .size(36.0)
            .color(RColor::WHITE),
    )
    .child(spacer(12.0))
    .child(
        RText(format!(
            "{}: {:.0}%",
            t(tr, "master-volume", "Master"),
            master * 100.0
        ))
        .size(18.0)
        .color(RColor::WHITE),
    )
    .child(Row(Modifier::new().gap(8.0)).child((
        mk_button_sm("-", move || {
            push(&a_m_down, UiAction::SetMasterVol(master - 0.1))
        }),
        mk_button_sm("+", move || {
            push(&a_m_up, UiAction::SetMasterVol(master + 0.1))
        }),
    )))
    .child(spacer(8.0))
    .child(
        RText(format!(
            "{}: {:.0}%",
            t(tr, "sfx-volume", "SFX"),
            sfx * 100.0
        ))
        .size(18.0)
        .color(RColor::WHITE),
    )
    .child(Row(Modifier::new().gap(8.0)).child((
        mk_button_sm("-", move || push(&a_s_down, UiAction::SetSfxVol(sfx - 0.1))),
        mk_button_sm("+", move || push(&a_s_up, UiAction::SetSfxVol(sfx + 0.1))),
    )))
    .child(spacer(8.0))
    .child(
        RText(format!(
            "{}: {:.0}%",
            t(tr, "music-volume", "Music"),
            music * 100.0
        ))
        .size(18.0)
        .color(RColor::WHITE),
    )
    .child(Row(Modifier::new().gap(8.0)).child((
        mk_button_sm("-", move || {
            push(&a_mu_down, UiAction::SetMusicVol(music - 0.1))
        }),
        mk_button_sm("+", move || {
            push(&a_mu_up, UiAction::SetMusicVol(music + 0.1))
        }),
    )))
    .child(spacer(8.0))
    .child(
        RText(format!("{}:", t(tr, "language", "Language")))
            .size(18.0)
            .color(RColor::WHITE),
    )
    .child(Row(Modifier::new().gap(6.0)).child(lang_dropdown))
    .child(spacer(16.0))
    .child(mk_button(
        &t(tr, "save", "Save"),
        col(60, 120, 200),
        move || push(&a_save, UiAction::SaveSettings),
    ))
    .child(mk_button(
        &t(tr, "back", "Back"),
        col(70, 70, 90),
        move || push(&a_back, UiAction::CloseOverlay),
    ));

    Column(
        Modifier::new()
            .fill_max_size()
            .justify_content(JustifyContent::CENTER)
            .align_items(AlignItems::CENTER)
            .background(RColor::from_rgba(0, 0, 0, 180)),
    )
    .child(inner)
}

fn credits_ui(st: &SharedUi, actions: Arc<Mutex<Vec<UiAction>>>) -> View {
    let a = actions.clone();
    let tr = &st.translations;
    let inner = Column(
        Modifier::new()
            .width(400.0)
            .padding(24.0)
            .background(col(20, 20, 28))
            .clip_rounded(12.0)
            .align_items(AlignItems::CENTER),
    )
    .child((
        RText(t(tr, "credits", "Credits"))
            .size(36.0)
            .color(RColor::WHITE),
        spacer(12.0),
        RText("Floppy Warriors — a janky ragdoll archery roguelite")
            .size(16.0)
            .color(RColor::WHITE),
        RText("Port of the Godot slice to Bevy + Repose")
            .size(16.0)
            .color(RColor::WHITE),
        RText("Engine: Bevy  Physics: bevy_rapier2d  UI: Repose")
            .size(16.0)
            .color(RColor::WHITE),
        spacer(16.0),
        mk_button(&t(tr, "back", "Back"), col(70, 70, 90), move || {
            push(&a, UiAction::CloseOverlay)
        }),
    ));

    Column(
        Modifier::new()
            .fill_max_size()
            .justify_content(JustifyContent::CENTER)
            .align_items(AlignItems::CENTER)
            .background(RColor::from_rgba(0, 0, 0, 180)),
    )
    .child(inner)
}

fn ingame_hud(st: &SharedUi) -> View {
    let tr = &st.translations;
    Column(
        Modifier::new()
            .fill_max_size()
            .padding(16.0)
            .align_items(AlignItems::FLEX_START)
            .justify_content(JustifyContent::FLEX_START),
    )
    .child((
        Row(Modifier::new().gap(18.0).align_items(AlignItems::CENTER)).child((
            RText(format!("{}: {}", t(tr, "round", "Round"), st.run_round))
                .size(22.0)
                .color(col(255, 210, 120)),
            RText(format!("{}: {}", t(tr, "score", "Score"), st.run_score))
                .size(22.0)
                .color(RColor::WHITE),
            RText(format!("{}: {}", t(tr, "best", "Best"), st.high_score))
                .size(16.0)
                .color(col(200, 200, 200)),
        )),
        RText(format!(
            "{}: {}/{}",
            t(tr, "hp", "HP"),
            st.player_hp,
            st.player_max_hp
        ))
        .size(16.0)
        .color(col(220, 120, 120)),
        RText(format!(
            "{}: {}/{}",
            t(tr, "enemy", "Enemy"),
            st.enemy_hp,
            st.enemy_max_hp
        ))
        .size(16.0)
        .color(col(120, 170, 220)),
        RText(&st.status_line).size(16.0).color(col(220, 220, 230)),
        spacer(4.0),
        RText(t(
            tr,
            "controls-hint",
            "Mouse aim  Hold click draw  Release shoot  Shift airdodge  Esc pause",
        ))
        .size(14.0)
        .color(col(180, 180, 180)),
    ))
}

fn mk_button(label: &str, _bg: RColor, on_click: impl Fn() + 'static) -> View {
    FilledTonalButton(
        Modifier::new().width(260.0).height(52.0).margin(8.0),
        on_click,
        ButtonConfig::default(),
        move || RText(label).size(20.0),
    )
}

fn mk_button_sm(label: &str, on_click: impl Fn() + 'static) -> View {
    FilledTonalButton(
        Modifier::new().width(48.0).height(40.0),
        on_click,
        ButtonConfig::default(),
        move || RText(label).size(20.0),
    )
}

fn col(r: u8, g: u8, b: u8) -> RColor {
    RColor::from_rgba(r, g, b, 255)
}

fn push(actions: &Arc<Mutex<Vec<UiAction>>>, a: UiAction) {
    if let Ok(mut q) = actions.lock() {
        q.push(a);
    }
}
