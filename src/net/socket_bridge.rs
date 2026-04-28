use bevy_ggrs::ggrs::{Message, NonBlockingSocket};
use matchbox_socket::{PeerId, WebRtcSocket};

/// Bridges matchbox_socket's `WebRtcSocket` to ggrs 0.12's `NonBlockingSocket<PeerId>` trait.
/// Handles serialization of ggrs messages over the matchbox unreliable channel.
pub struct MatchboxGgrsSocket {
    pub socket: WebRtcSocket,
}

impl NonBlockingSocket<PeerId> for MatchboxGgrsSocket {
    fn send_to(&mut self, msg: &Message, addr: &PeerId) {
        let buf = serde_json::to_vec(msg).expect("failed to serialize ggrs message");
        let channel = self.socket.channel_mut(0);
        channel.send(buf.into_boxed_slice(), *addr);
    }

    fn receive_all_messages(&mut self) -> Vec<(PeerId, Message)> {
        let channel = self.socket.channel_mut(0);
        channel
            .receive()
            .into_iter()
            .filter_map(|(peer, data)| {
                serde_json::from_slice(&data)
                    .ok()
                    .map(|msg| (peer, msg))
            })
            .collect()
    }
}
