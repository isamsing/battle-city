pub mod input;
pub mod protocol;
pub mod session;
pub mod socket_bridge;
pub mod stats;

use bevy::prelude::*;
use bevy_ggrs::prelude::*;

use crate::core::states::MenuScreen;
use input::BattleCityConfig;

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameMode>()
            .init_resource::<session::ServerUrl>()
            .init_resource::<stats::NetStats>()
            .add_plugins(GgrsPlugin::<BattleCityConfig>::default())
            .insert_resource(RollbackFrameRate(60))
            .add_systems(ReadInputs, input::read_local_inputs)
            .add_systems(
                Update,
                (session::lobby_net_update, session::finish_lobby)
                    .chain()
                    .run_if(in_state(MenuScreen::Lobby))
                    .run_if(resource_exists::<session::LobbyNet>),
            )
            .add_systems(
                Update,
                stats::pump_session.run_if(resource_exists::<Session<BattleCityConfig>>),
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
