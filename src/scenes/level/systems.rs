use bevy::prelude::*;
use bevy_ggrs::prelude::*;

use crate::core::config::{TILE_SIZE, MAP_WIDTH, MAP_HEIGHT};
use crate::net::input::{BattleCityConfig, INPUT_UP, INPUT_DOWN, INPUT_LEFT, INPUT_RIGHT, INPUT_FIRE};
use super::tank::*;
use super::bullet::*;
use super::player::{LocalPlayer, NetworkPlayer};
use super::map::{Solid, BrickTile};

// --- Collision helpers ---

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

fn bullet_out_of_bounds(pos: Vec3) -> bool {
    let half_map_w = MAP_WIDTH as f32 * TILE_SIZE / 2.0;
    let half_map_h = MAP_HEIGHT as f32 * TILE_SIZE / 2.0;
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

pub fn spawn_bullet(commands: &mut Commands, asset_server: &AssetServer, pos: Vec3, direction: Direction, owner: Entity) {
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

// --- Shared bullet collision (identical for local and networked) ---

pub fn bullet_collision(
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

// --- Local movement (single-player, delta-time based) ---

pub fn local_player_movement(
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

pub fn local_animate_tank(
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

pub fn local_fire_bullet(
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

pub fn move_bullets_local(
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

// --- Networked movement (deterministic, fixed timestep, runs in GgrsSchedule) ---

pub fn networked_player_movement(
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

pub fn networked_animate_tank(
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

pub fn networked_fire_bullet(
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

pub fn move_bullets_networked(
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
