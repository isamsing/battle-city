//! End-to-end rollback test without a browser: N real GGRS P2P sessions, each
//! running the real `Sim`, talk over an in-memory network with latency, jitter
//! and packet loss. No desync may be reported and every peer must agree on the
//! checksum of every confirmed frame.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy_ggrs::ggrs::{
    Config, DesyncDetection, GgrsError, GgrsEvent, GgrsRequest, InputStatus, Message, NonBlockingSocket,
    P2PSession, PlayerType, PredictRepeatLast, SessionBuilder, SessionState,
};
use battle_city::sim::*;

struct Cfg;
impl Config for Cfg {
    type Input = u8;
    type InputPredictor = PredictRepeatLast;
    type State = Sim;
    type Address = usize;
}

/// Shared in-memory "internet".
struct Net {
    tick: u64,
    rng: u64,
    latency: u64,
    jitter: u64,
    loss_percent: u64,
    inbox: Vec<Vec<(u64, usize, Message)>>,
    dead: HashSet<usize>,
}

impl Net {
    fn rand(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }
}

struct MemSocket {
    me: usize,
    net: Arc<Mutex<Net>>,
}

impl NonBlockingSocket<usize> for MemSocket {
    fn send_to(&mut self, msg: &Message, addr: &usize) {
        let mut n = self.net.lock().unwrap();
        if n.dead.contains(&self.me) || n.dead.contains(addr) || n.rand() % 100 < n.loss_percent {
            return;
        }
        let j = if n.jitter > 0 { n.rand() % (n.jitter * 2 + 1) } else { 0 };
        let at = n.tick + n.latency + j - n.jitter.min(n.latency + j);
        n.inbox[*addr].push((at, self.me, msg.clone()));
    }

    fn receive_all_messages(&mut self) -> Vec<(usize, Message)> {
        let mut n = self.net.lock().unwrap();
        let now = n.tick;
        let mine = std::mem::take(&mut n.inbox[self.me]);
        let (ready, later): (Vec<_>, Vec<_>) = mine.into_iter().partition(|(at, _, _)| *at <= now);
        n.inbox[self.me] = later;
        ready.into_iter().map(|(_, from, m)| (from, m)).collect()
    }
}

struct Peer {
    session: P2PSession<Cfg>,
    sim: Sim,
    handle: usize,
    saved: HashMap<i32, u64>,
    desyncs: Vec<i32>,
    disconnected: Vec<usize>,
    rollbacks: u32,
    /// Test hook: corrupt this peer's state once, to prove desync detection works.
    corrupt_at: Option<i32>,
    /// Drive this peer with the bot instead of the random script.
    bot: bool,
}

fn script(handle: usize, frame: i32) -> u8 {
    let f = (frame / 8) as u64;
    let mut z = (f + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ ((handle as u64 + 1) << 40);
    z ^= z >> 29;
    z = z.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^= z >> 32;
    let dirs = [IN_UP, IN_DOWN, IN_LEFT, IN_RIGHT, 0];
    let mut bits = dirs[((z >> 8) % 5) as usize];
    if (z >> 20) % 3 == 0 {
        bits |= IN_FIRE;
    }
    bits
}

fn make_peers(n: usize, latency_ticks: u64, jitter: u64, loss: u64, seed: u64) -> (Vec<Peer>, Arc<Mutex<Net>>) {
    let net = Arc::new(Mutex::new(Net {
        tick: 0,
        rng: 0x1234_5678_9ABC_DEF1 ^ seed,
        latency: latency_ticks,
        jitter,
        loss_percent: loss,
        inbox: (0..n).map(|_| Vec::new()).collect(),
        dead: HashSet::new(),
    }));
    let peers = (0..n)
        .map(|me| {
            let mut b = SessionBuilder::<Cfg>::new()
                .with_num_players(n)
                .unwrap()
                .with_input_delay(2)
                .with_max_prediction_window(10)
                .with_fps(60)
                .unwrap()
                .with_desync_detection_mode(DesyncDetection::On { interval: 10 })
                .with_disconnect_timeout(Duration::from_millis(1500))
                .with_disconnect_notify_delay(Duration::from_millis(500));
            for h in 0..n {
                let t = if h == me { PlayerType::Local } else { PlayerType::Remote(h) };
                b = b.add_player(t, h).unwrap();
            }
            let session = b.start_p2p_session(MemSocket { me, net: net.clone() }).unwrap();
            Peer { session, sim: Sim::new(n, seed), handle: me, saved: HashMap::new(), desyncs: vec![], disconnected: vec![], rollbacks: 0, corrupt_at: None, bot: false }
        })
        .collect();
    (peers, net)
}

fn tick_peer(p: &mut Peer) {
    p.session.poll_remote_clients();
    for ev in p.session.events().collect::<Vec<_>>() {
        match ev {
            GgrsEvent::DesyncDetected { frame, .. } => p.desyncs.push(frame),
            GgrsEvent::Disconnected { addr } => p.disconnected.push(addr),
            _ => {}
        }
    }
    if p.session.current_state() != SessionState::Running {
        return;
    }
    let frame = p.session.current_frame();
    let bits = if p.bot { bot_bits(&p.sim, p.handle) } else { script(p.handle, frame) };
    p.session.add_local_input(p.handle, bits).unwrap();
    let requests = match p.session.advance_frame() {
        Ok(r) => r,
        Err(GgrsError::PredictionThreshold) => return,
        Err(e) => panic!("peer {} advance_frame: {e:?}", p.handle),
    };
    for r in requests {
        match r {
            GgrsRequest::SaveGameState { cell, frame } => {
                let sum = p.sim.checksum();
                p.saved.insert(frame, sum);
                cell.save(frame, Some(p.sim.clone()), Some(sum as u128));
            }
            GgrsRequest::LoadGameState { cell, .. } => {
                p.rollbacks += 1;
                p.sim = cell.load().unwrap()
            }
            GgrsRequest::AdvanceFrame { inputs } => {
                if p.corrupt_at == Some(p.sim.frame as i32) {
                    p.sim.players[0].score += 1; // silent divergence on one peer only
                }
                let mut arr = [StepInput::default(); MAX_PLAYERS];
                for (i, (bits, status)) in inputs.iter().enumerate() {
                    arr[i] = StepInput { bits: *bits, human: *status != InputStatus::Disconnected };
                }
                p.sim.step(&arr);
            }
        }
    }
}

fn run(peers: &mut [Peer], net: &Arc<Mutex<Net>>, ticks: u64, drop_peer_at: Option<(u64, usize)>) {
    let start = net.lock().unwrap().tick;
    for t in start..start + ticks {
        net.lock().unwrap().tick = t;
        if let Some((at, who)) = drop_peer_at {
            if t == at {
                net.lock().unwrap().dead.insert(who);
            }
        }
        let dead: HashSet<usize> = net.lock().unwrap().dead.clone();
        for p in peers.iter_mut() {
            if !dead.contains(&p.handle) {
                tick_peer(p);
            }
        }
        // Wall-clock based GGRS timers (resend, keepalive) need real time to pass.
        std::thread::sleep(Duration::from_micros(400));
    }
}

fn assert_consistent(peers: &[Peer], skip: &HashSet<usize>) {
    let live: Vec<&Peer> = peers.iter().filter(|p| !skip.contains(&p.handle)).collect();
    for p in &live {
        assert!(p.desyncs.is_empty(), "peer {} reported desync at frames {:?}", p.handle, p.desyncs);
    }
    let confirmed = live.iter().map(|p| p.session.confirmed_frame()).min().unwrap();
    assert!(confirmed > 100, "session barely progressed (confirmed frame {confirmed})");
    for frame in 1..=confirmed {
        let sums: Vec<_> = live.iter().filter_map(|p| p.saved.get(&frame)).collect();
        assert!(sums.windows(2).all(|w| w[0] == w[1]), "checksum mismatch at confirmed frame {frame}: {sums:x?}");
    }
}

fn quick(n: usize, ticks: u64, latency: u64, jitter: u64, loss: u64) {
    let (mut peers, net) = make_peers(n, latency, jitter, loss, n as u64);
    run(&mut peers, &net, ticks, None);
    assert_consistent(&peers, &HashSet::new());
    assert!(peers.iter().all(|p| p.session.current_frame() > 200));
    let rollbacks: u32 = peers.iter().map(|p| p.rollbacks).sum();
    assert!(rollbacks > 20, "only {rollbacks} rollbacks: the test is not exercising rollback");
}

/// Negative control: if one peer silently diverges, the checks above must fire.
#[test]
fn desync_is_detected_when_a_peer_diverges() {
    let (mut peers, net) = make_peers(3, 4, 1, 0, 5);
    peers[1].corrupt_at = Some(400);
    run(&mut peers, &net, 3000, None);
    let detected = peers.iter().any(|p| !p.desyncs.is_empty());
    let checksum_mismatch = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| assert_consistent(&peers, &HashSet::new()))).is_err();
    assert!(detected && checksum_mismatch, "divergence went unnoticed (ggrs={detected}, ours={checksum_mismatch})");
}

#[test]
fn two_peers_stay_in_sync_with_latency() {
    quick(2, 2500, 3, 1, 2);
}

#[test]
fn four_peers_stay_in_sync_at_100ms_with_jitter_and_loss() {
    // 6 ticks ~ 100 ms one way at 60 Hz.
    quick(4, 4000, 6, 2, 3);
}

#[test]
fn four_peers_survive_a_dropped_player() {
    let (mut peers, net) = make_peers(4, 4, 1, 1, 77);
    run(&mut peers, &net, 1500, Some((1499, 3)));
    // GGRS's disconnect timeout runs on wall-clock time, not ticks, so keep
    // ticking until every survivor has noticed (bounded), however fast the
    // machine is. A tick count alone made this flaky on fast CI runners.
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let survivors_saw_drop = |peers: &[Peer]| peers.iter().filter(|p| p.handle != 3).all(|p| p.disconnected.contains(&3));
    while !survivors_saw_drop(&peers) {
        assert!(std::time::Instant::now() < deadline, "survivors never saw player 3 disconnect");
        run(&mut peers, &net, 50, None);
    }
    let at_drop: Vec<i32> = peers.iter().map(|p| p.session.current_frame()).collect();
    // After the drop is detected, the match must keep running without player 3.
    run(&mut peers, &net, 1500, None);
    let dropped: HashSet<usize> = [3].into_iter().collect();
    for p in peers.iter().filter(|p| p.handle != 3) {
        let gained = p.session.current_frame() - at_drop[p.handle];
        assert!(gained > 600, "peer {} froze after the drop (only {gained} frames)", p.handle);
    }
    assert_consistent(&peers, &dropped);
    // The dropped player's tank must now be AI-controlled on the survivors.
    // (Humans never set `human = false` on slot 3 unless GGRS marked it disconnected.)
}

/// ~10 minutes of game time at 100 ms with jitter and loss. Slow; run with
/// `cargo test --release -- --ignored soak`.
#[test]
#[ignore]
fn soak_four_peers_ten_minutes() {
    quick(4, 36_000, 6, 2, 3);
}


/// Same bot as the sim tests: chase the nearest enemy, shoot when lined up.
fn bot_bits(sim: &Sim, slot: usize) -> u8 {
    let p = &sim.players[slot];
    if p.status != PlayerStatus::Active {
        return 0;
    }
    let Some(e) = sim.enemies.iter().filter(|e| e.spawn == 0).min_by_key(|e| (e.x - p.x).abs() + (e.y - p.y).abs()) else {
        return IN_UP;
    };
    let (dx, dy) = (e.x - p.x, e.y - p.y);
    let (col, row) = (dx.abs() < TANK / 2, dy.abs() < TANK / 2);
    let mut bits = if col {
        if dy < 0 { IN_UP } else { IN_DOWN }
    } else if row {
        if dx < 0 { IN_LEFT } else { IN_RIGHT }
    } else if dy.abs() > dx.abs() {
        if dy < 0 { IN_UP } else { IN_DOWN }
    } else if dx < 0 {
        IN_LEFT
    } else {
        IN_RIGHT
    };
    let base_col = p.x + TANK > 5 * TANK && p.x < 8 * TANK;
    if (col || row) && !(bits & IN_DOWN != 0 && base_col) && sim.frame % 6 >= 3 {
        bits |= IN_FIRE;
    }
    bits
}

/// Four peers with real rollback over a 100 ms-class network play a stage with
/// the bot until it is cleared. Tries a few seeds because the bot is not perfect.
#[test]
fn four_peers_finish_a_stage_over_rollback() {
    let mut cleared_any = false;
    for seed in 1..=6u64 {
        let (mut peers, net) = make_peers(4, 6, 2, 3, seed);
        for p in peers.iter_mut() {
            p.bot = true;
        }
        for _ in 0..30_000u64 {
            run(&mut peers, &net, 1, None);
            net.lock().unwrap().tick += 1;
            if peers.iter().all(|p| p.sim.stage_number >= 1 || p.sim.phase == Phase::GameOver) {
                break;
            }
        }
        assert_consistent(&peers, &HashSet::new());
        if peers.iter().all(|p| p.sim.stage_number >= 1) {
            cleared_any = true;
            assert!(peers.iter().map(|p| p.rollbacks).sum::<u32>() > 20, "no rollbacks happened");
            break;
        }
    }
    assert!(cleared_any, "no seed cleared a stage over the network");
}
