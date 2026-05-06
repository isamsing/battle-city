mod start_menu;
mod lobby;
pub(crate) mod level;
pub mod bullet;
pub mod enemy;
pub mod map;
pub mod player;
pub mod tank;

use bevy::prelude::*;

pub use player::components::NetworkPlayer;

pub struct ScenesPlugin;

impl Plugin for ScenesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            start_menu::StartMenuPlugin,
            lobby::LobbyPlugin,
            level::LevelPlugin,
        ));
    }
}
