//! Stage layouts and fixed positions (spawns, eagle).
//!
//! Stages are authored as 13x13 tile ASCII maps (`.` empty, `B` brick, `S` steel,
//! `W` water, `F` forest, `I` ice). Each tile expands to 2x2 map cells. Every
//! stage gets the same eagle, the same three enemy spawns and **four player
//! spawns around the base**; those tiles are forced clear when a stage is built.

use super::types::*;

pub const STAGES: [[&str; 13]; 4] = [
    [
        ".............",
        ".B.B.B.B.B.B.",
        ".B.B.B.B.B.B.",
        ".B.B.B.B.B.B.",
        ".B.B.BSB.B.B.",
        "......S......",
        "SS.BB...BB.SS",
        "......I......",
        ".B.B.B.B.B.B.",
        ".B.B.B.B.B.B.",
        ".B.B.F.F.B.B.",
        ".B.B.....B.B.",
        ".............",
    ],
    [
        ".............",
        ".S.B.B.B.B.S.",
        ".S.B.....B.S.",
        "...B.WWW.B...",
        "BB.B.WWW.B.BB",
        "......B......",
        ".BB.SSSSS.BB.",
        "......B......",
        "FF.B.....B.FF",
        "FF.B.III.B.FF",
        "...B.III.B...",
        ".B.B.....B.B.",
        ".............",
    ],
    [
        ".............",
        ".BB.B...B.BB.",
        ".BB.B.S.B.BB.",
        ".....BSB.....",
        "WW.BB...BB.WW",
        "WW.........WW",
        ".....SSS.....",
        "B.B.B...B.B.B",
        "B.B.B.F.B.B.B",
        "......F......",
        ".FF.B...B.FF.",
        "B...........B",
        ".............",
    ],
    [
        ".............",
        ".S.S.S.S.S.S.",
        "B.B.B.B.B.B.B",
        "B.B.B.B.B.B.B",
        ".............",
        "..SS.....SS..",
        "WW..BBBBB..WW",
        "WW..B...B..WW",
        "....B.I.B....",
        "II..BB.BB..II",
        "II.........II",
        ".B.B.....B.B.",
        ".............",
    ],
];

/// Player slot -> spawn tile column (row 12). P1 and P2 sit beside the base as
/// on the NES, P3 and P4 are further out on the same row.
pub const PLAYER_SPAWN_TILES: [i32; MAX_PLAYERS] = [4, 8, 2, 10];
/// Enemy spawn tile columns (row 0), used in rotation.
pub const ENEMY_SPAWN_TILES: [i32; 3] = [0, 6, 12];

/// The eagle occupies cells (12..14, 24..26).
pub const EAGLE_CELLS: [(i32, i32); 4] = [(12, 24), (13, 24), (12, 25), (13, 25)];
/// Bricks (or steel, with the shovel) protecting the eagle.
pub const EAGLE_RING: [(i32, i32); 8] =
    [(11, 23), (12, 23), (13, 23), (14, 23), (11, 24), (11, 25), (14, 24), (14, 25)];

pub fn count() -> usize {
    STAGES.len()
}

pub fn idx(cx: i32, cy: i32) -> usize {
    (cy * GRID_CELLS + cx) as usize
}

/// Top-left of a player's spawn point, in sub-pixels.
pub fn player_spawn(slot: usize) -> (i32, i32) {
    (PLAYER_SPAWN_TILES[slot] * TANK, 12 * TANK)
}

pub fn enemy_spawn(slot: usize) -> (i32, i32) {
    (ENEMY_SPAWN_TILES[slot % ENEMY_SPAWN_TILES.len()] * TANK, 0)
}

pub fn build_cells(stage: usize) -> Vec<Cell> {
    let rows = &STAGES[stage % STAGES.len()];
    let mut cells = vec![Cell::Empty; (GRID_CELLS * GRID_CELLS) as usize];
    for (ty, row) in rows.iter().enumerate() {
        for (tx, ch) in row.chars().enumerate() {
            let cell = match ch {
                'B' => Cell::Brick,
                'S' => Cell::Steel,
                'W' => Cell::Water,
                'F' => Cell::Forest,
                'I' => Cell::Ice,
                _ => Cell::Empty,
            };
            for dy in 0..2 {
                for dx in 0..2 {
                    cells[idx(tx as i32 * 2 + dx, ty as i32 * 2 + dy)] = cell;
                }
            }
        }
    }
    // Force the fixed tiles clear: player spawns and enemy spawns.
    let mut clear_tile = |tx: i32, ty: i32| {
        for dy in 0..2 {
            for dx in 0..2 {
                cells[idx(tx * 2 + dx, ty * 2 + dy)] = Cell::Empty;
            }
        }
    };
    for tx in PLAYER_SPAWN_TILES {
        clear_tile(tx, 12);
    }
    for tx in ENEMY_SPAWN_TILES {
        clear_tile(tx, 0);
    }
    // The eagle tile and its brick ring.
    clear_tile(6, 12);
    for (cx, cy) in EAGLE_RING {
        cells[idx(cx, cy)] = Cell::Brick;
    }
    for (cx, cy) in EAGLE_CELLS {
        cells[idx(cx, cy)] = Cell::Eagle;
    }
    cells
}
