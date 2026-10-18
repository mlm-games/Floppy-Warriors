use bevy_ecs::prelude::*;
use game_utils::i18n::LocaleResources;

#[derive(Resource, Clone, Default)]
pub struct I18nStrings(pub LocaleResources);

const TRANSLATION_KEYS: &[&str] = &[
    "app-title",
    "start-game",
    "settings",
    "credits",
    "quit",
    "paused",
    "resume",
    "quit-to-title",
    "save",
    "back",
    "master-volume",
    "sfx-volume",
    "music-volume",
    "language",
    "score",
    "best",
    "controls-hint",
    "loading",
    "bones",
    "best-round",
    "bone-shop",
    "round",
    "boss-round",
    "boss-incoming",
    "choose-reward",
    "reward-help",
    "you-lose",
    "run-complete",
    "victory-bonus",
    "retry",
    "retry-hint",
    "hp",
    "enemy",
    "level",
    "runs",
    "wins",
    "buy",
    "max",
    "select",
    "game-subtitle",
    "offline-bones",
];

const LOCALES: &[(&str, &str)] = &[
    ("en", include_str!("../../assets/locales/en/main.ftl")),
    ("es", include_str!("../../assets/locales/es/main.ftl")),
    ("fr", include_str!("../../assets/locales/fr/main.ftl")),
    ("de", include_str!("../../assets/locales/de/main.ftl")),
    ("ja", include_str!("../../assets/locales/ja/main.ftl")),
    ("zh", include_str!("../../assets/locales/zh/main.ftl")),
    ("pt", include_str!("../../assets/locales/pt/main.ftl")),
];

pub fn register(world: &mut World) {
    let mut res = LocaleResources::default();
    for (locale, ftl) in LOCALES {
        res.register(locale, ftl, TRANSLATION_KEYS);
    }
    world.insert_resource(I18nStrings(res));
}

pub fn t(world: &World, key: &str, fallback: &str) -> String {
    world
        .get_resource::<I18nStrings>()
        .and_then(|s| {
            game_utils::i18n::get_current_translations(&s.0)
                .get(key)
                .cloned()
        })
        .unwrap_or_else(|| fallback.to_string())
}
