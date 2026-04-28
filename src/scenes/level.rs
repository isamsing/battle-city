use bevy::prelude::*;

use crate::core::states::GameState;

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), setup_level)
            .add_systems(
                Update,
                (player_movement, animate_tank).run_if(in_state(GameState::InGame)),
            );
    }
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct TankAnimation {
    timer: Timer,
    frame: usize,
    direction: Direction,
}

#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn as_str(&self) -> &'static str {
        match self {
            Direction::Up => "up",
            Direction::Down => "down",
            Direction::Left => "left",
            Direction::Right => "right",
        }
    }
}

const TANK_SPEED: f32 = 150.0;

fn setup_level(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let texture = asset_server.load("sprites/tanks/player1/level1/up_f0.png");
    commands.spawn((
        Sprite {
            image: texture,
            custom_size: Some(Vec2::new(48.0, 48.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, -200.0, 0.0)),
        TankAnimation {
            timer: Timer::from_seconds(0.15, TimerMode::Repeating),
            frame: 0,
            direction: Direction::Up,
        },
        Player,
    ));
}

fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut TankAnimation), With<Player>>,
) {
    for (mut transform, mut anim) in &mut query {
        let mut moving = false;

        if keyboard.pressed(KeyCode::ArrowUp) {
            transform.translation.y += TANK_SPEED * time.delta_secs();
            anim.direction = Direction::Up;
            moving = true;
        } else if keyboard.pressed(KeyCode::ArrowDown) {
            transform.translation.y -= TANK_SPEED * time.delta_secs();
            anim.direction = Direction::Down;
            moving = true;
        } else if keyboard.pressed(KeyCode::ArrowLeft) {
            transform.translation.x -= TANK_SPEED * time.delta_secs();
            anim.direction = Direction::Left;
            moving = true;
        } else if keyboard.pressed(KeyCode::ArrowRight) {
            transform.translation.x += TANK_SPEED * time.delta_secs();
            anim.direction = Direction::Right;
            moving = true;
        }

        if !moving {
            anim.timer.reset();
            anim.frame = 0;
        }
    }
}

fn animate_tank(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    asset_server: Res<AssetServer>,
    mut query: Query<(&mut TankAnimation, &mut Sprite), With<Player>>,
) {
    let moving = keyboard.pressed(KeyCode::ArrowUp)
        || keyboard.pressed(KeyCode::ArrowDown)
        || keyboard.pressed(KeyCode::ArrowLeft)
        || keyboard.pressed(KeyCode::ArrowRight);

    for (mut anim, mut sprite) in &mut query {
        if moving {
            anim.timer.tick(time.delta());
            if anim.timer.just_finished() {
                anim.frame = (anim.frame + 1) % 2;
            }
        }

        let path = format!(
            "sprites/tanks/player1/level1/{}_f{}.png",
            anim.direction.as_str(),
            anim.frame
        );
        sprite.image = asset_server.load(path);
    }
}
