use bevy::prelude::*;

mod core;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Battle City Reimagined".to_string(),
                resolution: (768u32, 720u32).into(),
                resizable: false,
                ..default()
            }),
            ..default()
        }).set(ImagePlugin::default_nearest()))
        .add_plugins(crate::core::GameCorePlugin)
        .run();
}
