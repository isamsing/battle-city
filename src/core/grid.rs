use bevy::prelude::*;

use super::config::TILE_SIZE;

#[derive(Component, Debug, Clone, Copy)]
pub struct GridPosition {
    pub x: f32,
    pub y: f32,
}

impl GridPosition {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn to_world(&self) -> Vec2 {
        Vec2::new(self.x * TILE_SIZE, self.y * TILE_SIZE)
    }

    pub fn from_world(world: Vec2) -> Self {
        Self {
            x: world.x / TILE_SIZE,
            y: world.y / TILE_SIZE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(pos.to_world(), Vec2::new(8.0, 16.0));
    }

    #[test]
    fn to_world_negative() {
        let pos = GridPosition::new(-1.0, -3.0);
        assert_eq!(pos.to_world(), Vec2::new(-8.0, -24.0));
    }

    #[test]
    fn to_world_fractional() {
        let pos = GridPosition::new(0.5, 1.5);
        assert_eq!(pos.to_world(), Vec2::new(4.0, 12.0));
    }

    #[test]
    fn from_world_origin() {
        let pos = GridPosition::from_world(Vec2::ZERO);
        assert_eq!(pos.x, 0.0);
        assert_eq!(pos.y, 0.0);
    }

    #[test]
    fn from_world_exact_tile() {
        let pos = GridPosition::from_world(Vec2::new(16.0, 24.0));
        assert_eq!(pos.x, 2.0);
        assert_eq!(pos.y, 3.0);
    }

    #[test]
    fn from_world_fractional() {
        let pos = GridPosition::from_world(Vec2::new(4.0, 12.0));
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
        let original = Vec2::new(32.0, 48.0);
        let roundtrip = GridPosition::from_world(original).to_world();
        assert_eq!(roundtrip, original);
    }
}
