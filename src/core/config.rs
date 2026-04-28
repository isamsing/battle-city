pub const TILE_SIZE: f32 = 8.0;
pub const MAP_WIDTH: u32 = 26;
pub const MAP_HEIGHT: u32 = 26;
pub const VIRTUAL_WIDTH: f32 = 256.0;
pub const VIRTUAL_HEIGHT: f32 = 240.0;
pub const WINDOW_SCALE: u32 = 3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_size_positive() {
        assert!(TILE_SIZE > 0.0, "TILE_SIZE must be positive to avoid division by zero");
    }

    #[test]
    fn map_dimensions_nonzero() {
        assert!(MAP_WIDTH > 0);
        assert!(MAP_HEIGHT > 0);
    }

    #[test]
    fn virtual_dimensions_fit_map() {
        assert!(VIRTUAL_WIDTH >= MAP_WIDTH as f32 * TILE_SIZE);
        assert!(VIRTUAL_HEIGHT >= MAP_HEIGHT as f32 * TILE_SIZE);
    }

    #[test]
    fn window_scale_positive() {
        assert!(WINDOW_SCALE > 0);
    }
}
