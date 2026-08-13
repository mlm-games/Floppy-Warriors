use crate::game::components::CombatMods;
use crate::save::SaveData;

#[derive(Clone, Copy)]
pub struct MetaDef {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub base_cost: u32,
    pub cost_growth: f32,
    pub max_level: u32,
}

pub const CATALOG: &[MetaDef] = &[
    MetaDef {
        id: "vitality",
        name: "Vitality",
        description: "+10 max HP / lvl",
        base_cost: 40,
        cost_growth: 1.55,
        max_level: 12,
    },
    MetaDef {
        id: "power",
        name: "Power",
        description: "+6% damage / lvl",
        base_cost: 55,
        cost_growth: 1.6,
        max_level: 10,
    },
    MetaDef {
        id: "quickdraw",
        name: "Quickdraw Drills",
        description: "+5% draw speed / lvl",
        base_cost: 50,
        cost_growth: 1.58,
        max_level: 10,
    },
    MetaDef {
        id: "head_hunter",
        name: "Head Hunter",
        description: "+8% headshot / lvl",
        base_cost: 70,
        cost_growth: 1.7,
        max_level: 8,
    },
    MetaDef {
        id: "impact",
        name: "Impact Training",
        description: "+8% knockback / lvl",
        base_cost: 45,
        cost_growth: 1.55,
        max_level: 8,
    },
    MetaDef {
        id: "bowstring",
        name: "Bowstring Oil",
        description: "+4% velocity / lvl",
        base_cost: 45,
        cost_growth: 1.55,
        max_level: 8,
    },
    MetaDef {
        id: "acrobat",
        name: "Acrobat",
        description: "-6% airdodge CD / lvl",
        base_cost: 60,
        cost_growth: 1.65,
        max_level: 6,
    },
    MetaDef {
        id: "fortune",
        name: "Bone Fortune",
        description: "+12% bones / lvl",
        base_cost: 80,
        cost_growth: 1.75,
        max_level: 5,
    },
    MetaDef {
        id: "multishot",
        name: "Starting Quiver",
        description: "+1 arrow every 2 lvls",
        base_cost: 200,
        cost_growth: 2.1,
        max_level: 4,
    },
];

pub fn cost_for(save: &SaveData, id: &str) -> u32 {
    let def = CATALOG.iter().find(|d| d.id == id).unwrap();
    let lvl = save.meta_level(id);
    if lvl >= def.max_level {
        return 0;
    }
    (def.base_cost as f32 * def.cost_growth.powi(lvl as i32)).round() as u32
}

pub fn can_buy(save: &SaveData, id: &str) -> bool {
    let def = CATALOG.iter().find(|d| d.id == id).unwrap();
    let lvl = save.meta_level(id);
    lvl < def.max_level && save.bones >= cost_for(save, id)
}

pub fn buy(save: &mut SaveData, id: &str) -> bool {
    if !can_buy(save, id) {
        return false;
    }
    let c = cost_for(save, id);
    save.bones -= c;
    *save.meta_levels.entry(id.to_string()).or_insert(0) += 1;
    true
}

pub fn apply_meta_to_mods(save: &SaveData, mods: &mut CombatMods) {
    let l = |id: &str| save.meta_level(id) as f32;
    mods.max_hp_bonus += (l("vitality") * 10.0) as i32;
    mods.damage_mult *= 1.0 + l("power") * 0.06;
    mods.draw_speed_mult *= 1.0 + l("quickdraw") * 0.05;
    mods.headshot_mult *= 1.0 + l("head_hunter") * 0.08;
    mods.knockback_mult *= 1.0 + l("impact") * 0.08;
    mods.velocity_mult *= 1.0 + l("bowstring") * 0.04;
    mods.airdodge_cd_mult *= (1.0 - l("acrobat") * 0.06).max(0.35);
    let extra = save.meta_level("multishot") / 2;
    mods.arrow_count = (mods.arrow_count + extra).min(5);
    mods.spread_deg += 4.0 * extra as f32;
}

pub fn bones_multiplier(save: &SaveData) -> f32 {
    1.0 + save.meta_level("fortune") as f32 * 0.12
}

pub fn calculate_run_bones(
    save: &SaveData,
    victory: bool,
    round_reached: u32,
    score: u32,
    kills: u32,
    headshots: u32,
) -> u32 {
    let raw =
        kills * 3 + headshots * 4 + round_reached * 6 + score / 8 + if victory { 120 } else { 0 };
    ((raw as f32) * bones_multiplier(save)).round().max(1.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::SaveData;

    #[test]
    fn bones_multiplier_scales_with_fortune() {
        let mut save = SaveData::default();
        save.meta_levels.insert("fortune".into(), 3);
        assert!((bones_multiplier(&save) - 1.36).abs() < 0.001);
    }

    #[test]
    fn calculate_run_bones_never_zero() {
        let save = SaveData::default();
        let bones = calculate_run_bones(&save, false, 1, 0, 0, 0);
        assert!(bones >= 1);
    }

    #[test]
    fn can_buy_respects_cost_and_max_level() {
        let mut save = SaveData {
            bones: 9999,
            ..Default::default()
        };
        assert!(can_buy(&save, "vitality"));

        save.meta_levels.insert("vitality".into(), 12);
        assert!(!can_buy(&save, "vitality"));

        save.meta_levels = Default::default();
        save.bones = 0;
        assert!(!can_buy(&save, "vitality"));
    }

    #[test]
    fn buy_charges_exactly_the_displayed_cost() {
        let mut save = SaveData {
            bones: 100_000,
            ..Default::default()
        };
        let before = save.bones;
        let cost = cost_for(&save, "power");

        assert!(buy(&mut save, "power"));
        assert_eq!(save.bones, before - cost);
        assert_eq!(save.meta_level("power"), 1);
    }
}
