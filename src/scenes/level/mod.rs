mod systems;

use bevy::prelude::*;
use bevy_ggrs::prelude::*;

use crate::core::states::GameState;
use crate::net::is_networked;

use super::bullet::components::{Bullet, FireCooldown};
use super::bullet::systems::*;
use super::player::systems::*;
use super::tank::components::TankAnimation;
use systems::setup_level;

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), setup_level)
            // Local-only systems (no networking)
            .add_systems(
                Update,
                (local_player_movement, local_animate_tank, local_fire_bullet,
                 move_bullets_local, bullet_collision)
                    .run_if(in_state(GameState::InGame))
                    .run_if(not(is_networked)),
            )
            // Networked deterministic systems (runs in GgrsSchedule)
            .add_systems(
                GgrsSchedule,
                (networked_player_movement, networked_fire_bullet,
                 move_bullets_networked, bullet_collision)
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
