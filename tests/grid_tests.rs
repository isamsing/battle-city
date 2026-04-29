use battle_city::core::config::TILE_SIZE;
use battle_city::core::grid::GridPosition;
use bevy::math::Vec2;

#[test]
fn new_stores_coordinates() {
    let pos = GridPosition::new(3.0, 5.0);
    assert_eq!(pos.x, 3.0);
    assert_eq!(pos.y, 5.0);
}

#[test]
fn to_world_origin() {
    let pos = GridPosition::new(0.0, 0.0);
    assert_eq!(pos.to_world(), Vec2::ZERO);
}

#[test]
fn to_world_positive() {
    let pos = GridPosition::new(1.0, 2.0);
    assert_eq!(pos.to_world(), Vec2::new(TILE_SIZE, 2.0 * TILE_SIZE));
}

#[test]
fn to_world_negative() {
    let pos = GridPosition::new(-1.0, -3.0);
    assert_eq!(pos.to_world(), Vec2::new(-TILE_SIZE, -3.0 * TILE_SIZE));
}

#[test]
fn to_world_fractional() {
    let pos = GridPosition::new(0.5, 1.5);
    assert_eq!(pos.to_world(), Vec2::new(0.5 * TILE_SIZE, 1.5 * TILE_SIZE));
}

#[test]
fn from_world_origin() {
    let pos = GridPosition::from_world(Vec2::ZERO);
    assert_eq!(pos.x, 0.0);
    assert_eq!(pos.y, 0.0);
}

#[test]
fn from_world_exact_tile() {
    let pos = GridPosition::from_world(Vec2::new(2.0 * TILE_SIZE, 3.0 * TILE_SIZE));
    assert_eq!(pos.x, 2.0);
    assert_eq!(pos.y, 3.0);
}

#[test]
fn from_world_fractional() {
    let pos = GridPosition::from_world(Vec2::new(0.5 * TILE_SIZE, 1.5 * TILE_SIZE));
    assert_eq!(pos.x, 0.5);
    assert_eq!(pos.y, 1.5);
}

#[test]
fn roundtrip_to_world_from_world() {
    let original = GridPosition::new(3.0, 7.0);
    let roundtrip = GridPosition::from_world(original.to_world());
    assert_eq!(roundtrip.x, original.x);
    assert_eq!(roundtrip.y, original.y);
}

#[test]
fn roundtrip_from_world_to_world() {
    let original = Vec2::new(2.0 * TILE_SIZE, 4.0 * TILE_SIZE);
    let roundtrip = GridPosition::from_world(original).to_world();
    assert_eq!(roundtrip, original);
}
