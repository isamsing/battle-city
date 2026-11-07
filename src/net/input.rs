//! Local input collection: keyboard + gamepad -> one byte per frame for GGRS.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_ggrs::prelude::*;
use bevy_ggrs::{LocalInputs, LocalPlayers};
use serde::{Deserialize, Serialize};

pub use crate::sim::{IN_ABILITY as INPUT_ABILITY, IN_DOWN as INPUT_DOWN, IN_FIRE as INPUT_FIRE, IN_LEFT as INPUT_LEFT, IN_RIGHT as INPUT_RIGHT, IN_UP as INPUT_UP};

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Debug)]
pub struct PlayerInput(pub u8);

pub type BattleCityConfig = GgrsConfig<PlayerInput, matchbox_socket::PeerId>;

const STICK_DEADZONE: f32 = 0.5;

/// Collect the local player's input bits. Sends neutral input while the
/// browser tab/window is unfocused so a tank never keeps driving on its own.
pub fn read_local_inputs(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    windows: Query<&Window, With<PrimaryWindow>>,
    local_players: Res<LocalPlayers>,
) {
    let focused = windows.single().map(|w| w.focused).unwrap_or(true);
    let flags = if focused { collect_bits(&keyboard, &gamepads) } else { 0 };

    let mut local_inputs = HashMap::new();
    for handle in &local_players.0 {
        local_inputs.insert(*handle, PlayerInput(flags));
    }
    commands.insert_resource(LocalInputs::<BattleCityConfig>(local_inputs));
}

fn collect_bits(keyboard: &ButtonInput<KeyCode>, gamepads: &Query<&Gamepad>) -> u8 {
    let mut flags = 0u8;
    if keyboard.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
        flags |= INPUT_UP;
    }
    if keyboard.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
        flags |= INPUT_DOWN;
    }
    if keyboard.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) {
        flags |= INPUT_LEFT;
    }
    if keyboard.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) {
        flags |= INPUT_RIGHT;
    }
    if keyboard.pressed(KeyCode::Space) {
        flags |= INPUT_FIRE;
    }
    if keyboard.pressed(KeyCode::ShiftLeft) {
        flags |= INPUT_ABILITY;
    }
    for pad in gamepads.iter() {
        flags |= gamepad_bits(pad);
    }
    flags
}

/// Map a gamepad (d-pad or left stick, face buttons) to input bits.
pub fn gamepad_bits(pad: &Gamepad) -> u8 {
    let mut flags = 0u8;
    let stick = pad.left_stick();
    let dpad = pad.dpad();
    let v = Vec2::new(stick.x + dpad.x, stick.y + dpad.y);
    // One direction at a time, strongest axis wins (tanks never move diagonally).
    if v.x.abs() >= v.y.abs() {
        if v.x > STICK_DEADZONE {
            flags |= INPUT_RIGHT;
        } else if v.x < -STICK_DEADZONE {
            flags |= INPUT_LEFT;
        }
    } else if v.y > STICK_DEADZONE {
        flags |= INPUT_UP;
    } else if v.y < -STICK_DEADZONE {
        flags |= INPUT_DOWN;
    }
    if pad.pressed(GamepadButton::South) || pad.pressed(GamepadButton::West) {
        flags |= INPUT_FIRE;
    }
    if pad.pressed(GamepadButton::East) || pad.pressed(GamepadButton::North) {
        flags |= INPUT_ABILITY;
    }
    flags
}
