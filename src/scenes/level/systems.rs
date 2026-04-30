use bevy::prelude::*;

use bevy::camera::{OrthographicProjection, Projection, ScalingMode};

use crate::core::config::{TILE_SIZE, MAP_WIDTH, MAP_HEIGHT};
use crate::net::GameMode;

use crate::scenes::bullet::components::FireCooldown;
use crate::scenes::map::systems::{tile_position, load_level, spawn_tiles};
use crate::scenes::player::components::{LocalPlayer, NetworkPlayer};
use crate::scenes::tank::components::*;

pub fn setup_level(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    game_mode: Res<GameMode>,
) {
    let map_w = MAP_WIDTH as f32 * TILE_SIZE;
    let map_h = MAP_HEIGHT as f32 * TILE_SIZE;

    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: map_w,
                min_height: map_h,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));

    let level_data = load_level(1);
    let grid = level_data.to_tile_grid();
    spawn_tiles(&mut commands, &asset_server, &grid);

    // Background music
    commands.spawn(AudioPlayer::new(asset_server.load("levels/background.mp3")))
        .insert(PlaybackSettings::LOOP);

    match *game_mode {
        GameMode::Local => {
            spawn_local_player(
                &mut commands,
                &asset_server,
                8,
                24,
                KeyCode::ArrowUp,
                KeyCode::ArrowDown,
                KeyCode::ArrowLeft,
                KeyCode::ArrowRight,
                "sprites/tanks/player1/level1",
            );
        }
        GameMode::OnlineHost(_) | GameMode::OnlineJoin(_) => {
            let local_handle: usize = if matches!(*game_mode, GameMode::OnlineHost(_)) {
                0
            } else {
                1
            };

            spawn_network_player(&mut commands, &asset_server, 0, 8, 24, "sprites/tanks/player1/level1");
            spawn_network_player(&mut commands, &asset_server, 1, 16, 24, "sprites/tanks/player2/level1");

            commands.insert_resource(bevy_ggrs::LocalPlayers(vec![local_handle]));
        }
    }
}

fn spawn_local_player(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    col: i32,
    row: i32,
    up: KeyCode,
    down: KeyCode,
    left: KeyCode,
    right: KeyCode,
    sprite_path: &'static str,
) {
    let pos = tile_position(col, row);
    commands.spawn((
        Sprite {
            image: asset_server.load(format!("{sprite_path}/up_f0.png")),
            custom_size: Some(Vec2::splat(TILE_SIZE)),
            ..default()
        },
        Transform::from_translation(pos),
        TankAnimation {
            timer: Timer::from_seconds(0.15, TimerMode::Repeating),
            frame: 0,
            direction: Direction::Up,
        },
        LocalPlayer {
            up,
            down,
            left,
            right,
            sprite_path,
        },
        Tank,
        FireCooldown::new(),
    ));
}

fn spawn_network_player(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    handle: usize,
    col: i32,
    row: i32,
    sprite_path: &'static str,
) {
    let pos = tile_position(col, row);
    commands.spawn((
        Sprite {
            image: asset_server.load(format!("{sprite_path}/up_f0.png")),
            custom_size: Some(Vec2::splat(TILE_SIZE)),
            ..default()
        },
        Transform::from_translation(pos),
        TankAnimation {
            timer: Timer::from_seconds(0.15, TimerMode::Repeating),
            frame: 0,
            direction: Direction::Up,
        },
        NetworkPlayer {
            handle,
            sprite_path,
        },
        Tank,
        FireCooldown::new(),
    ));
}
