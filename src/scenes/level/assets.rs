use bevy::prelude::*;

use crate::core::config::{TILE_SIZE, MAP_WIDTH, MAP_HEIGHT};

use super::systems::LevelEntity;

#[derive(Component)]
pub struct StageIntroEntity;

#[derive(Component)]
pub struct ProgressBarFill;

#[derive(Resource)]
pub struct LevelAssets {
    pub handles: Vec<UntypedHandle>,
}

#[derive(Resource)]
pub struct StageIntroTimer(pub Timer);

pub fn start_stage_intro(mut commands: Commands, asset_server: Res<AssetServer>) {
    let map_w = MAP_WIDTH as f32 * TILE_SIZE;
    let map_h = MAP_HEIGHT as f32 * TILE_SIZE;

    // Camera
    commands.spawn((
        Camera2d,
        bevy::camera::Projection::Orthographic(bevy::camera::OrthographicProjection {
            scaling_mode: bevy::camera::ScalingMode::AutoMin {
                min_width: map_w,
                min_height: map_h,
            },
            ..bevy::camera::OrthographicProjection::default_2d()
        }),
        LevelEntity,
    ));

    // Gray border clear color
    commands.insert_resource(ClearColor(Color::srgb(0.75, 0.75, 0.75)));

    // Black background covering the playable map area
    commands.spawn((
        Sprite {
            color: Color::BLACK,
            custom_size: Some(Vec2::new(map_w, map_h)),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, -1.0)),
        LevelEntity,
    ));

    // NES-style "STAGE  1" text centered
    commands.spawn((
        Text::new("STAGE  1"),
        TextFont {
            font_size: 40.0,
            ..default()
        },
        TextColor(Color::WHITE),
        TextLayout::new_with_justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(45.0),
            width: Val::Percent(100.0),
            ..default()
        },
        StageIntroEntity,
        LevelEntity,
    ));

    // Progress bar background (dark gray)
    let bar_width = map_w * 0.5;
    let bar_height = 8.0;
    let bar_y = -map_h * 0.08;

    commands.spawn((
        Sprite {
            color: Color::srgb(0.2, 0.2, 0.2),
            custom_size: Some(Vec2::new(bar_width, bar_height)),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, bar_y, 0.5)),
        StageIntroEntity,
        LevelEntity,
    ));

    // Progress bar fill (white, starts at 0 width)
    commands.spawn((
        Sprite {
            color: Color::WHITE,
            custom_size: Some(Vec2::new(0.0, bar_height - 2.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(-(bar_width / 2.0), bar_y, 1.0)),
        ProgressBarFill,
        StageIntroEntity,
        LevelEntity,
    ));

    // Preload all game assets
    let mut handles: Vec<UntypedHandle> = Vec::new();

    // Tiles
    let tile_paths = [
        "sprites/tiles/brick_full.png",
        "sprites/tiles/steel_full.png",
        "sprites/tiles/water_f0.png",
        "sprites/tiles/trees.png",
        "sprites/tiles/ice.png",
        "sprites/tiles/eagle_alive.png",
        "sprites/tiles/eagle_destroyed.png",
    ];
    for path in tile_paths {
        handles.push(asset_server.load::<Image>(path).untyped());
    }

    // Spawn animation frames
    for i in 0..4 {
        handles.push(asset_server.load::<Image>(format!("sprites/spawn/spawn_f{i}.png")).untyped());
    }

    // Blast/explosion frames
    let blast_paths = [
        "sprites/blast/blast_small_0.png",
        "sprites/blast/blast_small_1.png",
        "sprites/blast/blast_small_2.png",
        "sprites/blast/blast_big_0.png",
        "sprites/blast/blast_big_1.png",
    ];
    for path in blast_paths {
        handles.push(asset_server.load::<Image>(path).untyped());
    }

    // Bullets
    let bullet_paths = [
        "sprites/bullets/bullet_up.png",
        "sprites/bullets/bullet_down.png",
        "sprites/bullets/bullet_left.png",
        "sprites/bullets/bullet_right.png",
    ];
    for path in bullet_paths {
        handles.push(asset_server.load::<Image>(path).untyped());
    }

    // Player tanks (player1 & player2, 4 levels, 4 directions, 2 frames)
    let directions = ["up", "down", "left", "right"];
    for player in ["player1", "player2"] {
        for level in 1..=4 {
            for dir in &directions {
                for frame in 0..=1 {
                    handles.push(
                        asset_server
                            .load::<Image>(format!(
                                "sprites/tanks/{player}/level{level}/{dir}_f{frame}.png"
                            ))
                            .untyped(),
                    );
                }
            }
        }
    }

    // Enemy tanks (basic, fast, power, armor × 4 directions × 2 frames)
    for enemy_type in ["basic", "fast", "power", "armor"] {
        for dir in &directions {
            for frame in 0..=1 {
                handles.push(
                    asset_server
                        .load::<Image>(format!(
                            "sprites/tanks/enemy/{enemy_type}/{dir}_f{frame}.png"
                        ))
                        .untyped(),
                );
            }
        }
    }

    // Audio
    handles.push(asset_server.load::<AudioSource>("levels/background.mp3").untyped());

    commands.insert_resource(LevelAssets { handles });
    commands.insert_resource(StageIntroTimer(Timer::from_seconds(2.0, TimerMode::Once)));
}

pub fn check_assets_ready(
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut timer: ResMut<StageIntroTimer>,
    assets: Res<LevelAssets>,
    mut next_phase: ResMut<NextState<crate::core::states::InGamePhase>>,
    mut bar_query: Query<(&mut Sprite, &mut Transform), With<ProgressBarFill>>,
) {
    timer.0.tick(time.delta());

    // Calculate loading progress
    let total = assets.handles.len();
    let loaded = assets.handles.iter()
        .filter(|handle| asset_server.is_loaded_with_dependencies(handle.id()))
        .count();
    let progress = if total > 0 { loaded as f32 / total as f32 } else { 1.0 };

    // Update progress bar fill
    let bar_max_width = MAP_WIDTH as f32 * TILE_SIZE * 0.5;
    let fill_width = bar_max_width * progress;
    for (mut sprite, mut transform) in &mut bar_query {
        sprite.custom_size = Some(Vec2::new(fill_width, sprite.custom_size.unwrap_or_default().y));
        // Anchor bar fill to the left edge of the background bar
        transform.translation.x = -(bar_max_width / 2.0) + (fill_width / 2.0);
    }

    if !timer.0.is_finished() {
        return;
    }

    if loaded == total {
        next_phase.set(crate::core::states::InGamePhase::Playing);
    }
}

pub fn cleanup_stage_intro(
    mut commands: Commands,
    query: Query<Entity, With<StageIntroEntity>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<StageIntroTimer>();
    commands.remove_resource::<LevelAssets>();
}
