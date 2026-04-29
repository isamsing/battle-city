use bevy::prelude::*;
use bevy_ggrs::prelude::*;

/// Networked player with GGRS handle
#[derive(Component)]
#[require(Rollback)]
pub struct NetworkPlayer {
    pub handle: usize,
    pub sprite_path: &'static str,
}
