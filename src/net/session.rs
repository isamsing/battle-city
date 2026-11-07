//! Matchbox connection, lobby protocol and GGRS session construction.
//!
//! Flow: the host and joiners connect to one matchbox room (`battle_city_CODE`).
//! Over a reliable channel they exchange [`LobbyMsg`]s (roster, ready flags).
//! When the host presses START it sends a seed and the final slot order; every
//! peer then builds the same GGRS session where *slot index == player handle*.

use std::time::Duration;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy::tasks::IoTaskPool;
use bevy_ggrs::ggrs::DesyncDetection;
use bevy_ggrs::prelude::*;
use matchbox_socket::{ChannelConfig, PeerId, RtcIceServerConfig, WebRtcSocket, WebRtcSocketBuilder};

use super::input::BattleCityConfig;
use super::protocol::{decode, encode, LobbyMsg, LOBBY_CHANNEL};
use super::socket_bridge::MatchboxGgrsSocket;
use super::GameMode;
use crate::core::states::GameState;
use crate::sim::MAX_PLAYERS;

/// Signaling server; override at build time with `BATTLE_CITY_SERVER=wss://...`.
pub const DEFAULT_SERVER: &str = match option_env!("BATTLE_CITY_SERVER") {
    Some(s) => s,
    None => "wss://battle-city.fly.dev",
};

#[derive(Resource, Clone)]
pub struct ServerUrl(pub String);

impl Default for ServerUrl {
    fn default() -> Self {
        Self(DEFAULT_SERVER.to_string())
    }
}

#[derive(Resource)]
pub struct MatchboxRes {
    pub socket: WebRtcSocket,
}

#[derive(Resource, Default, Clone)]
pub struct RoomCode(pub String);

pub fn generate_room_code() -> String {
    use rand::Rng;
    // Exclude ambiguous characters: 0/O, 1/I/L
    const CHARS: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";
    let mut rng = rand::rng();
    (0..4).map(|_| CHARS[rng.random_range(0..CHARS.len())] as char).collect()
}

/// STUN + TURN servers. TURN is needed behind strict/symmetric NATs; UDP first,
/// TCP as the fallback for networks that block UDP. Credentials can be
/// overridden at build time (`BATTLE_CITY_TURN_USER` / `BATTLE_CITY_TURN_PASS`).
pub fn ice_servers() -> Vec<RtcIceServerConfig> {
    let host = option_env!("BATTLE_CITY_TURN_HOST").unwrap_or("battle-city.fly.dev");
    vec![
        RtcIceServerConfig {
            urls: vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
            ],
            username: None,
            credential: None,
        },
        RtcIceServerConfig {
            urls: vec![
                format!("turn:{host}:3478?transport=udp"),
                format!("turn:{host}:3478?transport=tcp"),
            ],
            username: Some(option_env!("BATTLE_CITY_TURN_USER").unwrap_or("battlecity").to_string()),
            credential: Some(option_env!("BATTLE_CITY_TURN_PASS").unwrap_or("tankbattle2026").to_string()),
        },
    ]
}

pub fn start_matchbox_socket(commands: &mut Commands, mode: &GameMode, server_url: &ServerUrl) {
    let room_code = match mode {
        GameMode::OnlineHost(code) | GameMode::OnlineJoin(code) => code.clone(),
        GameMode::Local => return,
    };

    // No `?next=N`: the room is open and the host decides who plays.
    let room_url = format!("{}/battle_city_{room_code}", server_url.0);
    let mut builder = WebRtcSocketBuilder::new(room_url)
        .add_channel(ChannelConfig::unreliable()) // 0: GGRS
        .add_channel(ChannelConfig::reliable()); // 1: lobby
    for server in ice_servers() {
        builder = builder.ice_server(server);
    }
    let (socket, loop_fut) = builder.build();

    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = loop_fut.await {
            web_sys::console::error_1(&format!("matchbox socket error: {e:?}").into());
        }
    });

    #[cfg(not(target_arch = "wasm32"))]
    {
        let task_pool = IoTaskPool::get();
        task_pool.spawn(async move {
            if let Err(e) = loop_fut.await {
                error!("matchbox socket error: {e:?}");
            }
        })
        .detach();
    }

    commands.insert_resource(MatchboxRes { socket });
}

// ---------------------------------------------------------------- lobby state

/// Everything the lobby UI needs to show, and the host needs to decide.
#[derive(Resource, Default)]
pub struct LobbyNet {
    pub my_id: Option<PeerId>,
    pub is_host: bool,
    /// Host-chosen player count, 1..=4.
    pub max_players: u8,
    /// Slot order (index == player handle). Host first.
    pub roster: Vec<PeerId>,
    pub ready: HashMap<PeerId, bool>,
    pub local_ready: bool,
    pub start_requested: bool,
    pub started: Option<StartInfo>,
    pub connected: usize,
    pub tick: u32,
    pub rejected: bool,
}

#[derive(Clone, Debug)]
pub struct StartInfo {
    pub seed: u64,
    pub players: Vec<PeerId>,
}

impl LobbyNet {
    pub fn new(is_host: bool) -> Self {
        Self { is_host, max_players: 2, ..Default::default() }
    }

    pub fn is_ready(&self, peer: &PeerId) -> bool {
        if Some(*peer) == self.my_id {
            return self.is_host || self.local_ready;
        }
        self.ready.get(peer).copied().unwrap_or(false)
    }

    /// Host can start when the roster is full and everyone else is ready.
    pub fn can_start(&self) -> bool {
        self.is_host
            && self.roster.len() == self.max_players as usize
            && self.roster.iter().all(|p| self.is_ready(p))
    }

    pub fn set_max_players(&mut self, n: u8) {
        self.max_players = n.clamp(1, MAX_PLAYERS as u8).max(self.roster.len() as u8);
    }
}

/// Final match parameters, kept until the game scene builds the GGRS session.
#[derive(Resource)]
pub struct PendingMatch {
    pub socket: WebRtcSocket,
    pub players: Vec<PeerId>,
    pub local_handle: usize,
    pub seed: u64,
}

const HELLO_EVERY: u32 = 12;

/// Pump the signaling socket and run the lobby protocol.
pub fn lobby_net_update(mut lobby: ResMut<LobbyNet>, socket: Option<ResMut<MatchboxRes>>) {
    let Some(mut socket) = socket else { return };
    if socket.socket.try_update_peers().is_err() {
        return;
    }
    lobby.tick += 1;
    if lobby.my_id.is_none() {
        lobby.my_id = socket.socket.id();
        if let (Some(id), true) = (lobby.my_id, lobby.is_host) {
            lobby.roster = vec![id];
        }
    }
    let Some(my_id) = lobby.my_id else { return };
    let peers: Vec<PeerId> = socket.socket.connected_peers().collect();
    lobby.connected = peers.len() + 1;

    // Incoming lobby messages.
    let incoming = match socket.socket.get_channel_mut(LOBBY_CHANNEL) {
        Ok(ch) => ch.receive(),
        Err(_) => Vec::new(),
    };
    for (from, bytes) in incoming {
        let Some(msg) = decode(&bytes) else { continue };
        match msg {
            LobbyMsg::Hello { ready } => {
                lobby.ready.insert(from, ready);
                if lobby.is_host && !lobby.roster.contains(&from) && lobby.roster.len() < lobby.max_players as usize {
                    lobby.roster.push(from);
                }
            }
            LobbyMsg::Roster { max_players, players, ready } if !lobby.is_host => {
                lobby.max_players = max_players;
                for (p, r) in players.iter().zip(ready) {
                    lobby.ready.insert(*p, r);
                }
                lobby.roster = players;
                lobby.rejected = !lobby.roster.contains(&my_id) && lobby.roster.len() >= max_players as usize;
            }
            LobbyMsg::Start { seed, players } if !lobby.is_host
                && lobby.started.is_none() => {
                    lobby.started = Some(StartInfo { seed, players });
                }
            _ => {}
        }
    }

    // Host: drop players who left the room (before the match has started).
    if lobby.is_host && lobby.started.is_none() {
        lobby.roster.retain(|p| *p == my_id || peers.contains(p));
        let n = lobby.roster.len() as u8;
        lobby.max_players = lobby.max_players.max(n);
    }

    // Host: start the match.
    if lobby.is_host && lobby.start_requested && lobby.can_start() && lobby.started.is_none() {
        let seed = rand::random::<u64>();
        let players = lobby.roster.clone();
        let msg = encode(&LobbyMsg::Start { seed, players: players.clone() });
        if let Ok(ch) = socket.socket.get_channel_mut(LOBBY_CHANNEL) {
            for p in players.iter().filter(|p| **p != my_id) {
                let _ = ch.try_send(msg.clone(), *p);
            }
        }
        lobby.started = Some(StartInfo { seed, players });
    }
    lobby.start_requested = false;

    // Heartbeats.
    if lobby.tick.is_multiple_of(HELLO_EVERY) && lobby.started.is_none() {
        let hello = encode(&LobbyMsg::Hello { ready: lobby.local_ready });
        let roster_msg = lobby.is_host.then(|| {
            encode(&LobbyMsg::Roster {
                max_players: lobby.max_players,
                players: lobby.roster.clone(),
                ready: lobby.roster.iter().map(|p| lobby.is_ready(p)).collect(),
            })
        });
        if let Ok(ch) = socket.socket.get_channel_mut(LOBBY_CHANNEL) {
            for p in &peers {
                let _ = ch.try_send(hello.clone(), *p);
                if let Some(r) = &roster_msg {
                    let _ = ch.try_send(r.clone(), *p);
                }
            }
        }
    }
}

/// Once a `Start` is known, hand the socket over and enter the game.
pub fn finish_lobby(world: &mut World) {
    let Some(start) = world.resource_mut::<LobbyNet>().started.take() else { return };
    let my_id = world.resource::<LobbyNet>().my_id;
    let Some(local_handle) = my_id.and_then(|id| start.players.iter().position(|p| *p == id)) else {
        world.resource_mut::<LobbyNet>().rejected = true;
        return;
    };
    let Some(res) = world.remove_resource::<MatchboxRes>() else { return };
    world.insert_resource(PendingMatch {
        socket: res.socket,
        players: start.players,
        local_handle,
        seed: start.seed,
    });
    world.resource_mut::<NextState<GameState>>().set(GameState::InGame);
}

// ------------------------------------------------------------ session building

/// Build the online P2P session for `pending`. Slot index == player handle.
pub fn build_p2p_session(pending: PendingMatch) -> Session<BattleCityConfig> {
    let n = pending.players.len();
    let mut builder = SessionBuilder::<BattleCityConfig>::new()
        .with_num_players(n)
        .expect("player count")
        .with_input_delay(2)
        .with_max_prediction_window(10)
        .with_fps(60)
        .expect("fps")
        .with_desync_detection_mode(DesyncDetection::On { interval: 10 })
        .with_disconnect_timeout(Duration::from_secs(4))
        .with_disconnect_notify_delay(Duration::from_secs(1));
    for (handle, peer) in pending.players.iter().enumerate() {
        let kind = if handle == pending.local_handle { PlayerType::Local } else { PlayerType::Remote(*peer) };
        builder = builder.add_player(kind, handle).expect("add player");
    }
    let socket = MatchboxGgrsSocket { socket: pending.socket };
    Session::P2P(builder.start_p2p_session(socket).expect("p2p session"))
}

/// Offline play goes through the *same* simulation, driven by a GGRS sync-test
/// session that rolls back and re-checks checksums every frame in debug builds.
pub fn build_local_session() -> Session<BattleCityConfig> {
    let builder = SessionBuilder::<BattleCityConfig>::new()
        .with_num_players(1)
        .expect("player count")
        .with_check_distance(if cfg!(debug_assertions) { 2 } else { 1 })
        .with_input_delay(0);
    Session::SyncTest(builder.start_synctest_session().expect("synctest session"))
}
