use bevy::prelude::*;

use crate::core::states::GameState;

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), setup_level1);
    }
}

fn setup_level1(mut commands: Commands) {
    commands.spawn(Camera2d);
}
