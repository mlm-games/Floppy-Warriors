use bevy_ecs::prelude::*;
use glam::Vec2;

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum LimbKind {
    Torso,
    Head,
    UpperArmL,
    UpperArmR,
    ArmL,
    ArmR,
    LegL,
    LegR,
}

impl LimbKind {
    pub fn damage_mult(self) -> f32 {
        match self {
            Self::Head => 2.0,
            Self::Torso => 1.0,
            Self::UpperArmL | Self::UpperArmR | Self::ArmL | Self::ArmR => 0.75,
            Self::LegL | Self::LegR => 0.65,
        }
    }
}

pub fn limb_tint(kind: LimbKind, archetype: [f32; 4]) -> [f32; 4] {
    let base = match kind {
        LimbKind::Torso => [0.3, 0.42, 0.62, 1.0],
        LimbKind::Head => [0.96, 0.82, 0.68, 1.0],
        LimbKind::UpperArmL | LimbKind::UpperArmR | LimbKind::LegL | LimbKind::LegR => {
            [0.86, 0.7, 0.55, 1.0]
        }
        LimbKind::ArmL | LimbKind::ArmR => [0.96, 0.82, 0.68, 1.0],
    };
    [
        (base[0] * archetype[0]).min(1.0),
        (base[1] * archetype[1]).min(1.0),
        (base[2] * archetype[2]).min(1.0),
        1.0,
    ]
}

#[derive(Component)]
pub struct WarriorLimb {
    pub root: Entity,
    pub kind: LimbKind,
    pub hit_radius: f32,
    pub size: Vec2,
}

#[derive(Component)]
pub struct WarriorRoot {
    pub team: Team,
    pub health: i32,
    pub max_health: i32,
    pub is_dead: bool,
    pub torso: Entity,
    pub damage_mult: f32,
    pub velocity_mult: f32,
    pub knockback_mult: f32,
    pub headshot_mult: f32,
    pub draw_speed_mult: f32,
    pub arrow_count: u32,
    pub spread_deg: f32,
    pub damage_taken_mult: f32,
    pub crit_chance: f32,
    pub crit_mult: f32,
    pub kill_heal: i32,
    pub revives: u32,
    pub last_stand: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Team {
    Player,
    Enemy,
}

#[derive(Component)]
pub struct PlayerTag;

#[derive(Component)]
pub struct EnemyTag;

#[derive(Component)]
pub struct BowState {
    pub drawing: bool,
    pub draw_power: f32,
}

#[derive(Component)]
pub struct BowAim {
    pub angle: f32,
}

#[derive(Component)]
pub struct Airdodge {
    pub impulse: f32,
    pub cooldown: f32,
    pub remaining: f32,
}

#[derive(Component)]
pub struct EnemyAi {
    pub aim_error: f32,
    pub decision_min: f32,
    pub decision_max: f32,
    pub decision_timer: f32,
    pub draw_timer: f32,
    pub drawing: bool,
    pub aim_offset: Vec2,
    pub target: Option<Entity>,
}

#[derive(Component)]
pub struct Arrow {
    pub shooter: Entity,
    pub team: Team,
    pub damage: f32,
    pub damage_mult: f32,
    pub headshot_mult: f32,
    pub knockback_mult: f32,
    pub crit_chance: f32,
    pub crit_mult: f32,
    pub pos: Vec2,
    pub velocity: Vec2,
    pub has_hit: bool,
    pub stuck_life: f32,
    pub stuck_to: Option<Entity>,
    pub stuck_local_offset: Vec2,
    pub stuck_angle_offset: f32,
    pub angle: f32,
}

#[derive(Component)]
pub struct RagdollApplied;

#[derive(Component)]
pub struct SkipTint;

#[derive(Component)]
pub struct BowVisual {
    pub base_size: Vec2,
    pub rest_x: f32,
    pub pull_distance: f32,
}

#[derive(Component)]
pub struct WorldHealthBar {
    pub root: Entity,
    pub width: f32,
    pub y_offset: f32,
}

#[derive(Component)]
pub struct ActivePuppetMotor {
    pub stand_y: f32,
    pub hover_strength: f32,
    pub hover_damping: f32,
    pub upright_strength: f32,
    pub upright_damping: f32,
}

#[derive(Component)]
pub struct HitStun {
    pub remaining: f32,
}

#[derive(Component)]
pub struct Recovery {
    pub remaining: f32,
}

#[derive(Component, Clone, Copy)]
pub struct ArchetypeTint {
    pub tint: [f32; 4],
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnemyArchetype {
    Grunt,
    Fast,
    Tank,
    Sniper,
    Splitter,
    Boss,
}

#[derive(Clone, Copy, Debug)]
pub struct CombatMods {
    pub damage_mult: f32,
    pub velocity_mult: f32,
    pub knockback_mult: f32,
    pub headshot_mult: f32,
    pub draw_speed_mult: f32,
    pub arrow_count: u32,
    pub spread_deg: f32,
    pub max_hp_bonus: i32,
    pub airdodge_cd_mult: f32,
    pub damage_taken_mult: f32,
    pub crit_chance: f32,
    pub crit_mult: f32,
}

impl Default for CombatMods {
    fn default() -> Self {
        Self {
            damage_mult: 1.0,
            velocity_mult: 1.0,
            knockback_mult: 1.0,
            headshot_mult: 1.0,
            draw_speed_mult: 1.0,
            arrow_count: 1,
            spread_deg: 10.0,
            max_hp_bonus: 0,
            airdodge_cd_mult: 1.0,
            damage_taken_mult: 1.0,
            crit_chance: 0.0,
            crit_mult: 2.0,
        }
    }
}
