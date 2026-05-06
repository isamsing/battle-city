pub(crate) mod systems;

use bevy::prelude::*;
use bevy_ggrs::prelude::*;

use crate::core::states::GameState;
use crate::net::is_networked;

use super::bullet::components::{Bullet, FireCooldown};
use super::bullet::systems::*;
use super::player::systems::*;
use super::tank::components::{TankAnimation, TankState, SpawnAnimation};
use systems::{setup_level, spawn_background_music, show_game_over, local_spawn_animation, networked_spawn_animation, cleanup_level, cleanup_network_session, handle_escape_to_menu};

fn reset_clear_color(mut commands: Commands) {
    commands.insert_resource(ClearColor(Color::BLACK));
}

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), setup_level)
            .add_systems(OnExit(GameState::InGame), (reset_clear_color, cleanup_level, cleanup_network_session))
            .add_systems(
                Update,
                spawn_background_music.run_if(in_state(GameState::InGame)),
            )
            // Escape to return to menu
            .add_systems(
                Update,
                handle_escape_to_menu
                    .run_if(in_state(GameState::InGame)),
            )
            // Local-only systems (no networking)
            .add_systems(
                Update,
                (local_spawn_animation, local_player_movement, local_animate_tank, local_fire_bullet,
                 move_bullets_local, bullet_collision)
                    .chain()
                    .run_if(in_state(GameState::InGame))
                    .run_if(not(is_networked)),
            )
            // Networked deterministic systems (runs in GgrsSchedule)
            .add_systems(
                GgrsSchedule,
                (networked_spawn_animation, networked_player_movement, networked_fire_bullet,
                 move_bullets_networked, bullet_collision)
                    .chain()
                    .run_if(is_networked),
            )
            // Networked visual-only animation (runs in normal Update)
            .add_systems(
                Update,
                networked_animate_tank
                    .run_if(in_state(GameState::InGame))
                    .run_if(is_networked),
            )
            // Game over screen
            .add_systems(OnEnter(GameState::GameOver), show_game_over)
            // Register rollback components
            .rollback_component_with_clone::<Transform>()
            .rollback_component_with_clone::<TankAnimation>()
            .rollback_component_with_clone::<TankState>()
            .rollback_component_with_clone::<SpawnAnimation>()
            .rollback_component_with_clone::<Bullet>()
            .rollback_component_with_clone::<FireCooldown>();
    }
}
