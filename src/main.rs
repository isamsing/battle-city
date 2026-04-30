use bevy::prelude::*;
use bevy::asset::AssetMetaCheck;

use battle_city::core::GameCorePlugin;
use battle_city::net::NetPlugin;
use battle_city::scenes::ScenesPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Battle City Reimagined".to_string(),
                resolution: (768u32, 720u32).into(),
                resizable: false,
                canvas: Some("#bevy-canvas".to_string()),
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }).set(ImagePlugin::default_nearest())
          .set(AssetPlugin {
            meta_check: AssetMetaCheck::Never,
            ..default()
        }))
        .add_plugins(GameCorePlugin)
        .add_plugins(NetPlugin)
        .add_plugins(ScenesPlugin)
        .run();
}
