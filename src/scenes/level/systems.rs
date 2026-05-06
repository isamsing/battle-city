use bevy::prelude::*;

use bevy_ggrs::prelude::*;

use crate::core::config::TILE_SIZE;
use crate::core::states::GameState;
use crate::net::GameMode;
use crate::net::input::BattleCityConfig;

use crate::audio_resume::UserInteractionState;
use crate::core::states::WinnerInfo;
use crate::scenes::bullet::components::FireCooldown;
use crate::scenes::enemy::components::EnemySpawnState;
use crate::scenes::map::systems::{tile_position, load_level, spawn_tiles};
use crate::scenes::player::components::{LocalPlayer, NetworkPlayer};
use crate::scenes::tank::components::*;

#[derive(Component)]
pub struct LevelEntity;

#[derive(Resource)]
pub struct BackgroundMusicPending;

pub fn setup_gameplay(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    game_mode: Res<GameMode>,
) {
    let is_online = matches!(*game_mode, GameMode::OnlineHost(_) | GameMode::OnlineJoin(_));
    let level_data = load_level(if is_online { 2 } else { 1 });
    let grid = level_data.to_tile_grid();
    spawn_tiles(&mut commands, &asset_server, &grid, &level_data.eagle_positions);

    commands.insert_resource(BackgroundMusicPending);
    commands.insert_resource(EnemySpawnState::new());

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

            let (p1_col, p1_row) = (level_data.player_spawns[0].0, level_data.player_spawns[0].1);
            let (p2_col, p2_row) = (level_data.player_spawns[1].0, level_data.player_spawns[1].1);
            spawn_network_player(&mut commands, &asset_server, 0, p1_col, p1_row, "sprites/tanks/player1/level1");
            spawn_network_player(&mut commands, &asset_server, 1, p2_col, p2_row, "sprites/tanks/player2/level1");

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
    let spawn_anim = SpawnAnimation::new();
    let pos = tile_position(col, row);
    commands.spawn((
        Sprite {
            image: asset_server.load(spawn_anim.sprite_path()),
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
        TankState::Spawning,
        spawn_anim,
        FireCooldown::new(),
        LevelEntity,
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
    let spawn_anim = SpawnAnimation::new();
    let pos = tile_position(col, row);
    commands.spawn((
        Sprite {
            image: asset_server.load(spawn_anim.sprite_path()),
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
        TankState::Spawning,
        spawn_anim,
        FireCooldown::new(),
        LevelEntity,
    ));
}

pub fn spawn_background_music(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    interaction: Res<UserInteractionState>,
    pending: Option<Res<BackgroundMusicPending>>,
) {
    if pending.is_none() || !interaction.interacted {
        return;
    }
    commands.spawn((AudioPlayer::new(asset_server.load("levels/background.mp3")), LevelEntity))
        .insert(PlaybackSettings::LOOP);
    commands.remove_resource::<BackgroundMusicPending>();
}

pub fn local_spawn_animation(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut SpawnAnimation, &mut TankState)>,
) {
    for (entity, mut sprite, mut anim, mut state) in &mut query {
        if *state != TankState::Spawning {
            continue;
        }
        anim.timer.tick(time.delta());
        if anim.timer.just_finished() {
            anim.frame += 1;
            if anim.frame >= SpawnAnimation::TOTAL_FRAMES {
                anim.loops_remaining -= 1;
                if anim.loops_remaining == 0 {
                    *state = TankState::Active;
                    commands.entity(entity).remove::<SpawnAnimation>();
                    continue;
                }
                anim.frame = 0;
            }
            sprite.image = asset_server.load(anim.sprite_path());
        }
    }
}

pub fn networked_spawn_animation(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut query: Query<(Entity, &mut Sprite, &mut SpawnAnimation, &mut TankState)>,
) {
    for (entity, mut sprite, mut anim, mut state) in &mut query {
        if *state != TankState::Spawning {
            continue;
        }
        anim.timer.tick(std::time::Duration::from_secs_f32(FIXED_DT));
        if anim.timer.just_finished() {
            anim.frame += 1;
            if anim.frame >= SpawnAnimation::TOTAL_FRAMES {
                anim.loops_remaining -= 1;
                if anim.loops_remaining == 0 {
                    *state = TankState::Active;
                    commands.entity(entity).remove::<SpawnAnimation>();
                    continue;
                }
                anim.frame = 0;
            }
            sprite.image = asset_server.load(anim.sprite_path());
        }
    }
}

pub fn local_explosion_animation(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut ExplosionAnimation, &TankState)>,
) {
    for (entity, mut sprite, mut anim, state) in &mut query {
        if *state != TankState::Exploding {
            continue;
        }
        anim.timer.tick(time.delta());
        if anim.timer.just_finished() {
            anim.frame += 1;
            if anim.frame >= ExplosionAnimation::TOTAL_FRAMES {
                commands.entity(entity).despawn();
                continue;
            }
            sprite.image = asset_server.load(anim.sprite_path());
            sprite.custom_size = Some(Vec2::splat(anim.sprite_size()));
        }
    }
}

pub fn networked_explosion_animation(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut query: Query<(Entity, &mut Sprite, &mut ExplosionAnimation, &TankState)>,
) {
    for (entity, mut sprite, mut anim, state) in &mut query {
        if *state != TankState::Exploding {
            continue;
        }
        anim.timer.tick(std::time::Duration::from_secs_f32(FIXED_DT));
        if anim.timer.just_finished() {
            anim.frame += 1;
            if anim.frame >= ExplosionAnimation::TOTAL_FRAMES {
                commands.entity(entity).despawn();
                continue;
            }
            sprite.image = asset_server.load(anim.sprite_path());
            sprite.custom_size = Some(Vec2::splat(anim.sprite_size()));
        }
    }
}

pub fn show_game_over(mut commands: Commands, winner: Option<Res<WinnerInfo>>) {
    let player_num = winner.map(|w| w.winner_handle + 1).unwrap_or(1);
    commands.spawn((
        Text::new(format!("Player {} Wins!", player_num)),
        TextFont {
            font_size: 48.0,
            ..default()
        },
        TextColor(Color::WHITE),
        TextLayout::new_with_justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(40.0),
            width: Val::Percent(100.0),
            ..default()
        },
        LevelEntity,
    ));
}

pub fn cleanup_level(mut commands: Commands, query: Query<Entity, With<LevelEntity>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

pub fn cleanup_network_session(mut commands: Commands, mut game_mode: ResMut<GameMode>) {
    commands.remove_resource::<Session<BattleCityConfig>>();
    commands.remove_resource::<bevy_ggrs::LocalPlayers>();
    commands.remove_resource::<crate::net::session::MatchboxRes>();
    *game_mode = GameMode::Local;
}

pub fn handle_escape_to_menu(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        game_state.set(GameState::Menu);
    }
}
