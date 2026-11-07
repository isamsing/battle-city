//! Cost check for the simulation. A rollback tick can replay up to the
//! prediction window (10 frames), so one `step` must be far below a 16.6 ms frame.
//! Only live gameplay frames are timed: a finished game returns early and
//! would make the average look better than it is.

use battle_city::sim::*;
use std::time::{Duration, Instant};

#[test]
fn step_is_cheap_enough_for_rollback() {
    let mut sim = Sim::new(4, 1);
    let (mut live, mut spent, mut seed, mut f) = (0u32, Duration::ZERO, 1u64, 0u32);
    while live < 20_000 {
        if sim.phase != Phase::Playing {
            seed += 1;
            sim = Sim::new(4, seed);
        }
        let bits = if f % 40 < 20 { IN_RIGHT | IN_FIRE } else { IN_UP | IN_FIRE };
        let inputs = [StepInput { bits, human: true }; MAX_PLAYERS];
        let t = Instant::now();
        std::hint::black_box(sim.step(&inputs));
        spent += t.elapsed();
        live += 1;
        f += 1;
    }
    let per_step_us = spent.as_secs_f64() * 1e6 / live as f64;
    let replay_ms = per_step_us * 10.0 / 1000.0;
    // Measured ~2 us per step in release. The bound is loose enough for debug
    // builds on a slow CI machine but still catches an accidental O(n^2) blowup.
    assert!(replay_ms < 4.0, "10-frame replay costs {replay_ms:.3} ms ({per_step_us:.1} us/step)");
}
