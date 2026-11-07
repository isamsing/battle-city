pub const TILE_SIZE: f32 = 24.0;
pub const MAP_WIDTH: u32 = 26;
pub const MAP_HEIGHT: u32 = 26;
pub const VIRTUAL_WIDTH: f32 = 256.0;
pub const VIRTUAL_HEIGHT: f32 = 240.0;
pub const WINDOW_SCALE: u32 = 3;

/// Display names of the four player tanks, by slot.
pub const PLAYER_NAMES: [&str; 4] = ["YELLOW", "GREEN", "BLUE", "RED"];
/// Display colours (sRGB 0-255) of the four player tanks, by slot.
pub const PLAYER_RGB: [[u8; 3]; 4] = [[242, 194, 48], [108, 192, 112], [111, 164, 239], [240, 112, 104]];
