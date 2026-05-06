use bevy::prelude::*;

use crate::core::config::TILE_SIZE;
use crate::scenes::bullet::components::{BulletTeam, FireCooldown};
use crate::scenes::bullet::systems::spawn_bullet;
use crate::scenes::level::systems::LevelEntity;
use crate::scenes::map::components::Solid;
use crate::scenes::map::systems::tile_position;
use crate::scenes::player::components::{LocalPlayer, NetworkPlayer};
use crate::scenes::tank::components::*;

use super::components::*;

// --- Deterministic RNG ---

fn xorshift32(state: &mut u32) -> u32 {
    let mut x = *state;
    if x == 0 { x = 1; }
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    x
}

fn random_direction(rng: &mut u32) -> Direction {
    match xorshift32(rng) % 4 {
        0 => Direction::Up,
        1 => Direction::Down,
        2 => Direction::Left,
        _ => Direction::Right,
    }
}

// --- Spawn System ---

fn spawn_enemy(
    commands: &mut Commands,
    asset_server: &AssetServer,
    col: i32,
    row: i32,
    enemy_type: EnemyType,
    seed: u32,
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
            direction: Direction::Down,
        },
        Tank,
        TankState::Spawning,
        spawn_anim,
        EnemyTank {
            enemy_type,
            hp: enemy_type.max_hp(),
        },
        EnemyAI::new(enemy_type, seed),
        FireCooldown::new(),
        LevelEntity,
    ));
}

const SPAWN_POSITIONS: [(i32, i32); 3] = [(0, 0), (12, 0), (24, 0)];

pub fn local_enemy_spawn_wave(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut spawn_state: ResMut<EnemySpawnState>,
    enemy_query: Query<&TankState, With<EnemyTank>>,
    all_tanks: Query<&Transform, With<Tank>>,
) {
    if spawn_state.spawn_queue.is_empty() {
        return;
    }

    spawn_state.spawn_timer.tick(time.delta());
    if !spawn_state.spawn_timer.just_finished() {
        return;
    }

    let active_count = enemy_query.iter().filter(|s| **s != TankState::Exploding).count();
    if active_count >= spawn_state.max_active {
        return;
    }

    let (col, row) = SPAWN_POSITIONS[spawn_state.next_spawn_slot % 3];
    let spawn_pos = tile_position(col, row);

    // Check spawn position is clear
    let blocked = all_tanks.iter().any(|t| aabb_overlap(t.translation, spawn_pos, TILE_SIZE));
    if blocked {
        return;
    }

    let enemy_type = spawn_state.spawn_queue.remove(0);
    let seed = (spawn_state.enemies_remaining as u32 * 7919) + 42;
    spawn_state.enemies_remaining = spawn_state.spawn_queue.len();
    spawn_state.next_spawn_slot += 1;

    spawn_enemy(&mut commands, &asset_server, col, row, enemy_type, seed);
}

pub fn networked_enemy_spawn_wave(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    spawn_state: Option<ResMut<EnemySpawnState>>,
    enemy_query: Query<&TankState, With<EnemyTank>>,
    all_tanks: Query<&Transform, With<Tank>>,
) {
    let Some(mut spawn_state) = spawn_state else { return };
    if spawn_state.spawn_queue.is_empty() {
        return;
    }

    spawn_state.spawn_timer.tick(std::time::Duration::from_secs_f32(FIXED_DT));
    if !spawn_state.spawn_timer.just_finished() {
        return;
    }

    let active_count = enemy_query.iter().filter(|s| **s != TankState::Exploding).count();
    if active_count >= spawn_state.max_active {
        return;
    }

    let (col, row) = SPAWN_POSITIONS[spawn_state.next_spawn_slot % 3];
    let spawn_pos = tile_position(col, row);

    let blocked = all_tanks.iter().any(|t| aabb_overlap(t.translation, spawn_pos, TILE_SIZE));
    if blocked {
        return;
    }

    let enemy_type = spawn_state.spawn_queue.remove(0);
    let seed = (spawn_state.enemies_remaining as u32 * 7919) + 42;
    spawn_state.enemies_remaining = spawn_state.spawn_queue.len();
    spawn_state.next_spawn_slot += 1;

    spawn_enemy(&mut commands, &asset_server, col, row, enemy_type, seed);
}

// --- Movement System ---

pub fn local_enemy_movement(
    time: Res<Time>,
    mut enemy_query: Query<(Entity, &mut Transform, &mut TankAnimation, &mut EnemyAI, &EnemyTank, &TankState), With<EnemyTank>>,
    solid_query: Query<&Transform, (With<Solid>, Without<EnemyTank>)>,
    player_query: Query<&Transform, (With<LocalPlayer>, Without<EnemyTank>, Without<Solid>)>,
) {
    // Collect all enemy positions first for inter-tank collision
    let all_enemy_positions: Vec<(Entity, Vec3)> = enemy_query.iter().map(|(e, t, _, _, _, _)| (e, t.translation)).collect();
    let solid_positions: Vec<Vec3> = solid_query.iter().map(|t| t.translation).collect();
    let player_positions: Vec<Vec3> = player_query.iter().map(|t| t.translation).collect();

    for (entity, mut transform, mut anim, mut ai, enemy, tank_state) in &mut enemy_query {
        if *tank_state != TankState::Active {
            continue;
        }

        let speed = enemy.enemy_type.speed();
        let delta = speed * time.delta_secs();

        // Build obstacle list (solids + other enemies + players)
        let mut obstacles = solid_positions.clone();
        obstacles.extend_from_slice(&player_positions);
        for &(other_entity, other_pos) in &all_enemy_positions {
            if other_entity != entity {
                obstacles.push(other_pos);
            }
        }

        // Try to move in current direction
        let vel = anim.direction.to_velocity();
        let mut new_pos = transform.translation;
        new_pos.x += vel.x * delta;
        new_pos.y += vel.y * delta;

        if is_blocked(new_pos, &obstacles) {
            // Blocked — pick a new random direction
            anim.direction = random_direction(&mut ai.rng_state);
        } else {
            transform.translation = new_pos;
        }

        // Periodically consider changing direction
        ai.direction_timer.tick(time.delta());
        if ai.direction_timer.just_finished() {
            if xorshift32(&mut ai.rng_state) % 100 < 30 {
                anim.direction = random_direction(&mut ai.rng_state);
            }
        }
    }
}

pub fn networked_enemy_movement(
    mut enemy_query: Query<(Entity, &mut Transform, &mut TankAnimation, &mut EnemyAI, &EnemyTank, &TankState), With<EnemyTank>>,
    solid_query: Query<&Transform, (With<Solid>, Without<EnemyTank>)>,
    player_query: Query<&Transform, (With<NetworkPlayer>, Without<EnemyTank>, Without<Solid>)>,
) {
    let all_enemy_positions: Vec<(Entity, Vec3)> = enemy_query.iter().map(|(e, t, _, _, _, _)| (e, t.translation)).collect();
    let solid_positions: Vec<Vec3> = solid_query.iter().map(|t| t.translation).collect();
    let player_positions: Vec<Vec3> = player_query.iter().map(|t| t.translation).collect();

    for (entity, mut transform, mut anim, mut ai, enemy, tank_state) in &mut enemy_query {
        if *tank_state != TankState::Active {
            continue;
        }

        let delta = enemy.enemy_type.speed_per_frame();

        let mut obstacles = solid_positions.clone();
        obstacles.extend_from_slice(&player_positions);
        for &(other_entity, other_pos) in &all_enemy_positions {
            if other_entity != entity {
                obstacles.push(other_pos);
            }
        }

        let vel = anim.direction.to_velocity();
        let mut new_pos = transform.translation;
        new_pos.x += vel.x * delta;
        new_pos.y += vel.y * delta;

        if is_blocked(new_pos, &obstacles) {
            anim.direction = random_direction(&mut ai.rng_state);
        } else {
            transform.translation = new_pos;
        }

        ai.direction_timer.tick(std::time::Duration::from_secs_f32(FIXED_DT));
        if ai.direction_timer.just_finished() {
            if xorshift32(&mut ai.rng_state) % 100 < 30 {
                anim.direction = random_direction(&mut ai.rng_state);
            }
        }
    }
}

// --- Fire System ---

pub fn local_enemy_fire(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut query: Query<(Entity, &Transform, &TankAnimation, &mut EnemyAI, &EnemyTank, &mut FireCooldown, &TankState)>,
) {
    for (entity, transform, anim, mut ai, enemy, mut cooldown, tank_state) in &mut query {
        if *tank_state != TankState::Active {
            continue;
        }
        cooldown.timer.tick(time.delta());
        ai.fire_timer.tick(time.delta());

        if ai.fire_timer.just_finished() && cooldown.timer.is_finished() {
            spawn_bullet(
                &mut commands,
                &asset_server,
                transform.translation,
                anim.direction,
                entity,
                BulletTeam::Enemy,
                enemy.enemy_type.bullet_speed(),
            );
            cooldown.timer.reset();
        }
    }
}

pub fn networked_enemy_fire(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut query: Query<(Entity, &Transform, &TankAnimation, &mut EnemyAI, &EnemyTank, &mut FireCooldown, &TankState)>,
) {
    let dt = std::time::Duration::from_secs_f32(FIXED_DT);
    for (entity, transform, anim, mut ai, enemy, mut cooldown, tank_state) in &mut query {
        if *tank_state != TankState::Active {
            continue;
        }
        cooldown.timer.tick(dt);
        ai.fire_timer.tick(dt);

        if ai.fire_timer.just_finished() && cooldown.timer.is_finished() {
            spawn_bullet(
                &mut commands,
                &asset_server,
                transform.translation,
                anim.direction,
                entity,
                BulletTeam::Enemy,
                enemy.enemy_type.bullet_speed(),
            );
            cooldown.timer.reset();
        }
    }
}

// --- Animation System ---

pub fn local_enemy_animate(
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    mut query: Query<(&mut TankAnimation, &mut Sprite, &EnemyTank, &TankState)>,
) {
    for (mut anim, mut sprite, enemy, tank_state) in &mut query {
        if *tank_state != TankState::Active {
            continue;
        }

        // Always animating (enemies are always moving)
        anim.timer.tick(time.delta());
        if anim.timer.just_finished() {
            anim.frame = (anim.frame + 1) % 2;
        }

        let path = format!(
            "{}/{}_f{}.png",
            enemy.enemy_type.sprite_path(),
            anim.direction.as_str(),
            anim.frame
        );
        sprite.image = asset_server.load(path);

        // Armor tank color tinting based on HP
        if enemy.enemy_type == EnemyType::Armor {
            sprite.color = match enemy.hp {
                4 => Color::WHITE,
                3 => Color::srgb(0.5, 1.0, 0.5),
                2 => Color::srgb(1.0, 1.0, 0.3),
                _ => Color::srgb(1.0, 0.3, 0.3),
            };
        }
    }
}

pub fn networked_enemy_animate(
    asset_server: Res<AssetServer>,
    mut query: Query<(&TankAnimation, &mut Sprite, &EnemyTank, &TankState)>,
) {
    for (anim, mut sprite, enemy, tank_state) in &mut query {
        if *tank_state != TankState::Active {
            continue;
        }

        let path = format!(
            "{}/{}_f{}.png",
            enemy.enemy_type.sprite_path(),
            anim.direction.as_str(),
            anim.frame
        );
        sprite.image = asset_server.load(path);

        if enemy.enemy_type == EnemyType::Armor {
            sprite.color = match enemy.hp {
                4 => Color::WHITE,
                3 => Color::srgb(0.5, 1.0, 0.5),
                2 => Color::srgb(1.0, 1.0, 0.3),
                _ => Color::srgb(1.0, 0.3, 0.3),
            };
        }
    }
}
