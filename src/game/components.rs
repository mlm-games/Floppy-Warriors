use bevy::prelude::*;

#[derive(Component)]
pub struct GameCleanup;

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum LimbKind {
    Torso,
    Head,
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
            Self::ArmL | Self::ArmR => 0.75,
            Self::LegL | Self::LegR => 0.65,
        }
    }
}

#[derive(Component)]
pub struct WarriorLimb {
    pub root: Entity,
    pub kind: LimbKind,
    pub hit_radius: f32,
}

#[derive(Component)]
pub struct WarriorRoot {
    pub team: Team,
    pub health: i32,
    pub max_health: i32,
    pub is_dead: bool,
    pub torso: Entity,
    pub bow_pivot: Entity,
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
    pub velocity: Vec2,
    pub has_hit: bool,
    pub stuck_life: f32,
    pub stuck_to: Option<Entity>,
    pub stuck_local_offset: Vec2,
    pub stuck_angle_offset: f32,
}

#[derive(Component)]
pub struct Ground;

#[derive(Component)]
pub struct WorldHealthBar {
    pub root: Entity,
    pub fill: Entity,
    pub width: f32,
    pub y_offset: f32,
}

#[derive(Component)]
pub struct RagdollApplied;

#[derive(Component)]
pub struct ActivePuppetMotor {
    pub stand_y: f32,
    pub hover_strength: f32,
    pub hover_damping: f32,
    pub upright_strength: f32,
    pub upright_damping: f32,
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
