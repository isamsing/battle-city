use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    Loading,
    #[default]
    Menu,
    InGame,
    Paused,
    GameOver,
}

#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(GameState = GameState::Menu)]
pub enum MenuScreen {
    #[default]
    Title,
    ModeSelect,
    Lobby,
    TankSelect,
}

#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(GameState = GameState::InGame)]
pub enum InGamePhase {
    #[default]
    StageIntro,
    Playing,
    StageComplete,
}

#[derive(Resource)]
pub struct WinnerInfo {
    pub winner_handle: usize,
}
