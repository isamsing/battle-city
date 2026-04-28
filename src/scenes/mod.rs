mod start_menu;
mod level;

use bevy::prelude::*;

pub struct ScenesPlugin;

impl Plugin for ScenesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            start_menu::StartMenuPlugin,
            level::LevelPlugin,
        ));
    }
}
