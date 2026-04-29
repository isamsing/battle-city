use bevy::prelude::*;

use crate::core::config::{TILE_SIZE, MAP_WIDTH, MAP_HEIGHT};
use super::components::*;

// --- Map helpers ---

fn map_offset() -> Vec2 {
    Vec2::new(
        -(MAP_WIDTH as f32 * TILE_SIZE) / 2.0 + TILE_SIZE / 2.0,
        -(MAP_HEIGHT as f32 * TILE_SIZE) / 2.0 + TILE_SIZE / 2.0,
    )
}

pub fn tile_position(col: i32, row: i32) -> Vec3 {
    let offset = map_offset();
    Vec3::new(
        offset.x + col as f32 * TILE_SIZE,
        offset.y + (MAP_HEIGHT as i32 - 1 - row) as f32 * TILE_SIZE,
        0.0,
    )
}

// --- Tile spawning ---

pub fn spawn_tiles(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    level: &[Vec<Tile>],
) {
    let rows = MAP_HEIGHT as i32;
    let cols = MAP_WIDTH as i32;

    for row in 0..rows {
        for col in 0..cols {
            let tile = level[row as usize][col as usize];
            if tile == Tile::Empty {
                continue;
            }

            let texture_path = match tile {
                Tile::Brick => "sprites/tiles/brick_full.png",
                Tile::Steel => "sprites/tiles/steel_full.png",
                Tile::Water => "sprites/tiles/water_f0.png",
                Tile::Trees => "sprites/tiles/trees.png",
                Tile::Ice => "sprites/tiles/ice.png",
                Tile::Eagle => "sprites/tiles/eagle_alive.png",
                Tile::Empty => unreachable!(),
            };

            let pos = tile_position(col, row);
            let is_solid = matches!(tile, Tile::Brick | Tile::Steel | Tile::Water | Tile::Eagle);
            let mut entity = commands.spawn((
                Sprite {
                    image: asset_server.load(texture_path),
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    ..default()
                },
                Transform::from_translation(pos),
            ));
            if is_solid {
                entity.insert(Solid);
            }
            if tile == Tile::Brick {
                entity.insert(BrickTile);
            }
        }
    }
}

pub fn load_level(level_number: u32) -> LevelData {
    let ron_str = match level_number {
        1 => include_str!("../../../assets/levels/level_1.ron"),
        _ => panic!("Unknown level: {}", level_number),
    };
    ron::from_str(ron_str).expect("Failed to parse level RON data")
}
