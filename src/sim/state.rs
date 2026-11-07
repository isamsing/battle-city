//! The deterministic game state and its single `step` function.
//!
//! Everything here is integer maths, iterated in a fixed order (players by slot,
//! enemies and bullets by creation order), and driven by the seeded [`Rng`] in
//! the state. Two peers that feed the same inputs get bit-identical states.

use std::hash::{Hash, Hasher};

use super::rng::Rng;
use super::stage;
use super::types::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PlayerStatus {
    /// Slot unused in this session.
    Absent,
    Spawning,
    Active,
    /// Destroyed, waiting to respawn.
    Dead,
    /// No lives left.
    Out,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Phase {
    Playing,
    StageClear,
    GameOver,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Player {
    pub status: PlayerStatus,
    pub x: i32,
    pub y: i32,
    pub dir: Dir,
    pub lives: u8,
    pub score: u32,
    pub stars: u8,
    pub timer: u16,
    pub shield: u16,
    pub stun: u16,
    pub cooldown: u16,
    pub slide: u16,
    pub prev_bits: u8,
    pub human: bool,
    pub moving: bool,
    pub blocked: bool,
    pub ai_dir: Dir,
}

impl Player {
    fn absent() -> Self {
        Self {
            status: PlayerStatus::Absent,
            x: 0,
            y: 0,
            dir: Dir::Up,
            lives: 0,
            score: 0,
            stars: 0,
            timer: 0,
            shield: 0,
            stun: 0,
            cooldown: 0,
            slide: 0,
            prev_bits: 0,
            human: true,
            moving: false,
            blocked: false,
            ai_dir: Dir::Up,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Enemy {
    pub id: u32,
    pub kind: EnemyKind,
    pub x: i32,
    pub y: i32,
    pub dir: Dir,
    pub hp: u8,
    /// Carries a power-up that drops when it is hit.
    pub flashing: bool,
    pub spawn: u16,
    pub fire_cd: u16,
    pub turn_cd: u16,
    pub blocked: bool,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Bullet {
    /// Centre, in sub-pixels.
    pub x: i32,
    pub y: i32,
    pub dir: Dir,
    pub speed: i32,
    pub owner: Owner,
    /// Player star level; 3 breaks steel.
    pub power: u8,
}

impl Bullet {
    fn player_team(&self) -> bool {
        matches!(self.owner, Owner::Player(_))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PowerUp {
    pub kind: PowerKind,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Sim {
    pub frame: u32,
    pub stage_number: u32,
    pub num_players: u8,
    pub cells: Vec<Cell>,
    pub rng: Rng,
    pub players: [Player; MAX_PLAYERS],
    pub enemies: Vec<Enemy>,
    pub bullets: Vec<Bullet>,
    pub power: Option<PowerUp>,
    /// Enemies still waiting to spawn, next first.
    pub queue: Vec<EnemyKind>,
    pub spawn_cd: u16,
    pub spawn_slot: u8,
    pub spawned: u32,
    pub freeze: u16,
    pub shovel: u16,
    pub phase: Phase,
    pub phase_timer: u16,
    pub next_id: u32,
    pub eagle_alive: bool,
}

/// Stable 64-bit FNV-1a hasher. `usize`/`isize` are widened so native and wasm32
/// builds produce the same checksum.
pub struct Fnv64(u64);

impl Default for Fnv64 {
    fn default() -> Self {
        Fnv64(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for Fnv64 {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= *b as u64;
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    fn write_usize(&mut self, i: usize) {
        self.write_u64(i as u64);
    }
    fn write_isize(&mut self, i: isize) {
        self.write_i64(i as i64);
    }
}

fn snap(v: i32) -> i32 {
    ((v + CELL / 2) / CELL) * CELL
}

fn rects_overlap(ax: i32, ay: i32, aw: i32, bx: i32, by: i32, bw: i32) -> bool {
    ax < bx + bw && ax + aw > bx && ay < by + bw && ay + aw > by
}

impl Sim {
    /// Start a new game with `num_players` (1..=4) from `seed`.
    pub fn new(num_players: usize, seed: u64) -> Self {
        let n = num_players.clamp(1, MAX_PLAYERS);
        let mut sim = Sim {
            frame: 0,
            stage_number: 0,
            num_players: n as u8,
            cells: stage::build_cells(0),
            rng: Rng::new(seed),
            players: [Player::absent(), Player::absent(), Player::absent(), Player::absent()],
            enemies: Vec::new(),
            bullets: Vec::new(),
            power: None,
            queue: Vec::new(),
            spawn_cd: 60,
            spawn_slot: 0,
            spawned: 0,
            freeze: 0,
            shovel: 0,
            phase: Phase::Playing,
            phase_timer: 0,
            next_id: 1,
            eagle_alive: true,
        };
        for i in 0..n {
            let (x, y) = stage::player_spawn(i);
            let p = &mut sim.players[i];
            p.status = PlayerStatus::Spawning;
            p.timer = SPAWN_FRAMES;
            p.x = x;
            p.y = y;
            p.lives = START_LIVES;
        }
        sim.queue = sim.build_queue();
        sim
    }

    pub fn checksum(&self) -> u64 {
        let mut h = Fnv64::default();
        self.hash(&mut h);
        h.finish()
    }

    /// Maximum simultaneous enemies: 4, 6, 8, 8 for 1..4 players.
    pub fn max_active(&self) -> usize {
        (4 + 2 * (self.num_players as usize - 1)).min(8)
    }

    /// Enemies per stage: 20, 26, 32, 38 for 1..4 players.
    pub fn enemies_per_stage(&self) -> usize {
        20 + 6 * (self.num_players as usize - 1)
    }

    fn build_queue(&mut self) -> Vec<EnemyKind> {
        let s = self.stage_number.min(12);
        let weights = [
            70u32.saturating_sub(6 * s).max(10),
            15 + 2 * s,
            10 + 2 * s,
            5 + 2 * s,
        ];
        let total: u32 = weights.iter().sum();
        let mut out = Vec::new();
        for _ in 0..self.enemies_per_stage() {
            let mut r = self.rng.range(total);
            let mut kind = EnemyKind::Basic;
            for (k, w) in EnemyKind::ALL.iter().zip(weights) {
                if r < w {
                    kind = *k;
                    break;
                }
                r -= w;
            }
            out.push(kind);
        }
        out
    }

    // ---------------------------------------------------------------- queries

    fn cell(&self, cx: i32, cy: i32) -> Cell {
        self.cells[stage::idx(cx, cy)]
    }

    fn set_ring(&mut self, cell: Cell) {
        for (cx, cy) in stage::EAGLE_RING {
            self.cells[stage::idx(cx, cy)] = cell;
        }
    }

    fn blocked_at(&self, x: i32, y: i32, others: &[(i32, i32)]) -> bool {
        if x < 0 || y < 0 || x + TANK > FIELD || y + TANK > FIELD {
            return true;
        }
        let (cx0, cx1) = (x / CELL, (x + TANK - 1) / CELL);
        let (cy0, cy1) = (y / CELL, (y + TANK - 1) / CELL);
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                if self.cell(cx, cy).solid() {
                    return true;
                }
            }
        }
        others.iter().any(|&(ox, oy)| rects_overlap(x, y, TANK, ox, oy, TANK))
    }

    /// Positions of tanks that block movement (active players, spawned enemies).
    fn tank_rects(&self, skip_player: Option<usize>, skip_enemy: Option<u32>) -> Vec<(i32, i32)> {
        let mut v = Vec::new();
        for (i, p) in self.players.iter().enumerate() {
            if Some(i) != skip_player && p.status == PlayerStatus::Active {
                v.push((p.x, p.y));
            }
        }
        for e in &self.enemies {
            if Some(e.id) != skip_enemy && e.spawn == 0 {
                v.push((e.x, e.y));
            }
        }
        v
    }

    /// Is any tank (including spawning ones) overlapping this spawn area?
    fn area_occupied(&self, x: i32, y: i32, skip_player: Option<usize>) -> bool {
        let player_hit = self.players.iter().enumerate().any(|(i, p)| {
            Some(i) != skip_player
                && matches!(p.status, PlayerStatus::Active | PlayerStatus::Spawning)
                && rects_overlap(x, y, TANK, p.x, p.y, TANK)
        });
        player_hit || self.enemies.iter().any(|e| rects_overlap(x, y, TANK, e.x, e.y, TANK))
    }

    /// Move a tank up to `speed` sub-pixels in `new_dir`. Turning across an axis
    /// first snaps the other coordinate to the 8px cell grid (as on the NES).
    fn move_tank(&self, x: i32, y: i32, old: Dir, new: Dir, speed: i32, others: &[(i32, i32)]) -> (i32, i32, bool) {
        let (mut x, mut y) = (x, y);
        if old.horizontal() != new.horizontal() {
            if new.horizontal() {
                let sy = snap(y);
                if !self.blocked_at(x, sy, others) {
                    y = sy;
                }
            } else {
                let sx = snap(x);
                if !self.blocked_at(sx, y, others) {
                    x = sx;
                }
            }
        }
        let (dx, dy) = new.delta();
        for s in (1..=speed).rev() {
            let (nx, ny) = (x + dx * s, y + dy * s);
            if !self.blocked_at(nx, ny, others) {
                return (nx, ny, true);
            }
        }
        (x, y, false)
    }

    fn on_ice(&self, x: i32, y: i32) -> bool {
        self.cell((x + TANK / 2) / CELL, (y + TANK / 2) / CELL) == Cell::Ice
    }

    // ------------------------------------------------------------------- step

    /// Advance one frame. `inputs[i]` is slot `i`'s input. Returns events for
    /// audio/VFX (not part of the state, never fed back into the simulation).
    pub fn step(&mut self, inputs: &[StepInput; MAX_PLAYERS]) -> Vec<SimEvent> {
        let mut ev = Vec::new();
        self.frame += 1;
        match self.phase {
            Phase::GameOver => return ev,
            Phase::StageClear => {
                self.phase_timer = self.phase_timer.saturating_sub(1);
                if self.phase_timer == 0 {
                    self.next_stage();
                }
                return ev;
            }
            Phase::Playing => {}
        }
        self.freeze = self.freeze.saturating_sub(1);
        if self.shovel > 0 {
            self.shovel -= 1;
            if self.shovel == 0 {
                self.set_ring(Cell::Brick);
            }
        }
        for i in 0..MAX_PLAYERS {
            self.update_player(i, inputs[i], &mut ev);
        }
        self.update_enemies();
        self.update_bullets(&mut ev);
        self.pick_up_power(&mut ev);
        self.spawn_enemies();
        self.check_end(&mut ev);
        ev
    }

    fn event(&self, kind: SimEventKind, x: i32, y: i32) -> SimEvent {
        SimEvent { frame: self.frame, kind, x, y }
    }

    fn next_stage(&mut self) {
        self.stage_number += 1;
        self.cells = stage::build_cells(self.stage_number as usize);
        self.enemies.clear();
        self.bullets.clear();
        self.power = None;
        self.spawned = 0;
        self.spawn_cd = 60;
        self.freeze = 0;
        self.shovel = 0;
        self.eagle_alive = true;
        self.queue = self.build_queue();
        self.phase = Phase::Playing;
        for i in 0..MAX_PLAYERS {
            let (x, y) = stage::player_spawn(i);
            let p = &mut self.players[i];
            if p.status == PlayerStatus::Absent {
                continue;
            }
            if p.status == PlayerStatus::Out {
                p.lives = 1; // dead teammates rejoin each stage
            }
            p.status = PlayerStatus::Spawning;
            p.timer = SPAWN_FRAMES;
            p.x = x;
            p.y = y;
            p.dir = Dir::Up;
            p.stun = 0;
            p.shield = 0;
            p.slide = 0;
        }
    }

    fn ai_bits(&mut self, p: &mut Player) -> u8 {
        if p.blocked || self.rng.range(90) == 0 {
            p.ai_dir = Dir::from_index(self.rng.range(4));
        }
        let mut bits = match p.ai_dir {
            Dir::Up => IN_UP,
            Dir::Right => IN_RIGHT,
            Dir::Down => IN_DOWN,
            Dir::Left => IN_LEFT,
        };
        if self.rng.range(24) == 0 {
            bits |= IN_FIRE;
        }
        bits
    }

    fn update_player(&mut self, i: usize, input: StepInput, ev: &mut Vec<SimEvent>) {
        let mut p = self.players[i].clone();
        p.human = input.human;
        match p.status {
            PlayerStatus::Absent | PlayerStatus::Out => {}
            PlayerStatus::Dead => {
                p.timer = p.timer.saturating_sub(1);
                if p.timer == 0 {
                    if p.lives > 0 {
                        let (x, y) = stage::player_spawn(i);
                        p.status = PlayerStatus::Spawning;
                        p.timer = SPAWN_FRAMES;
                        p.x = x;
                        p.y = y;
                        p.dir = Dir::Up;
                        p.stars = 0;
                    } else {
                        p.status = PlayerStatus::Out;
                    }
                }
            }
            PlayerStatus::Spawning => {
                p.timer = p.timer.saturating_sub(1);
                if p.timer == 0 {
                    if self.area_occupied(p.x, p.y, Some(i)) {
                        p.timer = 1; // wait for the spawn point to clear
                    } else {
                        p.status = PlayerStatus::Active;
                        p.shield = PLAYER_SHIELD_FRAMES;
                    }
                }
            }
            PlayerStatus::Active => self.drive_player(i, &mut p, input, ev),
        }
        self.players[i] = p;
    }

    fn drive_player(&mut self, i: usize, p: &mut Player, input: StepInput, ev: &mut Vec<SimEvent>) {
        p.shield = p.shield.saturating_sub(1);
        p.cooldown = p.cooldown.saturating_sub(1);
        let mut bits = if p.human { input.bits } else { self.ai_bits(p) };
        if p.stun > 0 {
            p.stun -= 1;
            bits = 0;
        }
        let want = if bits & IN_UP != 0 {
            Some(Dir::Up)
        } else if bits & IN_DOWN != 0 {
            Some(Dir::Down)
        } else if bits & IN_LEFT != 0 {
            Some(Dir::Left)
        } else if bits & IN_RIGHT != 0 {
            Some(Dir::Right)
        } else {
            None
        };
        let others = self.tank_rects(Some(i), None);
        let on_ice = self.on_ice(p.x, p.y);
        if let Some(d) = want {
            let (nx, ny, moved) = self.move_tank(p.x, p.y, p.dir, d, PLAYER_SPEED, &others);
            p.x = nx;
            p.y = ny;
            p.dir = d;
            p.moving = true;
            p.blocked = !moved;
            if on_ice {
                p.slide = ICE_SLIDE;
            }
        } else {
            p.moving = false;
            p.blocked = false;
            if p.slide > 0 && on_ice {
                let (nx, ny, _) = self.move_tank(p.x, p.y, p.dir, p.dir, PLAYER_SPEED, &others);
                p.x = nx;
                p.y = ny;
                p.slide -= 1;
            } else {
                p.slide = 0;
            }
        }
        let fire_edge = bits & IN_FIRE != 0 && p.prev_bits & IN_FIRE == 0;
        let max_bullets = if p.stars >= 2 { 2 } else { 1 };
        let live = self.bullets.iter().filter(|b| b.owner == Owner::Player(i as u8)).count();
        if fire_edge && live < max_bullets && p.cooldown == 0 {
            let speed = if p.stars >= 1 { 48 } else { 32 };
            self.fire(p.x, p.y, p.dir, speed, Owner::Player(i as u8), p.stars);
            p.cooldown = 8;
            ev.push(self.event(SimEventKind::PlayerFire, p.x + TANK / 2, p.y + TANK / 2));
        }
        p.prev_bits = bits;
    }

    fn fire(&mut self, x: i32, y: i32, dir: Dir, speed: i32, owner: Owner, power: u8) {
        let (dx, dy) = dir.delta();
        self.bullets.push(Bullet {
            x: x + TANK / 2 + dx * TANK / 2,
            y: y + TANK / 2 + dy * TANK / 2,
            dir,
            speed,
            owner,
            power,
        });
    }

    fn update_enemies(&mut self) {
        for j in 0..self.enemies.len() {
            let mut e = self.enemies[j].clone();
            if e.spawn > 0 {
                e.spawn -= 1;
                self.enemies[j] = e;
                continue;
            }
            if self.freeze > 0 {
                self.enemies[j] = e;
                continue;
            }
            e.turn_cd = e.turn_cd.saturating_sub(1);
            if e.blocked || e.turn_cd == 0 {
                e.dir = match self.rng.range(8) {
                    0..=2 => Dir::Down,
                    3 => {
                        if e.x < 6 * TANK {
                            Dir::Right
                        } else {
                            Dir::Left
                        }
                    }
                    r => Dir::from_index(r),
                };
                e.turn_cd = 60 + self.rng.range(120) as u16;
            }
            let others = self.tank_rects(None, Some(e.id));
            // `move_tank` snaps on axis changes, so pass the previous heading.
            let prev = self.enemies[j].dir;
            let (nx, ny, moved) = self.move_tank(e.x, e.y, prev, e.dir, e.kind.speed(), &others);
            e.x = nx;
            e.y = ny;
            e.blocked = !moved;
            if e.fire_cd > 0 {
                e.fire_cd -= 1;
            } else {
                let live = self.bullets.iter().filter(|b| b.owner == Owner::Enemy(e.id)).count();
                if live == 0 {
                    self.fire(e.x, e.y, e.dir, e.kind.bullet_speed(), Owner::Enemy(e.id), 0);
                }
                e.fire_cd = 30 + self.rng.range(e.kind.fire_period()) as u16;
            }
            self.enemies[j] = e;
        }
    }

    fn update_bullets(&mut self, ev: &mut Vec<SimEvent>) {
        for b in &mut self.bullets {
            let (dx, dy) = b.dir.delta();
            b.x += dx * b.speed;
            b.y += dy * b.speed;
        }
        let n = self.bullets.len();
        let mut dead = vec![false; n];
        for i in 0..n {
            let b = self.bullets[i].clone();
            if b.x - BULLET_HALF < 0 || b.y - BULLET_HALF < 0 || b.x + BULLET_HALF > FIELD || b.y + BULLET_HALF > FIELD {
                dead[i] = true;
                ev.push(self.event(SimEventKind::SteelHit, b.x.clamp(0, FIELD), b.y.clamp(0, FIELD)));
                continue;
            }
            if self.bullet_vs_cells(&b, ev) {
                dead[i] = true;
                continue;
            }
            if self.bullet_vs_tanks(&b, ev) {
                dead[i] = true;
                continue;
            }
        }
        // Opposing bullets cancel each other.
        for i in 0..n {
            for j in (i + 1)..n {
                if dead[i] || dead[j] {
                    continue;
                }
                let (a, b) = (&self.bullets[i], &self.bullets[j]);
                if a.player_team() != b.player_team()
                    && (a.x - b.x).abs() < 2 * BULLET_HALF
                    && (a.y - b.y).abs() < 2 * BULLET_HALF
                {
                    dead[i] = true;
                    dead[j] = true;
                }
            }
        }
        let mut k = 0;
        self.bullets.retain(|_| {
            let keep = !dead[k];
            k += 1;
            keep
        });
        self.enemies.retain(|e| e.hp > 0);
    }

    /// Returns true when the bullet is stopped by terrain.
    ///
    /// The bullet destroys whatever it reaches *first* along its direction of
    /// travel, across its width, and stops there. Without this, one shot would
    /// break a ring brick and kill the eagle behind it in the same frame.
    fn bullet_vs_cells(&mut self, b: &Bullet, ev: &mut Vec<SimEvent>) -> bool {
        // 4px along the travel axis, one full cell across it.
        let (hw, hh) = if b.dir.horizontal() { (BULLET_HALF, CELL / 2) } else { (CELL / 2, BULLET_HALF) };
        let cx0 = ((b.x - hw).max(0)) / CELL;
        let cx1 = ((b.x + hw - 1).min(FIELD - 1)) / CELL;
        let cy0 = ((b.y - hh).max(0)) / CELL;
        let cy1 = ((b.y + hh - 1).min(FIELD - 1)) / CELL;
        let mut hits: Vec<(i32, i32)> = Vec::new();
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                if matches!(self.cells[stage::idx(cx, cy)], Cell::Brick | Cell::Steel | Cell::Eagle) {
                    hits.push((cx, cy));
                }
            }
        }
        if hits.is_empty() {
            return false;
        }
        // Distance along the travel axis, nearest first.
        let depth = |&(cx, cy): &(i32, i32)| match b.dir {
            Dir::Down => cy,
            Dir::Up => -cy,
            Dir::Right => cx,
            Dir::Left => -cx,
        };
        let first = hits.iter().map(depth).min().unwrap();
        for (cx, cy) in hits.into_iter().filter(|h| depth(h) == first) {
            let i = stage::idx(cx, cy);
            match self.cells[i] {
                Cell::Brick => {
                    self.cells[i] = Cell::Empty;
                    ev.push(self.event(SimEventKind::BrickBreak, b.x, b.y));
                }
                Cell::Steel => {
                    if b.power >= 3 {
                        self.cells[i] = Cell::Empty;
                    }
                    ev.push(self.event(SimEventKind::SteelHit, b.x, b.y));
                }
                Cell::Eagle => {
                    if self.eagle_alive {
                        self.eagle_alive = false;
                        ev.push(self.event(SimEventKind::EagleDestroyed, b.x, b.y));
                    }
                }
                _ => {}
            }
        }
        true
    }

    /// Returns true when the bullet hit a tank.
    fn bullet_vs_tanks(&mut self, b: &Bullet, ev: &mut Vec<SimEvent>) -> bool {
        let hit = |tx: i32, ty: i32| {
            b.x - BULLET_HALF < tx + TANK && b.x + BULLET_HALF > tx && b.y - BULLET_HALF < ty + TANK && b.y + BULLET_HALF > ty
        };
        for i in 0..MAX_PLAYERS {
            if b.owner == Owner::Player(i as u8) {
                continue;
            }
            let p = self.players[i].clone();
            if p.status != PlayerStatus::Active || !hit(p.x, p.y) {
                continue;
            }
            if b.player_team() {
                // Friendly fire stuns the teammate instead of killing them.
                if p.shield == 0 && p.stun == 0 {
                    self.players[i].stun = STUN_FRAMES;
                    ev.push(self.event(SimEventKind::PlayerStunned, p.x + TANK / 2, p.y + TANK / 2));
                }
            } else if p.shield == 0 {
                let q = &mut self.players[i];
                q.status = PlayerStatus::Dead;
                q.timer = RESPAWN_DELAY;
                q.lives = q.lives.saturating_sub(1);
                ev.push(self.event(SimEventKind::PlayerDied, p.x + TANK / 2, p.y + TANK / 2));
            }
            return true;
        }
        if let Owner::Player(owner) = b.owner {
            for j in 0..self.enemies.len() {
                let e = self.enemies[j].clone();
                if e.spawn > 0 || e.hp == 0 || !hit(e.x, e.y) {
                    continue;
                }
                let was_flashing = e.flashing;
                self.enemies[j].flashing = false;
                self.enemies[j].hp = e.hp.saturating_sub(1);
                let (cx, cy) = (e.x + TANK / 2, e.y + TANK / 2);
                if self.enemies[j].hp == 0 {
                    self.players[owner as usize].score += e.kind.score();
                    ev.push(self.event(SimEventKind::EnemyDied, cx, cy));
                } else {
                    ev.push(self.event(SimEventKind::EnemyHit, cx, cy));
                }
                if was_flashing {
                    self.drop_power(ev);
                }
                return true;
            }
        }
        false
    }

    fn drop_power(&mut self, ev: &mut Vec<SimEvent>) {
        let tx = 1 + self.rng.range(11) as i32;
        let ty = 1 + self.rng.range(10) as i32;
        let kind = PowerKind::ALL[self.rng.range(PowerKind::ALL.len() as u32) as usize];
        let pu = PowerUp { kind, x: tx * TANK, y: ty * TANK };
        self.power = Some(pu);
        ev.push(self.event(SimEventKind::PowerUpSpawned, pu.x + TANK / 2, pu.y + TANK / 2));
    }

    fn pick_up_power(&mut self, ev: &mut Vec<SimEvent>) {
        let Some(pu) = self.power else { return };
        for i in 0..MAX_PLAYERS {
            let p = &self.players[i];
            if p.status != PlayerStatus::Active || !rects_overlap(p.x, p.y, TANK, pu.x, pu.y, TANK) {
                continue;
            }
            self.power = None;
            self.players[i].score += 500;
            match pu.kind {
                PowerKind::Helmet => self.players[i].shield = HELMET_FRAMES,
                PowerKind::Timer => self.freeze = FREEZE_FRAMES,
                PowerKind::Shovel => {
                    self.set_ring(Cell::Steel);
                    self.shovel = SHOVEL_FRAMES;
                }
                PowerKind::Star => self.players[i].stars = (self.players[i].stars + 1).min(3),
                PowerKind::Grenade => {
                    for e in &mut self.enemies {
                        if e.spawn == 0 && e.hp > 0 {
                            e.hp = 0;
                            ev.push(SimEvent { frame: self.frame, kind: SimEventKind::EnemyDied, x: e.x + TANK / 2, y: e.y + TANK / 2 });
                        }
                    }
                    self.enemies.retain(|e| e.hp > 0);
                }
                PowerKind::Tank => self.players[i].lives = (self.players[i].lives + 1).min(9),
            }
            ev.push(self.event(SimEventKind::PowerUpTaken, pu.x + TANK / 2, pu.y + TANK / 2));
            return;
        }
    }

    fn spawn_enemies(&mut self) {
        if self.queue.is_empty() {
            return;
        }
        if self.spawn_cd > 0 {
            self.spawn_cd -= 1;
            return;
        }
        if self.enemies.len() >= self.max_active() {
            return;
        }
        for attempt in 0..3 {
            let slot = (self.spawn_slot as usize + attempt) % 3;
            let (x, y) = stage::enemy_spawn(slot);
            if self.area_occupied(x, y, None) {
                continue;
            }
            let kind = self.queue.remove(0);
            self.spawned += 1;
            let id = self.next_id;
            self.next_id += 1;
            let fire_cd = 60 + self.rng.range(60) as u16;
            self.enemies.push(Enemy {
                id,
                kind,
                x,
                y,
                dir: Dir::Down,
                hp: kind.hp(),
                flashing: self.spawned % 7 == 4,
                spawn: SPAWN_FRAMES,
                fire_cd,
                turn_cd: 60,
                blocked: false,
            });
            self.spawn_slot = ((slot + 1) % 3) as u8;
            let base = 190u32.saturating_sub(4 * self.stage_number + 20 * (self.num_players as u32 - 1));
            self.spawn_cd = base.max(60) as u16;
            return;
        }
    }

    fn check_end(&mut self, ev: &mut Vec<SimEvent>) {
        let present = self.players.iter().filter(|p| p.status != PlayerStatus::Absent);
        let all_out = present.clone().all(|p| p.status == PlayerStatus::Out);
        if !self.eagle_alive || all_out {
            self.phase = Phase::GameOver;
            ev.push(self.event(SimEventKind::GameOver, 0, 0));
        } else if self.queue.is_empty() && self.enemies.is_empty() {
            self.phase = Phase::StageClear;
            self.phase_timer = STAGE_CLEAR_FRAMES;
            ev.push(self.event(SimEventKind::StageClear, 0, 0));
        }
    }
}
