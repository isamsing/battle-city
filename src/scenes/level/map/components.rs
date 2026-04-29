use bevy::prelude::*;
use serde::Deserialize;

use crate::core::config::{MAP_WIDTH, MAP_HEIGHT};

#[derive(Component)]
pub struct Solid;

#[derive(Component)]
pub struct BrickTile;

#[derive(Clone, Copy, PartialEq, Deserialize)]
pub enum Tile {
    Empty,
    Brick,
    Steel,
    Water,
    Trees,
    Ice,
    Eagle,
}

#[derive(Deserialize)]
pub struct LevelData {
    pub brick_clusters: Vec<(i32, i32, i32, i32)>,
    pub steel_positions: Vec<(i32, i32)>,
    pub water_positions: Vec<(i32, i32)>,
    pub trees_positions: Vec<(i32, i32)>,
    pub eagle_position: (i32, i32),
    pub eagle_bricks: Vec<(i32, i32)>,
    pub player_spawns: Vec<(i32, i32)>,
}

impl LevelData {
    pub fn to_tile_grid(&self) -> Vec<Vec<Tile>> {
        let cols = MAP_WIDTH as usize;
        let rows = MAP_HEIGHT as usize;
        let mut map = vec![vec![Tile::Empty; cols]; rows];

        for &(col, row, w, h) in &self.brick_clusters {
            for r in row..row + h {
                for c in col..col + w {
                    if r >= 0 && r < MAP_HEIGHT as i32 && c >= 0 && c < MAP_WIDTH as i32 {
                        map[r as usize][c as usize] = Tile::Brick;
                    }
                }
            }
        }

        for &(col, row) in &self.steel_positions {
            map[row as usize][col as usize] = Tile::Steel;
        }

        for &(col, row) in &self.water_positions {
            map[row as usize][col as usize] = Tile::Water;
        }

        for &(col, row) in &self.trees_positions {
            map[row as usize][col as usize] = Tile::Trees;
        }

        let (ecol, erow) = self.eagle_position;
        map[erow as usize][ecol as usize] = Tile::Eagle;

        for &(col, row) in &self.eagle_bricks {
            map[row as usize][col as usize] = Tile::Brick;
        }

        map
    }
}
