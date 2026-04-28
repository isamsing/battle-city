use battle_city::core::config::*;

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
