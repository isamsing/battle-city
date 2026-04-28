pub mod input;
pub mod session;
pub mod socket_bridge;

use bevy::prelude::*;
use bevy_ggrs::prelude::*;

use crate::core::states::MenuScreen;
use input::BattleCityConfig;

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameMode>()
            .add_plugins(GgrsPlugin::<BattleCityConfig>::default())
            .insert_resource(RollbackFrameRate(60))
            .add_systems(ReadInputs, input::read_local_inputs)
            .add_systems(
                OnEnter(MenuScreen::Lobby),
                session::start_matchbox_socket.run_if(is_networked),
            )
            .add_systems(
                Update,
                session::wait_for_peers
                    .run_if(in_state(MenuScreen::Lobby))
                    .run_if(is_networked),
            );
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
