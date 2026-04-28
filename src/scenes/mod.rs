mod start_menu;
mod level_1;

use bevy::prelude::*;

pub struct ScenesPlugin;

impl Plugin for ScenesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            start_menu::StartMenuPlugin,
            level_1::LevelPlugin,
        ));
    }
}
