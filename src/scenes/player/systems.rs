use bevy::prelude::*;
use bevy_ggrs::prelude::*;

use crate::net::input::{BattleCityConfig, INPUT_UP, INPUT_DOWN, INPUT_LEFT, INPUT_RIGHT};
use crate::scenes::enemy::components::EnemyTank;
use crate::scenes::tank::components::*;
use crate::scenes::map::components::Solid;
use super::components::{LocalPlayer, NetworkPlayer};

// --- Local movement (single-player, delta-time based) ---

pub fn local_player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut player_query: Query<(Entity, &mut Transform, &mut TankAnimation, &LocalPlayer, &TankState), With<Tank>>,
    solid_query: Query<&Transform, (With<Solid>, Without<Tank>)>,
    enemy_query: Query<&Transform, (With<EnemyTank>, Without<LocalPlayer>, Without<Solid>)>,
) {
    let solid_positions: Vec<Vec3> = solid_query.iter().map(|t| t.translation).collect();
    let all_tanks: Vec<(Entity, Vec3)> = player_query.iter().map(|(e, t, _, _, _)| (e, t.translation)).collect();
    let enemy_positions: Vec<Vec3> = enemy_query.iter().map(|t| t.translation).collect();

    for (entity, mut transform, mut anim, player, tank_state) in &mut player_query {
        if *tank_state != TankState::Active {
            continue;
        }
        let mut obstacles = solid_positions.clone();
        obstacles.extend_from_slice(&enemy_positions);
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
            if !is_blocked(new_pos, &obstacles) {
                transform.translation = new_pos;
            }
            anim.direction = Direction::Up;
            moving = true;
        } else if keyboard.pressed(player.down) {
            let mut new_pos = transform.translation;
            new_pos.y -= delta;
            if !is_blocked(new_pos, &obstacles) {
                transform.translation = new_pos;
            }
            anim.direction = Direction::Down;
            moving = true;
        } else if keyboard.pressed(player.left) {
            let mut new_pos = transform.translation;
            new_pos.x -= delta;
            if !is_blocked(new_pos, &obstacles) {
                transform.translation = new_pos;
            }
            anim.direction = Direction::Left;
            moving = true;
        } else if keyboard.pressed(player.right) {
            let mut new_pos = transform.translation;
            new_pos.x += delta;
            if !is_blocked(new_pos, &obstacles) {
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
    mut query: Query<(&mut TankAnimation, &mut Sprite, &LocalPlayer, &TankState)>,
) {
    for (mut anim, mut sprite, player, tank_state) in &mut query {
        if *tank_state != TankState::Active {
            continue;
        }
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

pub fn networked_player_movement(
    inputs: Res<PlayerInputs<BattleCityConfig>>,
    mut player_query: Query<(Entity, &mut Transform, &mut TankAnimation, &NetworkPlayer, &TankState), With<Tank>>,
    solid_query: Query<&Transform, (With<Solid>, Without<Tank>)>,
    enemy_query: Query<&Transform, (With<EnemyTank>, Without<NetworkPlayer>, Without<Solid>)>,
) {
    let solid_positions: Vec<Vec3> = solid_query.iter().map(|t| t.translation).collect();
    let all_tanks: Vec<(Entity, Vec3)> = player_query.iter().map(|(e, t, _, _, _)| (e, t.translation)).collect();
    let enemy_positions: Vec<Vec3> = enemy_query.iter().map(|t| t.translation).collect();

    for (entity, mut transform, mut anim, net_player, tank_state) in &mut player_query {
        if *tank_state != TankState::Active {
            continue;
        }
        let mut obstacles = solid_positions.clone();
        obstacles.extend_from_slice(&enemy_positions);
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
            if !is_blocked(new_pos, &obstacles) {
                transform.translation = new_pos;
            }
            anim.direction = Direction::Up;
            moving = true;
        } else if flags & INPUT_DOWN != 0 {
            let mut new_pos = transform.translation;
            new_pos.y -= TANK_SPEED_PER_FRAME;
            if !is_blocked(new_pos, &obstacles) {
                transform.translation = new_pos;
            }
            anim.direction = Direction::Down;
            moving = true;
        } else if flags & INPUT_LEFT != 0 {
            let mut new_pos = transform.translation;
            new_pos.x -= TANK_SPEED_PER_FRAME;
            if !is_blocked(new_pos, &obstacles) {
                transform.translation = new_pos;
            }
            anim.direction = Direction::Left;
            moving = true;
        } else if flags & INPUT_RIGHT != 0 {
            let mut new_pos = transform.translation;
            new_pos.x += TANK_SPEED_PER_FRAME;
            if !is_blocked(new_pos, &obstacles) {
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
    mut query: Query<(&TankAnimation, &mut Sprite, &NetworkPlayer, &TankState)>,
) {
    for (anim, mut sprite, net_player, tank_state) in &mut query {
        if *tank_state != TankState::Active {
            continue;
        }
        let path = format!(
            "{}/{}_f{}.png",
            net_player.sprite_path,
            anim.direction.as_str(),
            anim.frame
        );
        sprite.image = asset_server.load(path);
    }
}
