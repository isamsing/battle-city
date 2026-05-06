use bevy::prelude::*;

use crate::core::config::{TILE_SIZE, MAP_WIDTH, MAP_HEIGHT};

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

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum TankState {
    #[default]
    Spawning,
    Active,
    Exploding,
}

#[derive(Component, Clone)]
pub struct SpawnAnimation {
    pub timer: Timer,
    pub frame: usize,
    pub loops_remaining: usize,
}

impl SpawnAnimation {
    pub const TOTAL_FRAMES: usize = 4;
    pub const TOTAL_LOOPS: usize = 4;
    pub const FRAME_DURATION_SECS: f32 = 0.05;

    pub fn new() -> Self {
        Self {
            timer: Timer::from_seconds(Self::FRAME_DURATION_SECS, TimerMode::Repeating),
            frame: 0,
            loops_remaining: Self::TOTAL_LOOPS,
        }
    }

    pub fn sprite_path(&self) -> String {
        format!("sprites/spawn/spawn_f{}.png", self.frame)
    }
}

impl Default for SpawnAnimation {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Component, Clone)]
pub struct ExplosionAnimation {
    pub timer: Timer,
    pub frame: usize,
}

impl ExplosionAnimation {
    pub const TOTAL_FRAMES: usize = 5;
    pub const FRAME_DURATION_SECS: f32 = 0.08;

    pub fn new() -> Self {
        Self {
            timer: Timer::from_seconds(Self::FRAME_DURATION_SECS, TimerMode::Repeating),
            frame: 0,
        }
    }

    pub fn sprite_path(&self) -> &'static str {
        match self.frame {
            0 => "sprites/blast/blast_small_0.png",
            1 => "sprites/blast/blast_small_1.png",
            2 => "sprites/blast/blast_small_2.png",
            3 => "sprites/blast/blast_big_0.png",
            _ => "sprites/blast/blast_big_1.png",
        }
    }

    pub fn sprite_size(&self) -> f32 {
        if self.frame >= 3 { TILE_SIZE * 2.0 } else { TILE_SIZE }
    }
}

impl Default for ExplosionAnimation {
    fn default() -> Self {
        Self::new()
    }
}

pub const TANK_SPEED: f32 = 150.0;
pub const FIXED_DT: f32 = 1.0 / 60.0;
pub const TANK_SPEED_PER_FRAME: f32 = TANK_SPEED * FIXED_DT;

// --- Shared collision helpers ---

pub fn aabb_overlap(a_pos: Vec3, b_pos: Vec3, size: f32) -> bool {
    let half = size / 2.0;
    (a_pos.x - half) < (b_pos.x + half)
        && (a_pos.x + half) > (b_pos.x - half)
        && (a_pos.y - half) < (b_pos.y + half)
        && (a_pos.y + half) > (b_pos.y - half)
}

pub fn out_of_bounds(pos: Vec3) -> bool {
    let half = TILE_SIZE / 2.0;
    let half_map_w = MAP_WIDTH as f32 * TILE_SIZE / 2.0;
    let half_map_h = MAP_HEIGHT as f32 * TILE_SIZE / 2.0;
    (pos.x - half) < -half_map_w
        || (pos.x + half) > half_map_w
        || (pos.y - half) < -half_map_h
        || (pos.y + half) > half_map_h
}

pub fn collides_with_solids(pos: Vec3, solids: &[Vec3]) -> bool {
    solids.iter().any(|&s| aabb_overlap(pos, s, TILE_SIZE))
}

pub fn is_blocked(pos: Vec3, solids: &[Vec3]) -> bool {
    out_of_bounds(pos) || collides_with_solids(pos, solids)
}
