use bevy::prelude::*;

use crate::scenes::tank::components::FIXED_DT;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum EnemyType {
    Basic,
    Fast,
    Power,
    Armor,
}

impl EnemyType {
    pub fn max_hp(&self) -> u8 {
        match self {
            EnemyType::Armor => 4,
            _ => 1,
        }
    }

    pub fn speed(&self) -> f32 {
        match self {
            EnemyType::Fast => 180.0,
            EnemyType::Power => 150.0,
            EnemyType::Basic | EnemyType::Armor => 75.0,
        }
    }

    pub fn speed_per_frame(&self) -> f32 {
        self.speed() * FIXED_DT
    }

    pub fn bullet_speed(&self) -> f32 {
        match self {
            EnemyType::Power => 450.0,
            EnemyType::Basic => 200.0,
            _ => 300.0,
        }
    }

    pub fn fire_interval(&self) -> f32 {
        match self {
            EnemyType::Power => 1.0,
            EnemyType::Fast | EnemyType::Armor => 1.5,
            EnemyType::Basic => 2.0,
        }
    }

    pub fn sprite_path(&self) -> &'static str {
        match self {
            EnemyType::Basic => "sprites/tanks/enemy/basic",
            EnemyType::Fast => "sprites/tanks/enemy/fast",
            EnemyType::Power => "sprites/tanks/enemy/power",
            EnemyType::Armor => "sprites/tanks/enemy/armor",
        }
    }

    pub fn score(&self) -> u32 {
        match self {
            EnemyType::Basic => 100,
            EnemyType::Fast => 200,
            EnemyType::Power => 300,
            EnemyType::Armor => 400,
        }
    }
}

#[derive(Component, Clone)]
pub struct EnemyTank {
    pub enemy_type: EnemyType,
    pub hp: u8,
}

#[derive(Component, Clone)]
pub struct EnemyAI {
    pub direction_timer: Timer,
    pub fire_timer: Timer,
    pub rng_state: u32,
}

impl EnemyAI {
    pub fn new(enemy_type: EnemyType, seed: u32) -> Self {
        Self {
            direction_timer: Timer::from_seconds(2.0, TimerMode::Repeating),
            fire_timer: Timer::from_seconds(enemy_type.fire_interval(), TimerMode::Repeating),
            rng_state: seed,
        }
    }
}

#[derive(Resource, Clone)]
pub struct EnemySpawnState {
    pub spawn_timer: Timer,
    pub enemies_remaining: usize,
    pub max_active: usize,
    pub spawn_queue: Vec<EnemyType>,
    pub next_spawn_slot: usize,
}

impl EnemySpawnState {
    pub fn new() -> Self {
        // Level 1 wave: 20 enemies total
        let mut spawn_queue = Vec::new();
        // 10 Basic, 4 Fast, 4 Power, 2 Armor (interleaved for variety)
        for _ in 0..3 { spawn_queue.push(EnemyType::Basic); }
        spawn_queue.push(EnemyType::Fast);
        for _ in 0..2 { spawn_queue.push(EnemyType::Basic); }
        spawn_queue.push(EnemyType::Power);
        spawn_queue.push(EnemyType::Fast);
        for _ in 0..2 { spawn_queue.push(EnemyType::Basic); }
        spawn_queue.push(EnemyType::Fast);
        spawn_queue.push(EnemyType::Power);
        for _ in 0..2 { spawn_queue.push(EnemyType::Basic); }
        spawn_queue.push(EnemyType::Power);
        spawn_queue.push(EnemyType::Fast);
        spawn_queue.push(EnemyType::Armor);
        spawn_queue.push(EnemyType::Basic);
        spawn_queue.push(EnemyType::Power);
        spawn_queue.push(EnemyType::Armor);

        Self {
            spawn_timer: Timer::from_seconds(3.0, TimerMode::Repeating),
            enemies_remaining: spawn_queue.len(),
            max_active: 4,
            spawn_queue,
            next_spawn_slot: 0,
        }
    }
}

#[derive(Resource, Default, Clone)]
pub struct Score {
    pub points: u32,
}
