use bevy::prelude::*;

use super::tank::{Direction, FIXED_DT};

#[derive(Component, Clone)]
pub struct Bullet {
    pub direction: Direction,
    pub owner: Entity,
}

#[derive(Component, Clone)]
pub struct FireCooldown {
    pub timer: Timer,
}

impl FireCooldown {
    pub fn new() -> Self {
        let mut timer = Timer::from_seconds(0.3, TimerMode::Once);
        // Start finished so the player can fire immediately
        timer.tick(timer.duration());
        Self { timer }
    }
}

pub const BULLET_SPEED: f32 = 300.0;
pub const BULLET_SPEED_PER_FRAME: f32 = BULLET_SPEED * FIXED_DT;
pub const BULLET_SIZE: f32 = 8.0;
