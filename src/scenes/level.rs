use bevy::prelude::*;
use bevy_ggrs::prelude::*;

use crate::core::config::TILE_SIZE;
use crate::core::states::GameState;
use crate::net::input::{BattleCityConfig, INPUT_UP, INPUT_DOWN, INPUT_LEFT, INPUT_RIGHT, INPUT_FIRE};
use crate::net::{GameMode, is_networked};

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), setup_level)
            // Local-only systems (no networking)
            .add_systems(
                Update,
                (local_player_movement, local_animate_tank, local_fire_bullet,
                 move_bullets_local, bullet_collision_local)
                    .run_if(in_state(GameState::InGame))
                    .run_if(not(is_networked)),
            )
            // Networked deterministic systems (runs in GgrsSchedule)
            .add_systems(
                GgrsSchedule,
                (networked_player_movement, networked_fire_bullet,
                 move_bullets_networked, bullet_collision_networked)
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

// --- Components ---

#[derive(Component)]
struct Solid;

#[derive(Component)]
struct Tank;

/// Local-only player with keybinds (used in single-player / local 2P)
#[derive(Component)]
struct LocalPlayer {
    up: KeyCode,
    down: KeyCode,
    left: KeyCode,
    right: KeyCode,
    sprite_path: &'static str,
}

/// Networked player with GGRS handle
#[derive(Component)]
#[require(Rollback)]
pub struct NetworkPlayer {
    pub handle: usize,
    pub sprite_path: &'static str,
}

#[derive(Component, Clone)]
struct TankAnimation {
    timer: Timer,
    frame: usize,
    direction: Direction,
}

#[derive(Component, Clone)]
struct Bullet {
    direction: Direction,
    owner: Entity,
}

#[derive(Component, Clone)]
struct FireCooldown {
    timer: Timer,
}

impl FireCooldown {
    fn new() -> Self {
        let mut timer = Timer::from_seconds(0.3, TimerMode::Once);
        // Start finished so the player can fire immediately
        timer.tick(timer.duration());
        Self { timer }
    }
}

#[derive(Component)]
struct BrickTile;

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

    fn to_velocity(&self) -> Vec2 {
        match self {
            Direction::Up => Vec2::new(0.0, 1.0),
            Direction::Down => Vec2::new(0.0, -1.0),
            Direction::Left => Vec2::new(-1.0, 0.0),
            Direction::Right => Vec2::new(1.0, 0.0),
        }
    }
}

// --- Constants ---

const TANK_SPEED: f32 = 150.0;
const FIXED_DT: f32 = 1.0 / 60.0;
const TANK_SPEED_PER_FRAME: f32 = TANK_SPEED * FIXED_DT;
const BULLET_SPEED: f32 = 300.0;
const BULLET_SPEED_PER_FRAME: f32 = BULLET_SPEED * FIXED_DT;
const BULLET_SIZE: f32 = 8.0;
const MAP_COLS: i32 = 26;
const MAP_ROWS: i32 = 26;

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

// --- Map helpers ---

fn map_offset() -> Vec2 {
    Vec2::new(
        -(MAP_COLS as f32 * TILE_SIZE) / 2.0 + TILE_SIZE / 2.0,
        -(MAP_ROWS as f32 * TILE_SIZE) / 2.0 + TILE_SIZE / 2.0,
    )
}

fn tile_position(col: i32, row: i32) -> Vec3 {
    let offset = map_offset();
    Vec3::new(
        offset.x + col as f32 * TILE_SIZE,
        offset.y + (MAP_ROWS - 1 - row) as f32 * TILE_SIZE,
        0.0,
    )
}

// --- Setup ---

fn setup_level(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    game_mode: Res<GameMode>,
) {
    commands.spawn(Camera2d);

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
            if tile == Tile::Brick {
                entity.insert(BrickTile);
            }
        }
    }

    match *game_mode {
        GameMode::Local => {
            // Single player: spawn only P1 with arrow keys
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

            // Player 1 (handle 0) at bottom-left
            spawn_network_player(
                &mut commands,
                &asset_server,
                0,
                8,
                24,
                "sprites/tanks/player1/level1",
            );
            // Player 2 (handle 1) at bottom-right
            spawn_network_player(
                &mut commands,
                &asset_server,
                1,
                16,
                24,
                "sprites/tanks/player2/level1",
            );

            // Tell GGRS which handle is local
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

// --- Level data ---

fn build_level_1() -> Vec<Vec<Tile>> {
    use Tile::*;

    let mut map = vec![vec![Empty; MAP_COLS as usize]; MAP_ROWS as usize];

    let brick_clusters: &[(i32, i32, i32, i32)] = &[
        (2, 2, 2, 4),   (6, 2, 2, 4),   (10, 2, 2, 4),
        (14, 2, 2, 4),  (18, 2, 2, 4),  (22, 2, 2, 4),
        (2, 8, 2, 4),   (6, 8, 2, 4),   (10, 8, 2, 4),
        (14, 8, 2, 4),  (18, 8, 2, 4),  (22, 8, 2, 4),
        (2, 14, 2, 4),  (6, 14, 2, 4),  (10, 14, 2, 2),
        (14, 14, 2, 2), (18, 14, 2, 4), (22, 14, 2, 4),
        (2, 20, 2, 4),  (6, 20, 2, 4),  (18, 20, 2, 4),
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

    let steel_positions: &[(i32, i32)] = &[
        (10, 10), (11, 10), (14, 10), (15, 10),
        (10, 11), (11, 11), (14, 11), (15, 11),
    ];
    for &(col, row) in steel_positions {
        map[row as usize][col as usize] = Steel;
    }

    let water_positions: &[(i32, i32)] = &[
        (10, 16), (11, 16), (14, 16), (15, 16),
        (10, 17), (11, 17), (14, 17), (15, 17),
    ];
    for &(col, row) in water_positions {
        map[row as usize][col as usize] = Water;
    }

    let trees_positions: &[(i32, i32)] = &[
        (0, 12), (1, 12), (24, 12), (25, 12),
        (0, 13), (1, 13), (24, 13), (25, 13),
    ];
    for &(col, row) in trees_positions {
        map[row as usize][col as usize] = Trees;
    }

    map[25][12] = Eagle;
    map[23][11] = Brick;
    map[23][12] = Brick;
    map[23][13] = Brick;
    map[23][14] = Brick;
    map[24][11] = Brick;
    map[25][11] = Brick;
    map[24][14] = Brick;
    map[25][14] = Brick;

    map
}

// --- Collision ---

fn aabb_overlap(a_pos: Vec3, b_pos: Vec3, size: f32) -> bool {
    let half = size / 2.0;
    (a_pos.x - half) < (b_pos.x + half)
        && (a_pos.x + half) > (b_pos.x - half)
        && (a_pos.y - half) < (b_pos.y + half)
        && (a_pos.y + half) > (b_pos.y - half)
}

fn collides_with_solids(pos: Vec3, solids: &[Vec3]) -> bool {
    solids.iter().any(|&s| aabb_overlap(pos, s, TILE_SIZE))
}

// --- Local movement (single-player, delta-time based) ---

fn local_player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut player_query: Query<(Entity, &mut Transform, &mut TankAnimation, &LocalPlayer), With<Tank>>,
    solid_query: Query<&Transform, (With<Solid>, Without<Tank>)>,
) {
    let solid_positions: Vec<Vec3> = solid_query.iter().map(|t| t.translation).collect();
    let all_tanks: Vec<(Entity, Vec3)> = player_query.iter().map(|(e, t, _, _)| (e, t.translation)).collect();

    for (entity, mut transform, mut anim, player) in &mut player_query {
        let mut obstacles = solid_positions.clone();
        for &(other_entity, other_pos) in &all_tanks {
            if other_entity != entity {
                obstacles.push(other_pos);
            }
        }
        let mut moving = false;
        let delta = TANK_SPEED * time.delta_secs();

        if keyboard.pressed(player.up) {
            let mut new_pos = transform.translation;
            new_pos.y += delta;
            if !collides_with_solids(new_pos, &obstacles) {
                transform.translation = new_pos;
            }
            anim.direction = Direction::Up;
            moving = true;
        } else if keyboard.pressed(player.down) {
            let mut new_pos = transform.translation;
            new_pos.y -= delta;
            if !collides_with_solids(new_pos, &obstacles) {
                transform.translation = new_pos;
            }
            anim.direction = Direction::Down;
            moving = true;
        } else if keyboard.pressed(player.left) {
            let mut new_pos = transform.translation;
            new_pos.x -= delta;
            if !collides_with_solids(new_pos, &obstacles) {
                transform.translation = new_pos;
            }
            anim.direction = Direction::Left;
            moving = true;
        } else if keyboard.pressed(player.right) {
            let mut new_pos = transform.translation;
            new_pos.x += delta;
            if !collides_with_solids(new_pos, &obstacles) {
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

fn local_animate_tank(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    asset_server: Res<AssetServer>,
    mut query: Query<(&mut TankAnimation, &mut Sprite, &LocalPlayer)>,
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

// --- Networked movement (deterministic, fixed timestep, runs in GgrsSchedule) ---

fn networked_player_movement(
    inputs: Res<PlayerInputs<BattleCityConfig>>,
    mut player_query: Query<(Entity, &mut Transform, &mut TankAnimation, &NetworkPlayer), With<Tank>>,
    solid_query: Query<&Transform, (With<Solid>, Without<Tank>)>,
) {
    let solid_positions: Vec<Vec3> = solid_query.iter().map(|t| t.translation).collect();
    let all_tanks: Vec<(Entity, Vec3)> = player_query.iter().map(|(e, t, _, _)| (e, t.translation)).collect();

    for (entity, mut transform, mut anim, net_player) in &mut player_query {
        let mut obstacles = solid_positions.clone();
        for &(other_entity, other_pos) in &all_tanks {
            if other_entity != entity {
                obstacles.push(other_pos);
            }
        }
        let (input, _status) = inputs[net_player.handle];
        let flags = input.0;
        let mut moving = false;

        if flags & INPUT_UP != 0 {
            let mut new_pos = transform.translation;
            new_pos.y += TANK_SPEED_PER_FRAME;
            if !collides_with_solids(new_pos, &obstacles) {
                transform.translation = new_pos;
            }
            anim.direction = Direction::Up;
            moving = true;
        } else if flags & INPUT_DOWN != 0 {
            let mut new_pos = transform.translation;
            new_pos.y -= TANK_SPEED_PER_FRAME;
            if !collides_with_solids(new_pos, &obstacles) {
                transform.translation = new_pos;
            }
            anim.direction = Direction::Down;
            moving = true;
        } else if flags & INPUT_LEFT != 0 {
            let mut new_pos = transform.translation;
            new_pos.x -= TANK_SPEED_PER_FRAME;
            if !collides_with_solids(new_pos, &obstacles) {
                transform.translation = new_pos;
            }
            anim.direction = Direction::Left;
            moving = true;
        } else if flags & INPUT_RIGHT != 0 {
            let mut new_pos = transform.translation;
            new_pos.x += TANK_SPEED_PER_FRAME;
            if !collides_with_solids(new_pos, &obstacles) {
                transform.translation = new_pos;
            }
            anim.direction = Direction::Right;
            moving = true;
        }

        if !moving {
            anim.timer.reset();
            anim.frame = 0;
        } else {
            anim.timer.tick(std::time::Duration::from_secs_f32(FIXED_DT));
            if anim.timer.just_finished() {
                anim.frame = (anim.frame + 1) % 2;
            }
        }
    }
}

// --- Networked animation (visual-only, reads TankAnimation state) ---

fn networked_animate_tank(
    asset_server: Res<AssetServer>,
    mut query: Query<(&TankAnimation, &mut Sprite, &NetworkPlayer)>,
) {
    for (anim, mut sprite, net_player) in &mut query {
        let path = format!(
            "{}/{}_f{}.png",
            net_player.sprite_path,
            anim.direction.as_str(),
            anim.frame
        );
        sprite.image = asset_server.load(path);
    }
}

// --- Bullet helpers ---

fn spawn_bullet(commands: &mut Commands, asset_server: &AssetServer, pos: Vec3, direction: Direction, owner: Entity) {
    let offset = direction.to_velocity() * TILE_SIZE * 0.6;
    let bullet_pos = Vec3::new(pos.x + offset.x, pos.y + offset.y, 1.0);
    let sprite_path = format!("sprites/bullets/bullet_{}.png", direction.as_str());
    commands.spawn((
        Sprite {
            image: asset_server.load(sprite_path),
            custom_size: Some(Vec2::splat(BULLET_SIZE)),
            ..default()
        },
        Transform::from_translation(bullet_pos),
        Bullet { direction, owner },
    ));
}

fn bullet_out_of_bounds(pos: Vec3) -> bool {
    let half_map_w = MAP_COLS as f32 * TILE_SIZE / 2.0;
    let half_map_h = MAP_ROWS as f32 * TILE_SIZE / 2.0;
    pos.x < -half_map_w || pos.x > half_map_w || pos.y < -half_map_h || pos.y > half_map_h
}

fn bullet_hits_rect(bullet_pos: Vec3, target_pos: Vec3, target_size: f32) -> bool {
    let half_b = BULLET_SIZE / 2.0;
    let half_t = target_size / 2.0;
    (bullet_pos.x - half_b) < (target_pos.x + half_t)
        && (bullet_pos.x + half_b) > (target_pos.x - half_t)
        && (bullet_pos.y - half_b) < (target_pos.y + half_t)
        && (bullet_pos.y + half_b) > (target_pos.y - half_t)
}

// --- Local bullet systems ---

fn local_fire_bullet(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(Entity, &Transform, &TankAnimation, &mut FireCooldown), With<LocalPlayer>>,
) {
    for (entity, transform, anim, mut cooldown) in &mut query {
        cooldown.timer.tick(time.delta());
        if keyboard.just_pressed(KeyCode::Space) && cooldown.timer.is_finished() {
            spawn_bullet(&mut commands, &asset_server, transform.translation, anim.direction, entity);
            cooldown.timer.reset();
        }
    }
}

fn move_bullets_local(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &Bullet)>,
) {
    for (entity, mut transform, bullet) in &mut query {
        let vel = bullet.direction.to_velocity();
        let delta = BULLET_SPEED * time.delta_secs();
        transform.translation.x += vel.x * delta;
        transform.translation.y += vel.y * delta;
        if bullet_out_of_bounds(transform.translation) {
            commands.entity(entity).despawn();
        }
    }
}

fn bullet_collision_local(
    mut commands: Commands,
    bullet_query: Query<(Entity, &Transform, &Bullet), Without<Tank>>,
    tank_query: Query<(Entity, &Transform), (With<Tank>, Without<Bullet>)>,
    solid_query: Query<(Entity, &Transform, Option<&BrickTile>), (With<Solid>, Without<Tank>, Without<Bullet>)>,
) {
    for (bullet_entity, bullet_transform, bullet) in &bullet_query {
        let bpos = bullet_transform.translation;
        let mut bullet_hit = false;

        // Check solid tiles
        for (solid_entity, solid_transform, brick) in &solid_query {
            if bullet_hits_rect(bpos, solid_transform.translation, TILE_SIZE) {
                bullet_hit = true;
                if brick.is_some() {
                    commands.entity(solid_entity).despawn();
                }
                break;
            }
        }

        // Check tanks
        if !bullet_hit {
            for (tank_entity, tank_transform) in &tank_query {
                if tank_entity == bullet.owner {
                    continue;
                }
                if bullet_hits_rect(bpos, tank_transform.translation, TILE_SIZE) {
                    bullet_hit = true;
                    commands.entity(tank_entity).despawn();
                    break;
                }
            }
        }

        if bullet_hit {
            commands.entity(bullet_entity).despawn();
        }
    }
}

// --- Networked bullet systems ---

fn networked_fire_bullet(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    inputs: Res<PlayerInputs<BattleCityConfig>>,
    mut query: Query<(Entity, &Transform, &TankAnimation, &NetworkPlayer, &mut FireCooldown), With<Tank>>,
) {
    for (entity, transform, anim, net_player, mut cooldown) in &mut query {
        cooldown.timer.tick(std::time::Duration::from_secs_f32(FIXED_DT));
        let (input, _status) = inputs[net_player.handle];
        if input.0 & INPUT_FIRE != 0 && cooldown.timer.is_finished() {
            spawn_bullet(&mut commands, &asset_server, transform.translation, anim.direction, entity);
            cooldown.timer.reset();
        }
    }
}

fn move_bullets_networked(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &Bullet)>,
) {
    for (entity, mut transform, bullet) in &mut query {
        let vel = bullet.direction.to_velocity();
        transform.translation.x += vel.x * BULLET_SPEED_PER_FRAME;
        transform.translation.y += vel.y * BULLET_SPEED_PER_FRAME;
        if bullet_out_of_bounds(transform.translation) {
            commands.entity(entity).despawn();
        }
    }
}

fn bullet_collision_networked(
    mut commands: Commands,
    bullet_query: Query<(Entity, &Transform, &Bullet), Without<Tank>>,
    tank_query: Query<(Entity, &Transform), (With<Tank>, Without<Bullet>)>,
    solid_query: Query<(Entity, &Transform, Option<&BrickTile>), (With<Solid>, Without<Tank>, Without<Bullet>)>,
) {
    for (bullet_entity, bullet_transform, bullet) in &bullet_query {
        let bpos = bullet_transform.translation;
        let mut bullet_hit = false;

        for (solid_entity, solid_transform, brick) in &solid_query {
            if bullet_hits_rect(bpos, solid_transform.translation, TILE_SIZE) {
                bullet_hit = true;
                if brick.is_some() {
                    commands.entity(solid_entity).despawn();
                }
                break;
            }
        }

        if !bullet_hit {
            for (tank_entity, tank_transform) in &tank_query {
                if tank_entity == bullet.owner {
                    continue;
                }
                if bullet_hits_rect(bpos, tank_transform.translation, TILE_SIZE) {
                    bullet_hit = true;
                    commands.entity(tank_entity).despawn();
                    break;
                }
            }
        }

        if bullet_hit {
            commands.entity(bullet_entity).despawn();
        }
    }
}
