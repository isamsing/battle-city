use bevy::prelude::*;

use crate::core::states::MenuScreen;
use crate::net::session::{generate_room_code, start_matchbox_socket, ServerUrl};
use crate::net::GameMode;

pub struct LobbyPlugin;

impl Plugin for LobbyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(MenuScreen::Lobby), setup_lobby)
            .add_systems(
                Update,
                (handle_lobby_input, update_lobby_ui)
                    .run_if(in_state(MenuScreen::Lobby)),
            )
            .add_systems(OnExit(MenuScreen::Lobby), cleanup_lobby);
    }
}

#[derive(Component)]
struct LobbyEntity;

#[derive(Component)]
struct StatusText;

#[derive(Resource)]
struct LobbyState {
    phase: LobbyPhase,
    room_code: String,
    input_buffer: String,
}

#[derive(PartialEq)]
enum LobbyPhase {
    Choosing,
    WaitingAsHost,
    EnteringCode,
    EnteringUrl,
    Connecting,
}

fn setup_lobby(mut commands: Commands) {
    commands.insert_resource(LobbyState {
        phase: LobbyPhase::Choosing,
        room_code: String::new(),
        input_buffer: String::new(),
    });

    commands.spawn((Camera2d, LobbyEntity));

    // Instructions text
    commands.spawn((
        Text::new("H - HOST GAME\nJ - JOIN GAME\nS - SET SERVER\nESC - BACK"),
        TextFont {
            font_size: 28.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(30.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        LobbyEntity,
    ));

    // Status text (shows room code or connection status)
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::srgb(0.5, 1.0, 0.5)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(55.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        StatusText,
        LobbyEntity,
    ));
}

const URL_KEYS: &[(KeyCode, char, char)] = &[
    (KeyCode::KeyA, 'a', 'A'), (KeyCode::KeyB, 'b', 'B'), (KeyCode::KeyC, 'c', 'C'),
    (KeyCode::KeyD, 'd', 'D'), (KeyCode::KeyE, 'e', 'E'), (KeyCode::KeyF, 'f', 'F'),
    (KeyCode::KeyG, 'g', 'G'), (KeyCode::KeyH, 'h', 'H'), (KeyCode::KeyI, 'i', 'I'),
    (KeyCode::KeyJ, 'j', 'J'), (KeyCode::KeyK, 'k', 'K'), (KeyCode::KeyL, 'l', 'L'),
    (KeyCode::KeyM, 'm', 'M'), (KeyCode::KeyN, 'n', 'N'), (KeyCode::KeyO, 'o', 'O'),
    (KeyCode::KeyP, 'p', 'P'), (KeyCode::KeyQ, 'q', 'Q'), (KeyCode::KeyR, 'r', 'R'),
    (KeyCode::KeyS, 's', 'S'), (KeyCode::KeyT, 't', 'T'), (KeyCode::KeyU, 'u', 'U'),
    (KeyCode::KeyV, 'v', 'V'), (KeyCode::KeyW, 'w', 'W'), (KeyCode::KeyX, 'x', 'X'),
    (KeyCode::KeyY, 'y', 'Y'), (KeyCode::KeyZ, 'z', 'Z'),
];

const ROOM_CODE_KEYS: &[(KeyCode, char)] = &[
    (KeyCode::KeyA, 'A'), (KeyCode::KeyB, 'B'), (KeyCode::KeyC, 'C'),
    (KeyCode::KeyD, 'D'), (KeyCode::KeyE, 'E'), (KeyCode::KeyF, 'F'),
    (KeyCode::KeyG, 'G'), (KeyCode::KeyH, 'H'), (KeyCode::KeyI, 'I'),
    (KeyCode::KeyJ, 'J'), (KeyCode::KeyK, 'K'), (KeyCode::KeyL, 'L'),
    (KeyCode::KeyM, 'M'), (KeyCode::KeyN, 'N'), (KeyCode::KeyO, 'O'),
    (KeyCode::KeyP, 'P'), (KeyCode::KeyQ, 'Q'), (KeyCode::KeyR, 'R'),
    (KeyCode::KeyS, 'S'), (KeyCode::KeyT, 'T'), (KeyCode::KeyU, 'U'),
    (KeyCode::KeyV, 'V'), (KeyCode::KeyW, 'W'), (KeyCode::KeyX, 'X'),
    (KeyCode::KeyY, 'Y'), (KeyCode::KeyZ, 'Z'),
    (KeyCode::Digit0, '0'), (KeyCode::Digit1, '1'), (KeyCode::Digit2, '2'),
    (KeyCode::Digit3, '3'), (KeyCode::Digit4, '4'), (KeyCode::Digit5, '5'),
    (KeyCode::Digit6, '6'), (KeyCode::Digit7, '7'), (KeyCode::Digit8, '8'),
    (KeyCode::Digit9, '9'),
];

fn handle_lobby_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut lobby: ResMut<LobbyState>,
    mut game_mode: ResMut<GameMode>,
    mut menu_state: ResMut<NextState<MenuScreen>>,
    mut server_url: ResMut<ServerUrl>,
) {
    // ESC: go back (or cancel current input phase)
    if keyboard.just_pressed(KeyCode::Escape) {
        match lobby.phase {
            LobbyPhase::EnteringCode | LobbyPhase::EnteringUrl => {
                lobby.phase = LobbyPhase::Choosing;
                lobby.input_buffer.clear();
                return;
            }
            _ => {
                *game_mode = GameMode::Local;
                menu_state.set(MenuScreen::ModeSelect);
                return;
            }
        }
    }

    match lobby.phase {
        LobbyPhase::Choosing => {
            if keyboard.just_pressed(KeyCode::KeyH) {
                let code = generate_room_code();
                lobby.room_code = code.clone();
                lobby.phase = LobbyPhase::WaitingAsHost;
                *game_mode = GameMode::OnlineHost(code);
                start_matchbox_socket(commands, game_mode.into(), server_url.into());
                return;
            }
            if keyboard.just_pressed(KeyCode::KeyJ) {
                lobby.phase = LobbyPhase::EnteringCode;
                lobby.input_buffer.clear();
            }
            if keyboard.just_pressed(KeyCode::KeyS) {
                lobby.phase = LobbyPhase::EnteringUrl;
                lobby.input_buffer = server_url.0.clone();
            }
        }
        LobbyPhase::EnteringUrl => {
            let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
            for &(code, lower, upper) in URL_KEYS {
                if keyboard.just_pressed(code) {
                    lobby.input_buffer.push(if shift { upper } else { lower });
                }
            }
            // Digits
            for digit in 0..10u8 {
                let code = match digit {
                    0 => KeyCode::Digit0, 1 => KeyCode::Digit1, 2 => KeyCode::Digit2,
                    3 => KeyCode::Digit3, 4 => KeyCode::Digit4, 5 => KeyCode::Digit5,
                    6 => KeyCode::Digit6, 7 => KeyCode::Digit7, 8 => KeyCode::Digit8,
                    _ => KeyCode::Digit9,
                };
                if keyboard.just_pressed(code) {
                    lobby.input_buffer.push((b'0' + digit) as char);
                }
            }
            // Special characters for URLs
            if keyboard.just_pressed(KeyCode::Period) {
                lobby.input_buffer.push('.');
            }
            if keyboard.just_pressed(KeyCode::Slash) {
                lobby.input_buffer.push('/');
            }
            if keyboard.just_pressed(KeyCode::Minus) {
                lobby.input_buffer.push(if shift { '_' } else { '-' });
            }
            if keyboard.just_pressed(KeyCode::Semicolon) {
                if shift {
                    lobby.input_buffer.push(':');
                }
            }
            if keyboard.just_pressed(KeyCode::Backspace) {
                lobby.input_buffer.pop();
            }
            if keyboard.just_pressed(KeyCode::Enter) && !lobby.input_buffer.is_empty() {
                server_url.0 = lobby.input_buffer.clone();
                lobby.input_buffer.clear();
                lobby.phase = LobbyPhase::Choosing;
            }
        }
        LobbyPhase::EnteringCode => {
            for &(code, ch) in ROOM_CODE_KEYS {
                if keyboard.just_pressed(code) && lobby.input_buffer.len() < 4 {
                    lobby.input_buffer.push(ch);
                }
            }
            if keyboard.just_pressed(KeyCode::Backspace) {
                lobby.input_buffer.pop();
            }
            if keyboard.just_pressed(KeyCode::Enter) && lobby.input_buffer.len() == 4 {
                let code = lobby.input_buffer.clone();
                lobby.room_code = code.clone();
                lobby.phase = LobbyPhase::Connecting;
                *game_mode = GameMode::OnlineJoin(code);
                start_matchbox_socket(commands, game_mode.into(), server_url.into());
                return;
            }
        }
        LobbyPhase::WaitingAsHost | LobbyPhase::Connecting => {
            // Waiting for peer — handled by NetPlugin's wait_for_peers system
        }
    }
}

fn update_lobby_ui(
    lobby: Res<LobbyState>,
    server_url: Res<ServerUrl>,
    mut query: Query<&mut Text, With<StatusText>>,
) {
    for mut text in &mut query {
        let status = match &lobby.phase {
            LobbyPhase::Choosing => {
                format!("SERVER: {}", server_url.0)
            }
            LobbyPhase::WaitingAsHost => {
                format!("ROOM CODE: {}\nWAITING FOR OPPONENT...", lobby.room_code)
            }
            LobbyPhase::EnteringCode => {
                let remaining = 4 - lobby.input_buffer.len();
                format!("ENTER CODE: {}{}", lobby.input_buffer, "_".repeat(remaining))
            }
            LobbyPhase::EnteringUrl => {
                format!("SERVER URL: {}_\n\nENTER TO CONFIRM | ESC TO CANCEL", lobby.input_buffer)
            }
            LobbyPhase::Connecting => "CONNECTING...".to_string(),
        };
        **text = status;
    }
}

fn cleanup_lobby(mut commands: Commands, query: Query<Entity, With<LobbyEntity>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<LobbyState>();
}
