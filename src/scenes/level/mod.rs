pub(crate) mod assets;
pub(crate) mod systems;

use bevy::prelude::*;
use bevy_ggrs::prelude::*;
use bevy_ggrs::ResourceChecksumPlugin;

use crate::core::states::{GameState, InGamePhase};
use assets::{check_assets_ready, cleanup_stage_intro, start_stage_intro};
use systems::*;

fn reset_clear_color(mut commands: Commands) {
    commands.insert_resource(ClearColor(Color::BLACK));
}

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app
            // The whole game state is one rollback resource, hashed every frame
            // so GGRS can detect desyncs between peers.
            .rollback_resource_with_clone::<SimRes>()
            .add_plugins(ResourceChecksumPlugin::<SimRes>(|s| s.0.checksum()))
            .add_observer(log_sync_test_mismatch)
            // Stage intro: preload assets and show "STAGE 1"
            .add_systems(OnEnter(GameState::InGame), start_stage_intro)
            .add_systems(Update, check_assets_ready.run_if(in_state(InGamePhase::StageIntro)))
            .add_systems(OnExit(InGamePhase::StageIntro), cleanup_stage_intro)
            .add_systems(OnEnter(InGamePhase::Playing), setup_gameplay)
            .add_systems(OnExit(GameState::InGame), (reset_clear_color, cleanup_level, cleanup_network_session))
            // One deterministic step per GGRS frame, for offline and online alike.
            .add_systems(GgrsSchedule, advance_sim.run_if(resource_exists::<SimRes>))
            // Rendering and UI only read the sim.
            .add_systems(
                Update,
                (sync_terrain, render_dynamic, spawn_blasts, animate_blasts, update_hud, toggle_debug, update_debug, spawn_background_music)
                    .chain()
                    .run_if(in_state(InGamePhase::Playing))
                    .run_if(resource_exists::<SimRes>),
            )
            .add_systems(Update, handle_escape_to_menu.run_if(in_state(GameState::InGame)));
    }
}
