use bevy::prelude::*;

use crate::core::states::{GameState, MenuScreen};

pub struct StartMenuPlugin;

impl Plugin for StartMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(MenuScreen::Title), setup_title)
            .add_systems(
                Update,
                (blink_prompt, animate_tank, handle_title_input)
                    .run_if(in_state(MenuScreen::Title)),
            )
            .add_systems(OnExit(MenuScreen::Title), cleanup_menu)
            .add_systems(OnEnter(MenuScreen::ModeSelect), setup_mode_select)
            .add_systems(
                Update,
                (handle_mode_select_input, update_cursor)
                    .run_if(in_state(MenuScreen::ModeSelect)),
            )
            .add_systems(OnExit(MenuScreen::ModeSelect), cleanup_menu);
    }
}

// --- Shared ---

#[derive(Component)]
struct MenuEntity;

fn cleanup_menu(mut commands: Commands, query: Query<Entity, With<MenuEntity>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

// --- Title Screen ---

#[derive(Component)]
struct BlinkingText {
    timer: Timer,
    visible: bool,
}

#[derive(Component)]
struct AnimatedTank {
    timer: Timer,
    frame: usize,
}

fn setup_title(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((Camera2d, MenuEntity));

    // Title text "BATTLE CITY"
    commands.spawn((
        Text::new("BATTLE CITY"),
        TextFont {
            font_size: 48.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(20.0),
            left: Val::Percent(50.0),
            ..default()
        },
        Transform::from_translation(Vec3::new(-150.0, 0.0, 0.0)),
        MenuEntity,
    ));

    // Animated tank sprite
    let texture = asset_server.load("sprites/tanks/player1/level1/down_f0.png");
    commands.spawn((
        Sprite {
            image: texture,
            custom_size: Some(Vec2::new(64.0, 64.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, -20.0, 0.0)),
        AnimatedTank {
            timer: Timer::from_seconds(0.3, TimerMode::Repeating),
            frame: 0,
        },
        MenuEntity,
    ));

    // "PRESS ENTER" blinking text
    commands.spawn((
        Text::new("PRESS ENTER"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Percent(20.0),
            left: Val::Percent(50.0),
            ..default()
        },
        Transform::from_translation(Vec3::new(-80.0, 0.0, 0.0)),
        BlinkingText {
            timer: Timer::from_seconds(0.5, TimerMode::Repeating),
            visible: true,
        },
        MenuEntity,
    ));
}

fn blink_prompt(time: Res<Time>, mut query: Query<(&mut BlinkingText, &mut TextColor)>) {
    for (mut blink, mut color) in &mut query {
        blink.timer.tick(time.delta());
        if blink.timer.just_finished() {
            blink.visible = !blink.visible;
            color.0 = if blink.visible {
                Color::WHITE
            } else {
                Color::srgba(1.0, 1.0, 1.0, 0.0)
            };
        }
    }
}

fn animate_tank(
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    mut query: Query<(&mut AnimatedTank, &mut Sprite)>,
) {
    for (mut tank, mut sprite) in &mut query {
        tank.timer.tick(time.delta());
        if tank.timer.just_finished() {
            tank.frame = (tank.frame + 1) % 2;
            let path = format!("sprites/tanks/player1/level1/down_f{}.png", tank.frame);
            sprite.image = asset_server.load(path);
        }
    }
}

fn handle_title_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<MenuScreen>>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        next_state.set(MenuScreen::ModeSelect);
    }
}

// --- Mode Select Screen ---

#[derive(Resource)]
struct SelectedOption(usize);

#[derive(Component)]
struct CursorTank;

const OPTION_Y: [f32; 2] = [20.0, -40.0];

fn setup_mode_select(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(SelectedOption(0));

    commands.spawn((Camera2d, MenuEntity));

    // "1 PLAYER" option
    commands.spawn((
        Text::new("SINGLEPLAYER"),
        TextFont {
            font_size: 32.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_translation(Vec3::new(-60.0, OPTION_Y[0], 0.0)),
        MenuEntity,
    ));

    // "2 PLAYERS" option
    commands.spawn((
        Text::new("MULTIPLAYER"),
        TextFont {
            font_size: 32.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_translation(Vec3::new(-60.0, OPTION_Y[1], 0.0)),
        MenuEntity,
    ));

    // Tank cursor sprite (points right)
    let texture = asset_server.load("sprites/tanks/player1/level1/right_f0.png");
    commands.spawn((
        Sprite {
            image: texture,
            custom_size: Some(Vec2::new(32.0, 32.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(-120.0, OPTION_Y[0], 0.0)),
        CursorTank,
        MenuEntity,
    ));
}

fn handle_mode_select_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selected: ResMut<SelectedOption>,
    mut game_state: ResMut<NextState<GameState>>,
    mut menu_state: ResMut<NextState<MenuScreen>>,
) {
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        selected.0 = 0;
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        selected.0 = 1;
    }
    if keyboard.just_pressed(KeyCode::Enter) {
        match selected.0 {
            0 => game_state.set(GameState::InGame),
            1 => menu_state.set(MenuScreen::Lobby),
            _ => {}
        }
    }
}

fn update_cursor(selected: Res<SelectedOption>, mut query: Query<&mut Transform, With<CursorTank>>) {
    for mut transform in &mut query {
        transform.translation.y = OPTION_Y[selected.0];
    }
}
