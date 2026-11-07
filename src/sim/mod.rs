//! Deterministic, Bevy-free game simulation.
//!
//! The whole game state lives in one [`Sim`] value that uses integer maths only
//! (sub-pixel fixed point), a seeded RNG and fixed iteration order. That makes
//! it trivially snapshot-able for GGRS rollback (`Clone`), checksum-able
//! (`Hash`) and testable headless. Offline and online play both drive the same
//! [`Sim::step`], so there is exactly one implementation of the game rules.

pub mod rng;
pub mod stage;
pub mod state;
pub mod types;

pub use state::*;
pub use types::*;
