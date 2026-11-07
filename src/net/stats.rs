//! Live network statistics for the debug overlay and the desync detector.

use bevy::prelude::*;
use bevy_ggrs::prelude::*;

use super::input::BattleCityConfig;

#[derive(Resource, Default, Debug)]
pub struct NetStats {
    pub desyncs: u32,
    pub last_desync_frame: Option<i32>,
    /// `(player handle, round-trip ms)` for each remote player.
    pub pings: Vec<(usize, u128)>,
    pub frames_ahead: i32,
    pub current_frame: i32,
    pub confirmed_frame: i32,
    /// Handles whose peers dropped; the simulation runs them as AI.
    pub dropped: Vec<usize>,
    pub interrupted_ms: Option<u128>,
    pub synchronized: bool,
}

/// Drain GGRS events and refresh [`NetStats`]. Disconnects do not end the
/// match: GGRS marks the handle's input as `Disconnected` and the simulation
/// hands that tank to the AI on every peer at the same frame.
pub fn pump_session(mut session: ResMut<Session<BattleCityConfig>>, mut stats: ResMut<NetStats>) {
    let Session::P2P(s) = &mut *session else { return };
    let events: Vec<_> = s.events().collect();
    for event in events {
        match event {
            GgrsEvent::DesyncDetected { frame, local_checksum, remote_checksum, .. } => {
                stats.desyncs += 1;
                stats.last_desync_frame = Some(frame);
                error!("DESYNC at frame {frame}: local {local_checksum:#x} remote {remote_checksum:#x}");
            }
            GgrsEvent::Disconnected { addr } => {
                for h in s.handles_by_address(addr) {
                    if !stats.dropped.contains(&h) {
                        warn!("player {h} disconnected; AI takes over");
                        stats.dropped.push(h);
                    }
                }
            }
            GgrsEvent::NetworkInterrupted { disconnect_timeout, .. } => stats.interrupted_ms = Some(disconnect_timeout),
            GgrsEvent::NetworkResumed { .. } => stats.interrupted_ms = None,
            GgrsEvent::Synchronized { .. } => stats.synchronized = true,
            _ => {}
        }
    }
    stats.pings.clear();
    for h in s.remote_player_handles() {
        if let Ok(st) = s.network_stats(h) {
            stats.pings.push((h, st.ping));
        }
    }
    stats.frames_ahead = s.frames_ahead();
    stats.current_frame = s.current_frame();
    stats.confirmed_frame = s.confirmed_frame();
}
