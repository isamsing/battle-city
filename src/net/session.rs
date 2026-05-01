use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy::tasks::IoTaskPool;
use bevy_ggrs::prelude::*;
use matchbox_socket::{PeerId, WebRtcSocket};

use crate::core::states::GameState;
use super::input::BattleCityConfig;
use super::socket_bridge::MatchboxGgrsSocket;
use super::GameMode;

#[derive(Resource, Clone)]
pub struct ServerUrl(pub String);

impl Default for ServerUrl {
    fn default() -> Self {
        Self("wss://battle-city.fly.dev".to_string())
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
    (0..4)
        .map(|_| CHARS[rng.random_range(0..CHARS.len())] as char)
        .collect()
}

pub fn start_matchbox_socket(mut commands: Commands, mode: Res<GameMode>, server_url: Res<ServerUrl>) {
    let room_code = match &*mode {
        GameMode::OnlineHost(code) => code.clone(),
        GameMode::OnlineJoin(code) => code.clone(),
        GameMode::Local => return,
    };

    let room_url = format!("{}/battle_city_{room_code}?next=2", server_url.0);
    let (socket, loop_fut) = WebRtcSocket::new_unreliable(room_url);

    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async move {
        loop_fut.await.expect("matchbox socket error");
    });

    #[cfg(not(target_arch = "wasm32"))]
    {
        let task_pool = IoTaskPool::get();
        task_pool.spawn(loop_fut).detach();
    }

    commands.insert_resource(MatchboxRes { socket });
}

pub fn wait_for_peers(
    mut commands: Commands,
    socket: Option<ResMut<MatchboxRes>>,
    mut next_state: ResMut<NextState<GameState>>,
    mode: Res<GameMode>,
) {
    let Some(mut socket) = socket else { return };

    socket.socket.update_peers();
    let peers: Vec<PeerId> = socket.socket.connected_peers().collect();

    if peers.is_empty() {
        return;
    }

    let local_handle: usize = match *mode {
        GameMode::OnlineHost(_) => 0,
        GameMode::OnlineJoin(_) => 1,
        GameMode::Local => return,
    };
    let remote_handle = 1 - local_handle;

    let mut session_builder = SessionBuilder::<BattleCityConfig>::new()
        .with_num_players(2)
        .expect("failed to set num players")
        .with_input_delay(2);

    session_builder = session_builder
        .add_player(PlayerType::Local, local_handle)
        .expect("failed to add local player");
    session_builder = session_builder
        .add_player(PlayerType::Remote(peers[0]), remote_handle)
        .expect("failed to add remote player");

    // Extract the raw socket and wrap for ggrs 0.12
    let raw_socket = std::mem::replace(
        &mut socket.socket,
        // Placeholder — we're about to remove this resource
        WebRtcSocket::new_unreliable("ws://placeholder").0,
    );
    let ggrs_socket = MatchboxGgrsSocket { socket: raw_socket };

    let session = session_builder
        .start_p2p_session(ggrs_socket)
        .expect("failed to start P2P session");

    commands.remove_resource::<MatchboxRes>();
    commands.insert_resource(Session::P2P(session));
    next_state.set(GameState::InGame);
}
