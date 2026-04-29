use bevy::prelude::*;

#[derive(Component)]
pub struct Tank;

#[derive(Component, Clone)]
pub struct TankAnimation {
    pub timer: Timer,
    pub frame: usize,
    pub direction: Direction,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Direction::Up => "up",
            Direction::Down => "down",
            Direction::Left => "left",
            Direction::Right => "right",
        }
    }

    pub fn to_velocity(&self) -> Vec2 {
        match self {
            Direction::Up => Vec2::new(0.0, 1.0),
            Direction::Down => Vec2::new(0.0, -1.0),
            Direction::Left => Vec2::new(-1.0, 0.0),
            Direction::Right => Vec2::new(1.0, 0.0),
        }
    }
}

pub const TANK_SPEED: f32 = 150.0;
pub const FIXED_DT: f32 = 1.0 / 60.0;
pub const TANK_SPEED_PER_FRAME: f32 = TANK_SPEED * FIXED_DT;
