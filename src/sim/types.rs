//! Constants and small value types shared by the simulation and the renderer.

use std::hash::Hash;

/// Sub-pixel resolution: positions are stored in 1/16 of an NES pixel.
pub const SUB: i32 = 16;
pub const CELL_PX: i32 = 8;
pub const TILE_PX: i32 = 16;
/// The playfield is 13x13 tiles of 16px (26x26 cells of 8px), like the NES.
pub const GRID_TILES: i32 = 13;
pub const GRID_CELLS: i32 = 26;
pub const FIELD_PX: i32 = GRID_TILES * TILE_PX;
pub const FIELD: i32 = FIELD_PX * SUB;
pub const CELL: i32 = CELL_PX * SUB;
pub const TANK: i32 = TILE_PX * SUB;
pub const BULLET_HALF: i32 = 2 * SUB;
pub const MAX_PLAYERS: usize = 4;

pub const SPAWN_FRAMES: u16 = 60;
pub const PLAYER_SHIELD_FRAMES: u16 = 180;
pub const HELMET_FRAMES: u16 = 600;
pub const FREEZE_FRAMES: u16 = 600;
pub const SHOVEL_FRAMES: u16 = 1200;
pub const STUN_FRAMES: u16 = 150;
pub const RESPAWN_DELAY: u16 = 60;
pub const STAGE_CLEAR_FRAMES: u16 = 240;
pub const START_LIVES: u8 = 3;
pub const PLAYER_SPEED: i32 = 16;
pub const ICE_SLIDE: u16 = 24;

pub const IN_UP: u8 = 1 << 0;
pub const IN_DOWN: u8 = 1 << 1;
pub const IN_LEFT: u8 = 1 << 2;
pub const IN_RIGHT: u8 = 1 << 3;
pub const IN_FIRE: u8 = 1 << 4;
pub const IN_ABILITY: u8 = 1 << 5;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Dir {
    Up,
    Right,
    Down,
    Left,
}

impl Dir {
    /// Unit step in sim coordinates (y grows downwards).
    pub fn delta(self) -> (i32, i32) {
        match self {
            Dir::Up => (0, -1),
            Dir::Right => (1, 0),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
        }
    }

    pub fn horizontal(self) -> bool {
        matches!(self, Dir::Left | Dir::Right)
    }

    pub fn from_index(i: u32) -> Dir {
        match i % 4 {
            0 => Dir::Up,
            1 => Dir::Right,
            2 => Dir::Down,
            _ => Dir::Left,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Dir::Up => "up",
            Dir::Right => "right",
            Dir::Down => "down",
            Dir::Left => "left",
        }
    }
}

/// One 8x8px map cell. A 16px map tile is 2x2 cells.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Cell {
    Empty,
    Brick,
    Steel,
    Water,
    Forest,
    Ice,
    Eagle,
}

impl Cell {
    /// Blocks tanks.
    pub fn solid(self) -> bool {
        matches!(self, Cell::Brick | Cell::Steel | Cell::Water | Cell::Eagle)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum EnemyKind {
    Basic,
    Fast,
    Power,
    Armor,
}

impl EnemyKind {
    pub const ALL: [EnemyKind; 4] = [EnemyKind::Basic, EnemyKind::Fast, EnemyKind::Power, EnemyKind::Armor];

    pub fn index(self) -> usize {
        match self {
            EnemyKind::Basic => 0,
            EnemyKind::Fast => 1,
            EnemyKind::Power => 2,
            EnemyKind::Armor => 3,
        }
    }

    pub fn hp(self) -> u8 {
        if self == EnemyKind::Armor { 4 } else { 1 }
    }

    /// Sub-pixels per frame.
    pub fn speed(self) -> i32 {
        match self {
            EnemyKind::Fast => 22,
            EnemyKind::Power => 16,
            EnemyKind::Basic | EnemyKind::Armor => 8,
        }
    }

    pub fn bullet_speed(self) -> i32 {
        match self {
            EnemyKind::Power => 40,
            EnemyKind::Basic => 20,
            _ => 28,
        }
    }

    /// Average frames between shots.
    pub fn fire_period(self) -> u32 {
        match self {
            EnemyKind::Power => 60,
            EnemyKind::Fast | EnemyKind::Armor => 80,
            EnemyKind::Basic => 120,
        }
    }

    pub fn score(self) -> u32 {
        100 * (self.index() as u32 + 1)
    }

    pub fn sprite_dir(self) -> &'static str {
        match self {
            EnemyKind::Basic => "basic",
            EnemyKind::Fast => "fast",
            EnemyKind::Power => "power",
            EnemyKind::Armor => "armor",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Owner {
    Player(u8),
    Enemy(u32),
}

/// Same order as the icons in the NES sprite sheet.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PowerKind {
    Helmet,
    Timer,
    Shovel,
    Star,
    Grenade,
    Tank,
}

impl PowerKind {
    pub const ALL: [PowerKind; 6] = [
        PowerKind::Helmet,
        PowerKind::Timer,
        PowerKind::Shovel,
        PowerKind::Star,
        PowerKind::Grenade,
        PowerKind::Tank,
    ];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|k| *k == self).unwrap()
    }

    pub fn sprite(self) -> &'static str {
        match self {
            PowerKind::Helmet => "sprites/powerups/helmet.png",
            PowerKind::Timer => "sprites/powerups/timer.png",
            PowerKind::Shovel => "sprites/powerups/shovel.png",
            PowerKind::Star => "sprites/powerups/star.png",
            PowerKind::Grenade => "sprites/powerups/grenade.png",
            PowerKind::Tank => "sprites/powerups/tank.png",
        }
    }
}

/// Per-frame input for one player slot.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub struct StepInput {
    pub bits: u8,
    /// `false` when the slot is controlled by the AI (player disconnected).
    pub human: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SimEventKind {
    PlayerFire,
    BrickBreak,
    SteelHit,
    EnemyHit,
    EnemyDied,
    PlayerDied,
    PlayerStunned,
    PowerUpSpawned,
    PowerUpTaken,
    EagleDestroyed,
    StageClear,
    GameOver,
}

/// Output-only notification for audio/VFX. Not part of the rollback state.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SimEvent {
    pub frame: u32,
    pub kind: SimEventKind,
    /// Centre position in sub-pixels.
    pub x: i32,
    pub y: i32,
}
