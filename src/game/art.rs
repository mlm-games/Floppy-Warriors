use bevy::prelude::*;

/// Body-part textures rasterized from the Inkscape SVGs in `assets/images/`
/// (see `tools/export_art.sh`). Sprites reference these handles, then set
/// `custom_size` for scale and `color` for team/archetype tinting.
#[derive(Resource, Clone)]
pub struct WarriorArt {
    /// Torso with tunic + shoulder trim.
    pub body: Handle<Image>,
    /// Head with face.
    pub head: Handle<Image>,
    /// Short upper-arm segment.
    pub upper_arm: Handle<Image>,
    /// Forearm + fist (attached to the upper arm).
    pub hand: Handle<Image>,
    /// Shank.
    pub leg: Handle<Image>,
    /// Bow held on the bow pivot.
    pub bow: Handle<Image>,
    /// Landscape title backdrop.
    pub bg: Handle<Image>,
}

pub fn load_warrior_art(asset_server: &AssetServer) -> WarriorArt {
    let p = |name: &str| asset_server.load(format!("images/png/{name}.png"));
    WarriorArt {
        body: p("body"),
        head: p("head"),
        upper_arm: p("upper_arm"),
        hand: p("hand"),
        leg: p("leg"),
        bow: p("bow"),
        bg: p("bg"),
    }
}