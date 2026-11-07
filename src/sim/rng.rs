//! Seeded xorshift64 RNG. Part of the rollback state, so it is `Clone + Hash`.

use std::hash::Hash;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // Mix the seed so that small seeds (1, 2, 3 ...) give unrelated streams.
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        Self(if z == 0 { 0x2545_F491_4F6C_DD1D } else { z })
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 32) as u32
    }

    /// Uniform-ish integer in `0..n` (`n > 0`).
    pub fn range(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}
