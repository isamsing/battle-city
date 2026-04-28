use battle_city::core::states::GameState;
use battle_city::core::GameCorePlugin;
use bevy::prelude::*;

#[test]
fn game_core_plugin_registers_states() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(GameCorePlugin);
    app.update();

    let state = app.world().resource::<State<GameState>>();
    assert_eq!(*state.get(), GameState::Menu);
}
