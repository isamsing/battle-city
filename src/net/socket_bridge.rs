use bevy_ggrs::ggrs::{Message, NonBlockingSocket};
use bevy::prelude::*;
use matchbox_socket::{PeerId, WebRtcSocket};

use super::protocol::GGRS_CHANNEL;

/// Bridges matchbox_socket's `WebRtcSocket` to ggrs 0.12's `NonBlockingSocket<PeerId>`.
///
/// GGRS messages are serialized with bincode (much smaller than the JSON used
/// before, which matters for 4-way mesh traffic) and sent over the unreliable,
/// unordered matchbox channel.
pub struct MatchboxGgrsSocket {
    pub socket: WebRtcSocket,
}

impl NonBlockingSocket<PeerId> for MatchboxGgrsSocket {
    fn send_to(&mut self, msg: &Message, addr: &PeerId) {
        let Ok(buf) = bincode::serialize(msg) else {
            warn!("failed to serialize ggrs message");
            return;
        };
        let Ok(channel) = self.socket.get_channel_mut(GGRS_CHANNEL) else {
            return;
        };
        // A closed channel means the peer left; GGRS notices via its own timeout.
        let _ = channel.try_send(buf.into_boxed_slice(), *addr);
    }

    fn receive_all_messages(&mut self) -> Vec<(PeerId, Message)> {
        let Ok(channel) = self.socket.get_channel_mut(GGRS_CHANNEL) else {
            return Vec::new();
        };
        channel
            .receive()
            .into_iter()
            .filter_map(|(peer, data)| bincode::deserialize(&data).ok().map(|msg| (peer, msg)))
            .collect()
    }
}
