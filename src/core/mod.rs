pub mod config;
pub mod grid;
pub mod states;

use bevy::prelude::*;

pub struct GameCorePlugin;

impl Plugin for GameCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<states::GameState>();
    }
}
