use bevy::prelude::*;

use super::config::TILE_SIZE;

#[derive(Component, Debug, Clone, Copy)]
pub struct GridPosition {
    pub x: f32,
    pub y: f32,
}

impl GridPosition {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn to_world(&self) -> Vec2 {
        Vec2::new(self.x * TILE_SIZE, self.y * TILE_SIZE)
    }

    pub fn from_world(world: Vec2) -> Self {
        Self {
            x: world.x / TILE_SIZE,
            y: world.y / TILE_SIZE,
        }
    }
}
