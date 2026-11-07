//! Determinism tests for the headless simulation: the same inputs must always
//! produce the same state, on every run and for every player count.

use battle_city::sim::*;

/// A scripted but busy input stream: pseudo-random movement and firing per slot,
/// generated from its own tiny LCG so the test never depends on the sim's RNG.
fn script(frame: u32, slot: usize) -> StepInput {
    let mut z = (frame as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ ((slot as u64 + 1) << 40);
    z ^= z >> 29;
    z = z.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^= z >> 32;
    let dirs = [IN_UP, IN_DOWN, IN_LEFT, IN_RIGHT, 0];
    let mut bits = dirs[((z >> 8) % 5) as usize];
    if (z >> 20) % 3 == 0 {
        bits |= IN_FIRE;
    }
    StepInput { bits, human: true }
}

fn run(players: usize, seed: u64, frames: u32) -> (Sim, Vec<u64>) {
    let mut sim = Sim::new(players, seed);
    let mut sums = Vec::with_capacity(frames as usize);
    for f in 0..frames {
        let mut inputs = [StepInput::default(); MAX_PLAYERS];
        for (slot, input) in inputs.iter_mut().enumerate() {
            *input = script(f / 8, slot); // hold each input 8 frames, like a human
        }
        sim.step(&inputs);
        sums.push(sim.checksum());
    }
    (sim, sums)
}

#[test]
fn same_inputs_same_state_every_frame() {
    for players in 1..=MAX_PLAYERS {
        let (a, sa) = run(players, 1234, 3000);
        let (b, sb) = run(players, 1234, 3000);
        assert_eq!(sa, sb, "{players}P: per-frame checksums diverged");
        assert_eq!(a, b, "{players}P: final state differs");
    }
}

#[test]
fn different_seeds_diverge() {
    let (_, a) = run(4, 1, 1200);
    let (_, b) = run(4, 2, 1200);
    assert_ne!(a.last(), b.last());
}

#[test]
fn simulation_actually_does_something() {
    let (sim, _) = run(4, 7, 3000);
    assert!(sim.frame == 3000);
    assert!(sim.spawned > 0, "no enemies ever spawned");
    assert!(sim.players.iter().any(|p| p.x != 0 || p.y != 0));
}

/// GGRS rollback: snapshot (clone) at frame N, run on, restore and replay; the
/// replay must land on the exact same checksums.
#[test]
fn rollback_replay_matches() {
    let mut sim = Sim::new(4, 99);
    let mut snapshot = None;
    let mut first = Vec::new();
    for f in 0..1500u32 {
        if f == 600 {
            snapshot = Some(sim.clone());
        }
        let mut inputs = [StepInput::default(); MAX_PLAYERS];
        for (s, i) in inputs.iter_mut().enumerate() {
            *i = script(f / 8, s);
        }
        sim.step(&inputs);
        if f >= 600 {
            first.push(sim.checksum());
        }
    }
    let mut sim = snapshot.unwrap();
    let mut replay = Vec::new();
    for f in 600..1500u32 {
        let mut inputs = [StepInput::default(); MAX_PLAYERS];
        for (s, i) in inputs.iter_mut().enumerate() {
            *i = script(f / 8, s);
        }
        sim.step(&inputs);
        replay.push(sim.checksum());
    }
    assert_eq!(first, replay);
}

#[test]
fn checksum_is_stable_across_platforms() {
    // Golden value: native and wasm32 must agree. If you change game rules this
    // changes; update it deliberately.
    let (_, sums) = run(2, 42, 600);
    assert_eq!(sums[599], 0xd50122b35c4c9f4b, "game rules changed; update the golden value deliberately");
}

#[test]
fn enemy_counts_scale_with_players() {
    let max: Vec<_> = (1..=4).map(|n| Sim::new(n, 0).max_active()).collect();
    let total: Vec<_> = (1..=4).map(|n| Sim::new(n, 0).enemies_per_stage()).collect();
    assert_eq!(max, vec![4, 6, 8, 8]);
    assert_eq!(total, vec![20, 26, 32, 38]);
}

#[test]
fn every_stage_has_four_clear_player_spawns() {
    use battle_city::sim::stage;
    for s in 0..stage::count() {
        let cells = stage::build_cells(s);
        for slot in 0..MAX_PLAYERS {
            let (x, y) = stage::player_spawn(slot);
            for cy in (y / CELL)..((y + TANK) / CELL) {
                for cx in (x / CELL)..((x + TANK) / CELL) {
                    assert!(!cells[stage::idx(cx, cy)].solid(), "stage {s} slot {slot} blocked");
                }
            }
        }
    }
}

#[test]
fn friendly_fire_stuns_instead_of_killing() {
    let mut sim = Sim::new(2, 5);
    // Put P1 facing right, P2 directly in front of it, both active.
    for p in sim.players.iter_mut().take(2) {
        p.status = PlayerStatus::Active;
        p.shield = 0;
    }
    sim.players[0].x = 2 * TANK;
    sim.players[0].y = 6 * TANK;
    sim.players[0].dir = Dir::Right;
    sim.players[1].x = 6 * TANK;
    sim.players[1].y = 6 * TANK;
    sim.cells.iter_mut().for_each(|c| *c = Cell::Empty);
    let fire = StepInput { bits: IN_FIRE, human: true };
    let idle = StepInput { bits: 0, human: true };
    let mut events = sim.step(&[fire, idle, idle, idle]);
    for _ in 0..120 {
        events.extend(sim.step(&[idle; MAX_PLAYERS]));
    }
    assert!(events.iter().any(|e| e.kind == SimEventKind::PlayerStunned), "no stun event");
    assert_eq!(sim.players[1].status, PlayerStatus::Active, "teammate must survive");
    assert_eq!(sim.players[1].lives, START_LIVES);
    assert!(sim.players[1].stun > 0, "teammate should still be stunned");
}

#[test]
fn disconnected_player_is_ai_controlled_and_deterministic() {
    let run_once = || {
        let mut sim = Sim::new(3, 11);
        for f in 0..1800u32 {
            let mut inputs = [StepInput::default(); MAX_PLAYERS];
            for (s, i) in inputs.iter_mut().enumerate() {
                *i = script(f / 8, s);
            }
            inputs[1].human = false; // P2 dropped
            sim.step(&inputs);
        }
        sim.checksum()
    };
    assert_eq!(run_once(), run_once());
}

// ---- rule tests: stage progression, power-ups, game over ----

fn idle() -> [StepInput; MAX_PLAYERS] {
    [StepInput { bits: 0, human: true }; MAX_PLAYERS]
}

fn activate(sim: &mut Sim) {
    for p in sim.players.iter_mut().filter(|p| p.status != PlayerStatus::Absent) {
        p.status = PlayerStatus::Active;
        p.shield = 0;
    }
}

#[test]
fn clearing_all_enemies_advances_to_the_next_stage() {
    let mut sim = Sim::new(4, 3);
    activate(&mut sim);
    sim.queue.clear();
    sim.enemies.clear();
    let ev = sim.step(&idle());
    assert!(ev.iter().any(|e| e.kind == SimEventKind::StageClear));
    assert_eq!(sim.phase, Phase::StageClear);
    for _ in 0..STAGE_CLEAR_FRAMES + 2 {
        sim.step(&idle());
    }
    assert_eq!(sim.phase, Phase::Playing);
    assert_eq!(sim.stage_number, 1);
    assert_eq!(sim.queue.len(), sim.enemies_per_stage());
    assert!(sim.players.iter().all(|p| p.status == PlayerStatus::Spawning), "all four respawn at their spawn points");
}

#[test]
fn destroying_the_eagle_ends_the_game() {
    let mut sim = Sim::new(2, 3);
    activate(&mut sim);
    sim.eagle_alive = false;
    let ev = sim.step(&idle());
    assert!(ev.iter().any(|e| e.kind == SimEventKind::GameOver));
    assert_eq!(sim.phase, Phase::GameOver);
}

#[test]
fn game_over_when_every_player_is_out_of_lives() {
    let mut sim = Sim::new(2, 3);
    for p in sim.players.iter_mut().take(2) {
        p.status = PlayerStatus::Out;
    }
    sim.step(&idle());
    assert_eq!(sim.phase, Phase::GameOver);
}

fn give(sim: &mut Sim, kind: PowerKind) {
    let (x, y) = (sim.players[0].x, sim.players[0].y);
    sim.power = Some(PowerUp { kind, x, y });
    sim.step(&idle());
    assert!(sim.power.is_none(), "{kind:?} was not picked up");
}

#[test]
fn every_power_up_has_its_effect() {
    let mut sim = Sim::new(1, 9);
    activate(&mut sim);
    sim.enemies.push(Enemy { id: 99, kind: EnemyKind::Basic, x: 6 * TANK, y: 0, dir: Dir::Down, hp: 1, flashing: false, spawn: 0, fire_cd: 9999, turn_cd: 9999, blocked: false });
    sim.queue.push(EnemyKind::Basic);

    give(&mut sim, PowerKind::Star);
    assert_eq!(sim.players[0].stars, 1);
    give(&mut sim, PowerKind::Helmet);
    assert!(sim.players[0].shield > HELMET_FRAMES - 5);
    give(&mut sim, PowerKind::Tank);
    assert_eq!(sim.players[0].lives, START_LIVES + 1);
    give(&mut sim, PowerKind::Timer);
    assert!(sim.freeze > 0);
    give(&mut sim, PowerKind::Shovel);
    assert!(sim.shovel > 0);
    assert!(stage_ring_is(&sim, Cell::Steel), "shovel turns the base walls to steel");
    give(&mut sim, PowerKind::Grenade);
    assert!(sim.enemies.is_empty(), "grenade destroys every enemy on screen");
}

fn stage_ring_is(sim: &Sim, cell: Cell) -> bool {
    use battle_city::sim::stage;
    stage::EAGLE_RING.iter().all(|&(cx, cy)| sim.cells[stage::idx(cx, cy)] == cell)
}

#[test]
fn shovel_wears_off_back_to_brick() {
    let mut sim = Sim::new(1, 9);
    activate(&mut sim);
    sim.queue.push(EnemyKind::Basic);
    give(&mut sim, PowerKind::Shovel);
    for _ in 0..SHOVEL_FRAMES + 2 {
        sim.step(&idle());
    }
    assert!(stage_ring_is(&sim, Cell::Brick));
}

#[test]
fn bullets_break_bricks_but_not_steel_until_three_stars() {
    use battle_city::sim::stage;
    let cell_after = |stars: u8, target: Cell| {
        let mut sim = Sim::new(1, 1);
        activate(&mut sim);
        sim.cells.iter_mut().for_each(|c| *c = Cell::Empty);
        sim.queue.push(EnemyKind::Basic);
        let (cx, cy) = (10, 6);
        sim.cells[stage::idx(cx, cy)] = target;
        sim.players[0].x = cx * CELL - 6 * TANK / 2;
        sim.players[0].y = cy * CELL - TANK / 2;
        sim.players[0].dir = Dir::Right;
        sim.players[0].stars = stars;
        let fire = StepInput { bits: IN_FIRE, human: true };
        sim.step(&[fire, StepInput::default(), StepInput::default(), StepInput::default()]);
        for _ in 0..90 {
            sim.step(&idle());
        }
        sim.cells[stage::idx(cx, cy)]
    };
    assert_eq!(cell_after(0, Cell::Brick), Cell::Empty);
    assert_eq!(cell_after(0, Cell::Steel), Cell::Steel);
    assert_eq!(cell_after(3, Cell::Steel), Cell::Empty);
}

#[test]
fn killing_an_enemy_scores_for_the_shooter_only() {
    let mut sim = Sim::new(2, 4);
    activate(&mut sim);
    sim.cells.iter_mut().for_each(|c| *c = Cell::Empty);
    sim.queue.push(EnemyKind::Basic);
    sim.players[1].x = 0;
    sim.players[1].y = 12 * TANK;
    sim.players[0].x = 2 * TANK;
    sim.players[0].y = 6 * TANK;
    sim.players[0].dir = Dir::Right;
    sim.enemies.push(Enemy { id: 7, kind: EnemyKind::Armor, x: 6 * TANK, y: 6 * TANK, dir: Dir::Left, hp: 1, flashing: false, spawn: 0, fire_cd: 9999, turn_cd: 9999, blocked: false });
    sim.freeze = 1000; // timer effect keeps the target still so the shot lands
    let fire = StepInput { bits: IN_FIRE, human: true };
    sim.step(&[fire, StepInput::default(), StepInput::default(), StepInput::default()]);
    for _ in 0..60 {
        sim.step(&idle());
    }
    assert_eq!(sim.players[0].score, EnemyKind::Armor.score());
    assert_eq!(sim.players[1].score, 0);
}
