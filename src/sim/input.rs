use bevy_ecs::prelude::*;
use glam::Vec2;

#[derive(Resource, Clone, Debug, Default)]
pub struct PlayerInput {
    pub aim_world: Option<Vec2>,
    pub stick: Vec2,
    pub draw_pressed: bool,
    pub draw_released: bool,
    pub dodge_pressed: bool,
    pub lmb_held: bool,
}

#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct RestartRequest(pub bool);
