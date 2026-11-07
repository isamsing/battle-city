use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::prelude::*;

use crate::core::config::{PLAYER_NAMES, PLAYER_RGB};
use crate::core::states::MenuScreen;
use crate::net::session::{generate_room_code, start_matchbox_socket, LobbyNet, MatchboxRes, ServerUrl};
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

#[derive(Component)]
struct InstructionText;

/// One line of the player list (slot 0..=3).
#[derive(Component)]
struct RosterLine(usize);

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

// --- Brick Letter Bitmaps (5 wide × 7 tall) ---
const CELL_SIZE: f32 = 5.0;
const LETTER_WIDTH: usize = 5;
const LETTER_HEIGHT: usize = 7;
const LETTER_GAP: usize = 1;
const TITLE_Y: f32 = 220.0;

fn get_letter_bitmap(ch: char) -> Option<[u8; 7]> {
    match ch {
        'M' => Some([
            0b10001,
            0b11011,
            0b10101,
            0b10001,
            0b10001,
            0b10001,
            0b10001,
        ]),
        'U' => Some([
            0b10001,
            0b10001,
            0b10001,
            0b10001,
            0b10001,
            0b10001,
            0b01110,
        ]),
        'L' => Some([
            0b10000,
            0b10000,
            0b10000,
            0b10000,
            0b10000,
            0b10000,
            0b11111,
        ]),
        'T' => Some([
            0b11111,
            0b00100,
            0b00100,
            0b00100,
            0b00100,
            0b00100,
            0b00100,
        ]),
        'I' => Some([
            0b11111,
            0b00100,
            0b00100,
            0b00100,
            0b00100,
            0b00100,
            0b11111,
        ]),
        'P' => Some([
            0b11110,
            0b10001,
            0b10001,
            0b11110,
            0b10000,
            0b10000,
            0b10000,
        ]),
        'A' => Some([
            0b01110,
            0b10001,
            0b10001,
            0b11111,
            0b10001,
            0b10001,
            0b10001,
        ]),
        'Y' => Some([
            0b10001,
            0b10001,
            0b01010,
            0b00100,
            0b00100,
            0b00100,
            0b00100,
        ]),
        'E' => Some([
            0b11111,
            0b10000,
            0b10000,
            0b11110,
            0b10000,
            0b10000,
            0b11111,
        ]),
        'R' => Some([
            0b11110,
            0b10001,
            0b10001,
            0b11110,
            0b10010,
            0b10001,
            0b10001,
        ]),
        _ => None,
    }
}

fn word_width_cells(word: &str) -> usize {
    let letter_count = word.chars().count();
    if letter_count == 0 {
        return 0;
    }
    letter_count * LETTER_WIDTH + (letter_count - 1) * LETTER_GAP
}

fn spawn_brick_word(
    commands: &mut Commands,
    texture: &Handle<Image>,
    word: &str,
    y_top: f32,
) {
    let total_cells = word_width_cells(word);
    let start_x = -(total_cells as f32 * CELL_SIZE) / 2.0;

    let mut cursor_x = 0usize;
    for ch in word.chars() {
        if let Some(bitmap) = get_letter_bitmap(ch) {
            for row in 0..LETTER_HEIGHT {
                for col in 0..LETTER_WIDTH {
                    let bit = (bitmap[row] >> (LETTER_WIDTH - 1 - col)) & 1;
                    if bit == 1 {
                        let x = start_x + (cursor_x + col) as f32 * CELL_SIZE;
                        let y = y_top - row as f32 * CELL_SIZE;
                        commands.spawn((
                            Sprite {
                                image: texture.clone(),
                                custom_size: Some(Vec2::new(CELL_SIZE, CELL_SIZE)),
                                ..default()
                            },
                            Transform::from_translation(Vec3::new(x, y, 0.0)),
                            LobbyEntity,
                        ));
                    }
                }
            }
            cursor_x += LETTER_WIDTH + LETTER_GAP;
        }
    }
}

fn setup_lobby(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(LobbyState {
        phase: LobbyPhase::Choosing,
        room_code: String::new(),
        input_buffer: String::new(),
    });

    commands.spawn((Camera2d, LobbyEntity));

    // --- Brick-tile "MULTIPLAYER" title ---
    let brick_texture = asset_server.load("sprites/tiles/brick_full.png");
    spawn_brick_word(&mut commands, &brick_texture, "MULTIPLAYER", TITLE_Y);

    // Instructions text
    commands.spawn((
        Text2d::new("H - HOST GAME\nJ - JOIN GAME\nS - SET SERVER\nESC - BACK"),
        InstructionText,
        TextFont {
            font_size: 28.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_translation(Vec3::new(0.0, 100.0, 0.0)),
        LobbyEntity,
    ));

    // Status text (shows room code or connection status)
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::srgb(0.5, 1.0, 0.5)),
        Transform::from_translation(Vec3::new(0.0, -40.0, 0.0)),
        StatusText,
        LobbyEntity,
    ));

    // Player list: one line per slot, coloured like the tank.
    for slot in 0..4 {
        let [r, g, b] = PLAYER_RGB[slot];
        commands.spawn((
            Text2d::new(""),
            TextFont { font_size: 22.0, ..default() },
            TextColor(Color::srgb_u8(r, g, b)),
            Transform::from_translation(Vec3::new(0.0, -110.0 - slot as f32 * 30.0, 0.0)),
            RosterLine(slot),
            LobbyEntity,
        ));
    }
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
    mut net: Option<ResMut<LobbyNet>>,
    mut key_events: MessageReader<KeyboardInput>,
    mut game_mode: ResMut<GameMode>,
    mut menu_state: ResMut<NextState<MenuScreen>>,
    mut server_url: ResMut<ServerUrl>,
) {
    // Key presses in the order they happened. Polling `just_pressed` over a key
    // table would reorder two keys that land in the same (slow) frame.
    let typed: Vec<KeyCode> = key_events
        .read()
        .filter(|e| e.state == ButtonState::Pressed)
        .map(|e| e.key_code)
        .collect();

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
                menu_state.set(MenuScreen::Title);
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
                commands.insert_resource(LobbyNet::new(true));
                start_matchbox_socket(&mut commands, &game_mode, &server_url);
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
            for code in &typed {
                if let Some(&(_, lower, upper)) = URL_KEYS.iter().find(|(k, _, _)| k == code) {
                    lobby.input_buffer.push(if shift { upper } else { lower });
                    continue;
                }
                let digit = match code {
                    KeyCode::Digit0 => Some('0'),
                    KeyCode::Digit1 => Some('1'),
                    KeyCode::Digit2 => Some('2'),
                    KeyCode::Digit3 => Some('3'),
                    KeyCode::Digit4 => Some('4'),
                    KeyCode::Digit5 => Some('5'),
                    KeyCode::Digit6 => Some('6'),
                    KeyCode::Digit7 => Some('7'),
                    KeyCode::Digit8 => Some('8'),
                    KeyCode::Digit9 => Some('9'),
                    _ => None,
                };
                if let Some(d) = digit {
                    lobby.input_buffer.push(d);
                    continue;
                }
                match code {
                    KeyCode::Period => lobby.input_buffer.push('.'),
                    KeyCode::Slash => lobby.input_buffer.push('/'),
                    KeyCode::Minus => lobby.input_buffer.push(if shift { '_' } else { '-' }),
                    KeyCode::Semicolon if shift => lobby.input_buffer.push(':'),
                    KeyCode::Backspace => {
                        lobby.input_buffer.pop();
                    }
                    _ => {}
                }
            }
            if keyboard.just_pressed(KeyCode::Enter) && !lobby.input_buffer.is_empty() {
                server_url.0 = lobby.input_buffer.clone();
                lobby.input_buffer.clear();
                lobby.phase = LobbyPhase::Choosing;
            }
        }
        LobbyPhase::EnteringCode => {
            for code in &typed {
                if let Some(&(_, ch)) = ROOM_CODE_KEYS.iter().find(|(k, _)| k == code)
                    && lobby.input_buffer.len() < 4 {
                        lobby.input_buffer.push(ch);
                    }
            }
            if typed.contains(&KeyCode::Backspace) {
                lobby.input_buffer.pop();
            }
            if keyboard.just_pressed(KeyCode::Enter) && lobby.input_buffer.len() == 4 {
                let code = lobby.input_buffer.clone();
                lobby.room_code = code.clone();
                lobby.phase = LobbyPhase::Connecting;
                *game_mode = GameMode::OnlineJoin(code);
                commands.insert_resource(LobbyNet::new(false));
                start_matchbox_socket(&mut commands, &game_mode, &server_url);
            }
        }
        LobbyPhase::WaitingAsHost => {
            // Host controls: 1-4 change the player count, Enter starts the match.
            if let Some(net) = net.as_deref_mut() {
                for (key, n) in [(KeyCode::Digit1, 1u8), (KeyCode::Digit2, 2), (KeyCode::Digit3, 3), (KeyCode::Digit4, 4)] {
                    if keyboard.just_pressed(key) {
                        net.set_max_players(n);
                    }
                }
                if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::KeyG) {
                    net.start_requested = true;
                }
            }
        }
        LobbyPhase::Connecting => {
            // Joiner: R toggles ready.
            if let Some(net) = net.as_deref_mut()
                && (keyboard.just_pressed(KeyCode::KeyR) || keyboard.just_pressed(KeyCode::Enter)) {
                    net.local_ready = !net.local_ready;
                }
        }
    }
}

fn update_lobby_ui(
    lobby: Res<LobbyState>,
    server_url: Res<ServerUrl>,
    net: Option<Res<LobbyNet>>,
    mut status: Query<&mut Text2d, (With<StatusText>, Without<RosterLine>, Without<InstructionText>)>,
    mut roster: Query<(&RosterLine, &mut Text2d), (Without<StatusText>, Without<InstructionText>)>,
    mut help: Query<&mut Text2d, (With<InstructionText>, Without<StatusText>, Without<RosterLine>)>,
) {
    let online = matches!(lobby.phase, LobbyPhase::WaitingAsHost | LobbyPhase::Connecting);
    for mut text in &mut help {
        **text = match (&lobby.phase, net.as_deref()) {
            (LobbyPhase::WaitingAsHost, _) => "1-4 - PLAYER COUNT\nENTER - START (ALL READY)\nESC - LEAVE".to_string(),
            (LobbyPhase::Connecting, _) => "R - READY / NOT READY\nESC - LEAVE".to_string(),
            _ => "H - HOST GAME\nJ - JOIN GAME\nS - SET SERVER\nESC - BACK".to_string(),
        };
    }
    for mut text in &mut status {
        let s = match (&lobby.phase, net.as_deref()) {
            (LobbyPhase::Choosing, _) => format!("SERVER: {}", server_url.0),
            (LobbyPhase::WaitingAsHost, Some(n)) => {
                let hint = if n.can_start() {
                    "PRESS ENTER TO START".to_string()
                } else if n.roster.len() < n.max_players as usize {
                    format!("WAITING FOR {} MORE...", n.max_players as usize - n.roster.len())
                } else {
                    "WAITING FOR READY...".to_string()
                };
                format!("ROOM CODE: {}   PLAYERS: {}\n{}", lobby.room_code, n.max_players, hint)
            }
            (LobbyPhase::WaitingAsHost, None) => format!("ROOM CODE: {}", lobby.room_code),
            (LobbyPhase::EnteringCode, _) => {
                let remaining = 4 - lobby.input_buffer.len();
                format!("ENTER CODE: {}{}", lobby.input_buffer, "_".repeat(remaining))
            }
            (LobbyPhase::EnteringUrl, _) => {
                format!("SERVER URL: {}_\n\nENTER TO CONFIRM | ESC TO CANCEL", lobby.input_buffer)
            }
            (LobbyPhase::Connecting, Some(n)) if n.rejected => "ROOM IS FULL".to_string(),
            (LobbyPhase::Connecting, Some(n)) if n.my_id.is_none() || n.roster.is_empty() => {
                format!("CONNECTING TO {}...", lobby.room_code)
            }
            (LobbyPhase::Connecting, Some(n)) => {
                let me = if n.local_ready { "READY - WAITING FOR HOST" } else { "PRESS R WHEN READY" };
                format!("ROOM {}   {}", lobby.room_code, me)
            }
            (LobbyPhase::Connecting, None) => "CONNECTING...".to_string(),
        };
        **text = s;
    }
    for (line, mut text) in &mut roster {
        let slot = line.0;
        **text = match net.as_deref() {
            Some(n) if online && slot < n.max_players as usize => match n.roster.get(slot) {
                Some(peer) => {
                    let you = if Some(*peer) == n.my_id { " (YOU)" } else { "" };
                    let host = if slot == 0 { " HOST" } else { "" };
                    let ready = if n.is_ready(peer) { "READY" } else { "NOT READY" };
                    format!("P{} {}{}{}  {}", slot + 1, PLAYER_NAMES[slot], host, you, ready)
                }
                None => format!("P{} {}  - EMPTY -", slot + 1, PLAYER_NAMES[slot]),
            },
            _ => String::new(),
        };
    }
}

fn cleanup_lobby(mut commands: Commands, query: Query<Entity, With<LobbyEntity>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<LobbyState>();
    commands.remove_resource::<LobbyNet>();
    // Drop any WebRTC socket still in the lobby. When a match starts, the
    // socket has already been moved into `PendingMatch` and is kept.
    commands.remove_resource::<MatchboxRes>();
}
