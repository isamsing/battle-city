pub mod input;
pub mod session;
pub mod socket_bridge;

use bevy::prelude::*;
use bevy_ggrs::prelude::*;

use crate::core::states::{GameState, MenuScreen};
use input::BattleCityConfig;

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameMode>()
            .init_resource::<session::ServerUrl>()
            .add_plugins(GgrsPlugin::<BattleCityConfig>::default())
            .insert_resource(RollbackFrameRate(60))
            .add_systems(ReadInputs, input::read_local_inputs)
            .add_systems(
                Update,
                session::wait_for_peers
                    .run_if(in_state(MenuScreen::Lobby))
                    .run_if(is_networked),
            )
            .add_systems(
                Update,
                handle_peer_disconnect
                    .run_if(in_state(GameState::InGame))
                    .run_if(is_networked),
            );
    }
}

fn handle_peer_disconnect(
    mut session: ResMut<Session<BattleCityConfig>>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    let Session::P2P(s) = &mut *session else { return };
    for event in s.events() {
        if matches!(event, GgrsEvent::Disconnected { .. }) {
            warn!("Peer disconnected, returning to menu");
            game_state.set(GameState::Menu);
            return;
        }
    }
}

#[derive(Resource, Default, Clone, PartialEq, Eq, Debug)]
pub enum GameMode {
    #[default]
    Local,
    OnlineHost(String),
    OnlineJoin(String),
}

pub fn is_networked(mode: Res<GameMode>) -> bool {
    !matches!(*mode, GameMode::Local)
}
