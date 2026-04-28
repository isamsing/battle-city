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
struct Solid;

#[derive(Component)]
struct Player {
    up: KeyCode,
    down: KeyCode,
    left: KeyCode,
    right: KeyCode,
    sprite_path: &'static str,
}

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
const TILE_SIZE: f32 = 24.0; // Display size for tiles (scaled up from 16x16)
const MAP_COLS: i32 = 26;
const MAP_ROWS: i32 = 26;

// Tile types for the level map
#[derive(Clone, Copy, PartialEq)]
enum Tile {
    Empty,
    Brick,
    Steel,
    Water,
    Trees,
    Ice,
    Eagle,
}

fn map_offset() -> Vec2 {
    // Center the map in the window
    Vec2::new(
        -(MAP_COLS as f32 * TILE_SIZE) / 2.0 + TILE_SIZE / 2.0,
        -(MAP_ROWS as f32 * TILE_SIZE) / 2.0 + TILE_SIZE / 2.0,
    )
}

fn tile_position(col: i32, row: i32) -> Vec3 {
    let offset = map_offset();
    Vec3::new(
        offset.x + col as f32 * TILE_SIZE,
        offset.y + (MAP_ROWS - 1 - row) as f32 * TILE_SIZE, // row 0 = top
        0.0,
    )
}

fn setup_level(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    // Define level layout (26x26 grid)
    // Row 0 = top of screen, Row 25 = bottom
    let level = build_level_1();

    // Spawn tiles
    for row in 0..MAP_ROWS {
        for col in 0..MAP_COLS {
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
        }
    }

    // Spawn player 1 (arrow keys) at bottom-left
    let p1_pos = tile_position(8, 24);
    commands.spawn((
        Sprite {
            image: asset_server.load("sprites/tanks/player1/level1/up_f0.png"),
            custom_size: Some(Vec2::splat(TILE_SIZE)),
            ..default()
        },
        Transform::from_translation(p1_pos),
        TankAnimation {
            timer: Timer::from_seconds(0.15, TimerMode::Repeating),
            frame: 0,
            direction: Direction::Up,
        },
        Player {
            up: KeyCode::ArrowUp,
            down: KeyCode::ArrowDown,
            left: KeyCode::ArrowLeft,
            right: KeyCode::ArrowRight,
            sprite_path: "sprites/tanks/player1/level1",
        },
    ));

    // Spawn player 2 (WASD) at bottom-right
    let p2_pos = tile_position(16, 24);
    commands.spawn((
        Sprite {
            image: asset_server.load("sprites/tanks/player2/level1/up_f0.png"),
            custom_size: Some(Vec2::splat(TILE_SIZE)),
            ..default()
        },
        Transform::from_translation(p2_pos),
        TankAnimation {
            timer: Timer::from_seconds(0.15, TimerMode::Repeating),
            frame: 0,
            direction: Direction::Up,
        },
        Player {
            up: KeyCode::KeyW,
            down: KeyCode::KeyS,
            left: KeyCode::KeyA,
            right: KeyCode::KeyD,
            sprite_path: "sprites/tanks/player2/level1",
        },
    ));
}

fn build_level_1() -> Vec<Vec<Tile>> {
    use Tile::*;

    let mut map = vec![vec![Empty; MAP_COLS as usize]; MAP_ROWS as usize];

    // Classic Battle City level 1 inspired layout
    // Brick clusters scattered around the map
    let brick_clusters: &[(i32, i32, i32, i32)] = &[
        // (col, row, width, height)
        (2, 2, 2, 4),
        (6, 2, 2, 4),
        (10, 2, 2, 4),
        (14, 2, 2, 4),
        (18, 2, 2, 4),
        (22, 2, 2, 4),
        (2, 8, 2, 4),
        (6, 8, 2, 4),
        (10, 8, 2, 4),
        (14, 8, 2, 4),
        (18, 8, 2, 4),
        (22, 8, 2, 4),
        (2, 14, 2, 4),
        (6, 14, 2, 4),
        (10, 14, 2, 2),
        (14, 14, 2, 2),
        (18, 14, 2, 4),
        (22, 14, 2, 4),
        (2, 20, 2, 4),
        (6, 20, 2, 4),
        (18, 20, 2, 4),
        (22, 20, 2, 4),
    ];

    for &(col, row, w, h) in brick_clusters {
        for r in row..row + h {
            for c in col..col + w {
                if r >= 0 && r < MAP_ROWS && c >= 0 && c < MAP_COLS {
                    map[r as usize][c as usize] = Brick;
                }
            }
        }
    }

    // Steel walls (a few strategic spots)
    let steel_positions: &[(i32, i32)] = &[
        (10, 10), (11, 10), (14, 10), (15, 10),
        (10, 11), (11, 11), (14, 11), (15, 11),
    ];
    for &(col, row) in steel_positions {
        map[row as usize][col as usize] = Steel;
    }

    // Water (blocking areas)
    let water_positions: &[(i32, i32)] = &[
        (10, 16), (11, 16), (14, 16), (15, 16),
        (10, 17), (11, 17), (14, 17), (15, 17),
    ];
    for &(col, row) in water_positions {
        map[row as usize][col as usize] = Water;
    }

    // Trees (cover)
    let trees_positions: &[(i32, i32)] = &[
        (0, 12), (1, 12), (24, 12), (25, 12),
        (0, 13), (1, 13), (24, 13), (25, 13),
    ];
    for &(col, row) in trees_positions {
        map[row as usize][col as usize] = Trees;
    }

    // Eagle at bottom center (row 25, cols 12-13)
    map[25][12] = Eagle;
    map[25][13] = Eagle;

    // Brick wall protecting the eagle
    // Top wall
    map[23][11] = Brick;
    map[23][12] = Brick;
    map[23][13] = Brick;
    map[23][14] = Brick;
    // Left wall
    map[24][11] = Brick;
    map[25][11] = Brick;
    // Right wall
    map[24][14] = Brick;
    map[25][14] = Brick;

    map
}

fn aabb_overlap(a_pos: Vec3, b_pos: Vec3, size: f32) -> bool {
    let half = size / 2.0;
    (a_pos.x - half) < (b_pos.x + half)
        && (a_pos.x + half) > (b_pos.x - half)
        && (a_pos.y - half) < (b_pos.y + half)
        && (a_pos.y + half) > (b_pos.y - half)
}

fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut player_query: Query<(&mut Transform, &mut TankAnimation, &Player)>,
    solid_query: Query<&Transform, (With<Solid>, Without<Player>)>,
) {
    for (mut transform, mut anim, player) in &mut player_query {
        let mut moving = false;
        let delta = TANK_SPEED * time.delta_secs();

        if keyboard.pressed(player.up) {
            let mut new_pos = transform.translation;
            new_pos.y += delta;
            if !collides_with_any(new_pos, &solid_query) {
                transform.translation = new_pos;
            }
            anim.direction = Direction::Up;
            moving = true;
        } else if keyboard.pressed(player.down) {
            let mut new_pos = transform.translation;
            new_pos.y -= delta;
            if !collides_with_any(new_pos, &solid_query) {
                transform.translation = new_pos;
            }
            anim.direction = Direction::Down;
            moving = true;
        } else if keyboard.pressed(player.left) {
            let mut new_pos = transform.translation;
            new_pos.x -= delta;
            if !collides_with_any(new_pos, &solid_query) {
                transform.translation = new_pos;
            }
            anim.direction = Direction::Left;
            moving = true;
        } else if keyboard.pressed(player.right) {
            let mut new_pos = transform.translation;
            new_pos.x += delta;
            if !collides_with_any(new_pos, &solid_query) {
                transform.translation = new_pos;
            }
            anim.direction = Direction::Right;
            moving = true;
        }

        if !moving {
            anim.timer.reset();
            anim.frame = 0;
        }
    }
}

fn collides_with_any(
    pos: Vec3,
    solids: &Query<&Transform, (With<Solid>, Without<Player>)>,
) -> bool {
    for solid_transform in solids.iter() {
        if aabb_overlap(pos, solid_transform.translation, TILE_SIZE) {
            return true;
        }
    }
    false
}

fn animate_tank(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    asset_server: Res<AssetServer>,
    mut query: Query<(&mut TankAnimation, &mut Sprite, &Player)>,
) {
    for (mut anim, mut sprite, player) in &mut query {
        let moving = keyboard.pressed(player.up)
            || keyboard.pressed(player.down)
            || keyboard.pressed(player.left)
            || keyboard.pressed(player.right);

        if moving {
            anim.timer.tick(time.delta());
            if anim.timer.just_finished() {
                anim.frame = (anim.frame + 1) % 2;
            }
        }

        let path = format!(
            "{}/{}_f{}.png",
            player.sprite_path,
            anim.direction.as_str(),
            anim.frame
        );
        sprite.image = asset_server.load(path);
    }
}
