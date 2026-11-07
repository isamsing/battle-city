//! Lobby messages, exchanged over the reliable matchbox channel before the
//! match starts. All of them are idempotent heartbeats except `Start`.

use matchbox_socket::PeerId;
use serde::{Deserialize, Serialize};

/// Channel 0 carries GGRS traffic (unreliable), channel 1 the lobby (reliable).
pub const GGRS_CHANNEL: usize = 0;
pub const LOBBY_CHANNEL: usize = 1;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum LobbyMsg {
    /// Sent by every peer a few times a second.
    Hello { ready: bool },
    /// Sent by the host: authoritative slot order (index = player handle).
    Roster { max_players: u8, players: Vec<PeerId>, ready: Vec<bool> },
    /// Sent once by the host: begin the match with this seed and slot order.
    Start { seed: u64, players: Vec<PeerId> },
}

pub fn encode(msg: &LobbyMsg) -> Box<[u8]> {
    serde_json::to_vec(msg).expect("lobby message serializes").into_boxed_slice()
}

pub fn decode(bytes: &[u8]) -> Option<LobbyMsg> {
    serde_json::from_slice(bytes).ok()
}
