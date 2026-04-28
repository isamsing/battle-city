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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_state_default_is_menu() {
        assert_eq!(GameState::default(), GameState::Menu);
    }

    #[test]
    fn menu_screen_default_is_title() {
        assert_eq!(MenuScreen::default(), MenuScreen::Title);
    }

    #[test]
    fn in_game_phase_default_is_stage_intro() {
        assert_eq!(InGamePhase::default(), InGamePhase::StageIntro);
    }

    #[test]
    fn game_state_variants_are_distinct() {
        let variants = [
            GameState::Loading,
            GameState::Menu,
            GameState::InGame,
            GameState::Paused,
            GameState::GameOver,
        ];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j]);
            }
        }
    }
}
