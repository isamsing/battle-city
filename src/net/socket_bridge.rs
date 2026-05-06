use bevy_ggrs::ggrs::{Message, NonBlockingSocket};
use bevy::prelude::*;
use matchbox_socket::{PeerId, WebRtcSocket};

/// Bridges matchbox_socket's `WebRtcSocket` to ggrs 0.12's `NonBlockingSocket<PeerId>` trait.
/// Handles serialization of ggrs messages over the matchbox unreliable channel.
pub struct MatchboxGgrsSocket {
    pub socket: WebRtcSocket,
}

/// Inserted when the WebRTC socket channel is detected as closed (peer disconnected).
#[derive(Resource)]
pub struct SocketDead;

impl NonBlockingSocket<PeerId> for MatchboxGgrsSocket {
    fn send_to(&mut self, msg: &Message, addr: &PeerId) {
        let buf = serde_json::to_vec(msg).expect("failed to serialize ggrs message");
        let Ok(channel) = self.socket.get_channel_mut(0) else {
            warn!("WebRTC channel unavailable, dropping outgoing message");
            return;
        };
        if let Err(_e) = channel.try_send(buf.into_boxed_slice(), *addr) {
            warn!("WebRTC send failed (channel closed), dropping message");
        }
    }

    fn receive_all_messages(&mut self) -> Vec<(PeerId, Message)> {
        let Ok(channel) = self.socket.get_channel_mut(0) else {
            return Vec::new();
        };
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
