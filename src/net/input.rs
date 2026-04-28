use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_ggrs::prelude::*;
use bevy_ggrs::{LocalInputs, LocalPlayers};
use bevy_ggrs::ggrs::PlayerHandle;
use serde::{Deserialize, Serialize};

pub const INPUT_UP: u8 = 1 << 0;
pub const INPUT_DOWN: u8 = 1 << 1;
pub const INPUT_LEFT: u8 = 1 << 2;
pub const INPUT_RIGHT: u8 = 1 << 3;
pub const INPUT_FIRE: u8 = 1 << 4;
pub const INPUT_ABILITY: u8 = 1 << 5;

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Debug)]
pub struct PlayerInput(pub u8);

pub type BattleCityConfig = GgrsConfig<PlayerInput, matchbox_socket::PeerId>;

pub fn read_local_inputs(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    local_players: Res<LocalPlayers>,
) {
    let mut local_inputs = HashMap::new();

    for handle in &local_players.0 {
        let mut flags = 0u8;
        if keyboard.pressed(KeyCode::ArrowUp) {
            flags |= INPUT_UP;
        }
        if keyboard.pressed(KeyCode::ArrowDown) {
            flags |= INPUT_DOWN;
        }
        if keyboard.pressed(KeyCode::ArrowLeft) {
            flags |= INPUT_LEFT;
        }
        if keyboard.pressed(KeyCode::ArrowRight) {
            flags |= INPUT_RIGHT;
        }
        if keyboard.pressed(KeyCode::Space) {
            flags |= INPUT_FIRE;
        }
        if keyboard.pressed(KeyCode::ShiftLeft) {
            flags |= INPUT_ABILITY;
        }
        local_inputs.insert(*handle, PlayerInput(flags));
    }

    commands.insert_resource(LocalInputs::<BattleCityConfig>(local_inputs));
}
