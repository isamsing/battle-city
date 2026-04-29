use bevy::prelude::*;
use bevy_ggrs::prelude::*;

/// Local-only player with keybinds (used in single-player / local 2P)
#[derive(Component)]
pub struct LocalPlayer {
    pub up: KeyCode,
    pub down: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,
    pub sprite_path: &'static str,
}

/// Networked player with GGRS handle
#[derive(Component)]
#[require(Rollback)]
pub struct NetworkPlayer {
    pub handle: usize,
    pub sprite_path: &'static str,
}
