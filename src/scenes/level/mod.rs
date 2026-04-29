mod bullet;
mod map;
mod player;
mod systems;
mod tank;

use bevy::prelude::*;
use bevy_ggrs::prelude::*;

use crate::core::config::TILE_SIZE;
use crate::core::states::GameState;
use crate::net::{GameMode, is_networked};

pub use player::NetworkPlayer;
use bullet::*;
use map::tile_position;
use player::LocalPlayer;
use systems::*;
use tank::*;

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), setup_level)
            // Local-only systems (no networking)
            .add_systems(
                Update,
                (local_player_movement, local_animate_tank, local_fire_bullet,
                 move_bullets_local, bullet_collision)
                    .run_if(in_state(GameState::InGame))
                    .run_if(not(is_networked)),
            )
            // Networked deterministic systems (runs in GgrsSchedule)
            .add_systems(
                GgrsSchedule,
                (networked_player_movement, networked_fire_bullet,
                 move_bullets_networked, bullet_collision)
                    .run_if(is_networked),
            )
            // Networked visual-only animation (runs in normal Update)
            .add_systems(
                Update,
                networked_animate_tank
                    .run_if(in_state(GameState::InGame))
                    .run_if(is_networked),
            )
            // Register rollback components
            .rollback_component_with_clone::<Transform>()
            .rollback_component_with_clone::<TankAnimation>()
            .rollback_component_with_clone::<Bullet>()
            .rollback_component_with_clone::<FireCooldown>();
    }
}

// --- Setup ---

fn setup_level(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    game_mode: Res<GameMode>,
) {
    commands.spawn(Camera2d);

    let level_data = map::load_level(1);
    let grid = level_data.to_tile_grid();
    map::spawn_tiles(&mut commands, &asset_server, &grid);

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
