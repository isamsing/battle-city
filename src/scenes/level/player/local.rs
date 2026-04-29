use bevy::prelude::*;

/// Local-only player with keybinds (used in single-player / local 2P)
#[derive(Component)]
pub struct LocalPlayer {
    pub up: KeyCode,
    pub down: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,
    pub sprite_path: &'static str,
}
