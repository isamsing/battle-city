use bevy::prelude::*;

use crate::core::states::{GameState, MenuScreen};

pub struct StartMenuPlugin;

impl Plugin for StartMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(MenuScreen::Title), setup_title)
            .add_systems(
                Update,
                (handle_title_input, update_cursor).run_if(in_state(MenuScreen::Title)),
            )
            .add_systems(OnExit(MenuScreen::Title), cleanup_menu);
    }
}

// --- Components & Resources ---

#[derive(Component)]
struct MenuEntity;

#[derive(Component)]
struct CursorTank;

#[derive(Resource)]
struct SelectedOption(usize);

const OPTION_COUNT: usize = 2;

fn cleanup_menu(mut commands: Commands, query: Query<Entity, With<MenuEntity>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

// --- Brick Letter Bitmaps (5 wide × 7 tall) ---
// Each row is a u8 where bits 4..0 represent columns left-to-right

fn get_letter_bitmap(ch: char) -> Option<[u8; 7]> {
    match ch {
        'B' => Some([
            0b11110,
            0b10001,
            0b10001,
            0b11110,
            0b10001,
            0b10001,
            0b11110,
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
        'T' => Some([
            0b11111,
            0b00100,
            0b00100,
            0b00100,
            0b00100,
            0b00100,
            0b00100,
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
        'E' => Some([
            0b11111,
            0b10000,
            0b10000,
            0b11110,
            0b10000,
            0b10000,
            0b11111,
        ]),
        'C' => Some([
            0b01110,
            0b10001,
            0b10000,
            0b10000,
            0b10000,
            0b10001,
            0b01110,
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
        'Y' => Some([
            0b10001,
            0b10001,
            0b01010,
            0b00100,
            0b00100,
            0b00100,
            0b00100,
        ]),
        ' ' => None, // space = no bricks
        _ => None,
    }
}

// --- Title Screen Setup ---

const CELL_SIZE: f32 = 6.0;
const LETTER_WIDTH: usize = 5;
const LETTER_HEIGHT: usize = 7;
const LETTER_GAP: usize = 1; // cells between letters

const TITLE_LINE1_Y: f32 = 160.0;  // "BATTLE"
const TITLE_LINE2_Y: f32 = 100.0;  // "CITY"
const MENU_Y_START: f32 = -40.0;
const MENU_Y_SPACING: f32 = 40.0;
const MENU_TEXT_X: f32 = 0.0;       // menu text centered
const CURSOR_X: f32 = -120.0;       // tank well to the left of text

fn setup_title(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(SelectedOption(0));
    commands.spawn((Camera2d, MenuEntity));

    let brick_texture = asset_server.load("sprites/tiles/brick_full.png");

    // --- Score / HI line at top ---
    commands.spawn((
        Text2d::new("I-    00  HI- 20000"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_translation(Vec3::new(0.0, 280.0, 0.0)),
        MenuEntity,
    ));

    // --- Brick-tile "BATTLE" (line 1) and "CITY" (line 2) ---
    spawn_brick_word(&mut commands, &brick_texture, "BATTLE", TITLE_LINE1_Y);
    spawn_brick_word(&mut commands, &brick_texture, "CITY", TITLE_LINE2_Y);

    // --- Menu options (text to the right of cursor tank) ---
    let options = ["SinglePlayer", "MultiPlayer"];
    for (i, label) in options.iter().enumerate() {
        let y = MENU_Y_START - i as f32 * MENU_Y_SPACING;
        commands.spawn((
            Text2d::new(*label),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_translation(Vec3::new(MENU_TEXT_X, y, 0.0)),
            MenuEntity,
        ));
    }

    // --- Tank cursor (to the left of text) ---
    let cursor_texture = asset_server.load("sprites/tanks/player1/level1/right_f0.png");
    commands.spawn((
        Sprite {
            image: cursor_texture,
            custom_size: Some(Vec2::new(28.0, 28.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(CURSOR_X, MENU_Y_START, 0.0)),
        CursorTank,
        MenuEntity,
    ));

    // --- Credits at bottom ---
    commands.spawn((
        Text2d::new("© 2026 D&D GAMES LTD."),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_translation(Vec3::new(0.0, -200.0, 0.0)),
        MenuEntity,
    ));

    commands.spawn((
        Text2d::new("ALL RIGHTS RESERVED"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_translation(Vec3::new(0.0, -230.0, 0.0)),
        MenuEntity,
    ));
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
                            MenuEntity,
                        ));
                    }
                }
            }
            cursor_x += LETTER_WIDTH + LETTER_GAP;
        }
    }
}

// --- Input & Cursor ---

fn handle_title_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selected: ResMut<SelectedOption>,
    mut game_state: ResMut<NextState<GameState>>,
    mut menu_state: ResMut<NextState<MenuScreen>>,
) {
    if keyboard.just_pressed(KeyCode::ArrowUp) && selected.0 > 0 {
        selected.0 -= 1;
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) && selected.0 < OPTION_COUNT - 1 {
        selected.0 += 1;
    }
    if keyboard.just_pressed(KeyCode::Enter) {
        match selected.0 {
            0 => game_state.set(GameState::InGame),
            1 => menu_state.set(MenuScreen::Lobby),
            _ => {}
        }
    }
}

fn update_cursor(selected: Res<SelectedOption>, mut query: Query<&mut Transform, With<CursorTank>>) {
    for mut transform in &mut query {
        transform.translation.y = MENU_Y_START - selected.0 as f32 * MENU_Y_SPACING;
    }
}
