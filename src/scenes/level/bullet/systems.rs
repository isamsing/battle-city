use bevy::prelude::*;
use bevy_ggrs::prelude::*;

use crate::core::config::{TILE_SIZE, MAP_WIDTH, MAP_HEIGHT};
use crate::net::input::{BattleCityConfig, INPUT_FIRE};
use crate::scenes::level::tank::components::*;
use crate::scenes::level::player::components::{LocalPlayer, NetworkPlayer};
use crate::scenes::level::map::components::{Solid, BrickTile};
use super::components::*;

// --- Helpers ---

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

// --- Shared bullet collision ---

pub fn bullet_collision(
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

// --- Local bullet systems ---

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

// --- Networked bullet systems ---

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
