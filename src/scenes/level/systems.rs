//! In-game scene: owns the rollback `Sim`, drives it from GGRS inputs and
//! renders it. The simulation itself lives in [`crate::sim`] and knows nothing
//! about Bevy; everything here is glue, drawing and UI.

use bevy::prelude::*;
use bevy_ggrs::ggrs::InputStatus;
use bevy_ggrs::prelude::*;

use crate::audio_resume::UserInteractionState;
use crate::core::config::{PLAYER_NAMES, PLAYER_RGB, TILE_SIZE};
use crate::core::states::GameState;
use crate::net::input::BattleCityConfig;
use crate::net::session::{build_local_session, build_p2p_session, PendingMatch};
use crate::net::stats::NetStats;
use crate::net::GameMode;
use crate::scenes::tank::components::ExplosionAnimation;
use crate::sim::*;

#[derive(Component)]
pub struct LevelEntity;

#[derive(Resource)]
pub struct BackgroundMusicPending;

/// The rollback-registered game state.
#[derive(Resource, Clone)]
pub struct SimRes(pub Sim);

/// Audio/VFX notifications from the sim. Not rolled back; frames already seen
/// are skipped so a rollback replay never fires an effect twice.
#[derive(Resource, Default)]
pub struct SimEvents {
    pub pending: Vec<SimEvent>,
    pub max_frame: u32,
}

/// World units per NES pixel (a sim cell is 8px = `TILE_SIZE` world units).
const S: f32 = TILE_SIZE / CELL_PX as f32;
const FIELD_HALF_PX: f32 = (FIELD_PX / 2) as f32;

/// Top-left sub-pixel position + size in NES px -> world-space centre.
fn world(x: i32, y: i32, w_px: f32, h_px: f32) -> Vec2 {
    let px = x as f32 / SUB as f32;
    let py = y as f32 / SUB as f32;
    Vec2::new((px + w_px / 2.0 - FIELD_HALF_PX) * S, -(py + h_px / 2.0 - FIELD_HALF_PX) * S)
}

// ------------------------------------------------------------------ setup

pub fn setup_gameplay(world: &mut World) {
    let pending = world.remove_resource::<PendingMatch>();
    let (sim, session) = match pending {
        Some(p) if p.players.len() > 1 => {
            let sim = Sim::new(p.players.len(), p.seed);
            (sim, build_p2p_session(p))
        }
        Some(p) => (Sim::new(1, p.seed), build_local_session()),
        None => (Sim::new(1, rand::random()), build_local_session()),
    };
    world.insert_resource(SimRes(sim));
    world.insert_resource(session);
    world.insert_resource(SimEvents::default());
    world.insert_resource(TerrainView::default());
    world.insert_resource(BackgroundMusicPending);
    world.insert_resource(NetStats::default());
    world.insert_resource(DebugOverlay(false));

    let asset_server = world.resource::<AssetServer>().clone();
    let mut commands = world.commands();
    commands.spawn((
        Sprite { image: asset_server.load("sprites/tiles/eagle_alive.png"), custom_size: Some(Vec2::splat(TILE_SIZE * 2.0)), ..default() },
        Transform::from_translation(world_eagle().extend(0.6)),
        EagleView,
        LevelEntity,
    ));
    spawn_hud(&mut commands);
}

fn world_eagle() -> Vec2 {
    world(6 * TANK, 12 * TANK, TILE_PX as f32, TILE_PX as f32)
}

// ------------------------------------------------------------- simulation

/// Runs inside `GgrsSchedule`: exactly one sim step per confirmed/predicted frame.
pub fn advance_sim(
    inputs: Res<PlayerInputs<BattleCityConfig>>,
    mut sim: ResMut<SimRes>,
    mut events: ResMut<SimEvents>,
) {
    let mut arr = [StepInput::default(); MAX_PLAYERS];
    for (i, slot) in arr.iter_mut().enumerate().take(inputs.len()) {
        let (input, status) = inputs[i];
        *slot = StepInput { bits: input.0, human: status != InputStatus::Disconnected };
    }
    for ev in sim.0.step(&arr) {
        if ev.frame > events.max_frame {
            events.pending.push(ev);
        }
    }
    events.max_frame = events.max_frame.max(sim.0.frame);
}

pub fn log_sync_test_mismatch(trigger: On<SyncTestMismatch>) {
    error!("rollback self-check FAILED: frames {:?} differ", trigger.event().mismatched_frames);
}

// -------------------------------------------------------------- rendering

#[derive(Component)]
pub struct DynSprite;
#[derive(Component)]
pub struct EagleView;
#[derive(Component)]
pub struct Blast;

/// Cached copy of the terrain so only changed cells touch the ECS.
#[derive(Resource, Default)]
pub struct TerrainView {
    cached: Vec<Cell>,
    entities: Vec<Option<Entity>>,
}

fn cell_image(cell: Cell) -> Option<(&'static str, f32)> {
    match cell {
        Cell::Brick => Some(("sprites/tiles/brick_full.png", 0.5)),
        Cell::Steel => Some(("sprites/tiles/steel_full.png", 0.5)),
        Cell::Water => Some(("sprites/tiles/water_f0.png", 0.4)),
        Cell::Ice => Some(("sprites/tiles/ice.png", 0.2)),
        Cell::Forest => Some(("sprites/tiles/trees.png", 3.0)),
        Cell::Empty | Cell::Eagle => None,
    }
}

pub fn sync_terrain(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    sim: Res<SimRes>,
    mut view: ResMut<TerrainView>,
) {
    let cells = &sim.0.cells;
    if view.cached.len() != cells.len() {
        view.cached = vec![Cell::Empty; cells.len()];
        view.entities = vec![None; cells.len()];
    }
    for (i, &cell) in cells.iter().enumerate() {
        if view.cached[i] == cell {
            continue;
        }
        if let Some(e) = view.entities[i].take() {
            commands.entity(e).despawn();
        }
        view.cached[i] = cell;
        let Some((path, z)) = cell_image(cell) else { continue };
        let (cx, cy) = ((i as i32) % GRID_CELLS, (i as i32) / GRID_CELLS);
        // Each 8px cell shows its quadrant of the 16px tile texture.
        let (qx, qy) = ((cx % 2) as f32 * 8.0, (cy % 2) as f32 * 8.0);
        let e = commands
            .spawn((
                Sprite {
                    image: asset_server.load(path),
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    rect: Some(Rect::new(qx, qy, qx + 8.0, qy + 8.0)),
                    ..default()
                },
                Transform::from_translation(world(cx * CELL, cy * CELL, 8.0, 8.0).extend(z)),
                LevelEntity,
            ))
            .id();
        view.entities[i] = Some(e);
    }
}

pub fn render_dynamic(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    sim: Res<SimRes>,
    old: Query<Entity, With<DynSprite>>,
    mut eagle: Query<&mut Sprite, (With<EagleView>, Without<DynSprite>)>,
) {
    for e in &old {
        commands.entity(e).despawn();
    }
    let s = &sim.0;
    let tank_px = TILE_PX as f32;
    let mut put = |path: String, x: i32, y: i32, color: Color, z: f32| {
        commands.spawn((
            Sprite { image: asset_server.load(path), custom_size: Some(Vec2::splat(TILE_SIZE * 2.0)), color, ..default() },
            Transform::from_translation(world(x, y, tank_px, tank_px).extend(z)),
            DynSprite,
            LevelEntity,
        ));
    };
    let spawn_frame = |frame: u32| format!("sprites/spawn/spawn_f{}.png", (frame / 3) % 4);
    let tread = |x: i32, y: i32| ((x + y) / (SUB * 2)).rem_euclid(2);

    for (i, p) in s.players.iter().enumerate() {
        match p.status {
            PlayerStatus::Spawning => put(spawn_frame(s.frame), p.x, p.y, Color::WHITE, 1.0),
            PlayerStatus::Active => {
                if p.stun > 0 && (s.frame / 4) % 2 == 0 {
                    continue; // stunned tanks blink
                }
                let color = if p.shield > 0 && (s.frame / 4) % 2 == 0 { Color::srgb(0.65, 0.9, 1.0) } else { Color::WHITE };
                let path = format!("sprites/tanks/player{}/level{}/{}_f{}.png", i + 1, p.stars + 1, p.dir.as_str(), tread(p.x, p.y));
                put(path, p.x, p.y, color, 1.0);
            }
            _ => {}
        }
    }
    for e in &s.enemies {
        if e.spawn > 0 {
            put(spawn_frame(s.frame), e.x, e.y, Color::WHITE, 1.0);
            continue;
        }
        let mut color = match (e.kind, e.hp) {
            (EnemyKind::Armor, 4) | (EnemyKind::Armor, 0) => Color::WHITE,
            (EnemyKind::Armor, 3) => Color::srgb(0.5, 1.0, 0.5),
            (EnemyKind::Armor, 2) => Color::srgb(1.0, 1.0, 0.3),
            (EnemyKind::Armor, _) => Color::srgb(1.0, 0.3, 0.3),
            _ => Color::WHITE,
        };
        if e.flashing && (s.frame / 6) % 2 == 0 {
            color = Color::srgb(1.0, 0.35, 0.35);
        }
        let path = format!("sprites/tanks/enemy/{}/{}_f{}.png", e.kind.sprite_dir(), e.dir.as_str(), tread(e.x, e.y));
        put(path, e.x, e.y, color, 1.0);
    }
    if let Some(pu) = s.power {
        if (s.frame / 8) % 2 == 0 {
            put(pu.kind.sprite().to_string(), pu.x, pu.y, Color::WHITE, 1.5);
        }
    }
    for b in &s.bullets {
        commands.spawn((
            Sprite { image: asset_server.load(format!("sprites/bullets/bullet_{}.png", b.dir.as_str())), ..default() },
            Transform::from_translation(world(b.x, b.y, 0.0, 0.0).extend(2.0)).with_scale(Vec3::splat(S)),
            DynSprite,
            LevelEntity,
        ));
    }
    if let Ok(mut sprite) = eagle.single_mut() {
        let path = if s.eagle_alive { "sprites/tiles/eagle_alive.png" } else { "sprites/tiles/eagle_destroyed.png" };
        sprite.image = asset_server.load(path);
    }
}

pub fn spawn_blasts(mut commands: Commands, asset_server: Res<AssetServer>, mut events: ResMut<SimEvents>) {
    for ev in events.pending.drain(..) {
        if !matches!(ev.kind, SimEventKind::EnemyDied | SimEventKind::PlayerDied | SimEventKind::EagleDestroyed) {
            continue;
        }
        let anim = ExplosionAnimation::new();
        commands.spawn((
            Sprite { image: asset_server.load(anim.sprite_path()), custom_size: Some(Vec2::splat(anim.sprite_size())), ..default() },
            Transform::from_translation(world(ev.x, ev.y, 0.0, 0.0).extend(4.0)),
            anim,
            Blast,
            LevelEntity,
        ));
    }
}

pub fn animate_blasts(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut ExplosionAnimation), With<Blast>>,
) {
    for (entity, mut sprite, mut anim) in &mut query {
        anim.timer.tick(time.delta());
        if anim.timer.just_finished() {
            anim.frame += 1;
            if anim.frame >= ExplosionAnimation::TOTAL_FRAMES {
                commands.entity(entity).despawn();
                continue;
            }
            sprite.image = asset_server.load(anim.sprite_path());
            sprite.custom_size = Some(Vec2::splat(anim.sprite_size()));
        }
    }
}

// -------------------------------------------------------------------- HUD

#[derive(Component)]
pub struct PlayerHud(usize);
#[derive(Component)]
pub struct StageHud;
#[derive(Component)]
pub struct BannerHud;
#[derive(Component)]
pub struct DebugHud;

#[derive(Resource)]
pub struct DebugOverlay(pub bool);

fn spawn_hud(commands: &mut Commands) {
    for slot in 0..MAX_PLAYERS {
        let [r, g, b] = PLAYER_RGB[slot];
        commands.spawn((
            Text::new(""),
            TextFont { font_size: 15.0, ..default() },
            TextColor(Color::srgb_u8(r, g, b)),
            Node { position_type: PositionType::Absolute, left: Val::Px(8.0), top: Val::Px(8.0 + slot as f32 * 70.0), ..default() },
            PlayerHud(slot),
            LevelEntity,
        ));
    }
    commands.spawn((
        Text::new(""),
        TextFont { font_size: 16.0, ..default() },
        TextColor(Color::WHITE),
        Node { position_type: PositionType::Absolute, right: Val::Px(8.0), top: Val::Px(8.0), ..default() },
        StageHud,
        LevelEntity,
    ));
    commands.spawn((
        Text::new(""),
        TextFont { font_size: 40.0, ..default() },
        TextColor(Color::srgb(1.0, 0.9, 0.3)),
        TextLayout::new_with_justify(Justify::Center),
        Node { position_type: PositionType::Absolute, top: Val::Percent(42.0), width: Val::Percent(100.0), ..default() },
        BannerHud,
        LevelEntity,
    ));
    commands.spawn((
        Text::new(""),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgb(0.6, 1.0, 0.6)),
        Node { position_type: PositionType::Absolute, left: Val::Px(8.0), bottom: Val::Px(8.0), ..default() },
        Visibility::Hidden,
        DebugHud,
        LevelEntity,
    ));
}

pub fn update_hud(
    sim: Res<SimRes>,
    mut players: Query<(&PlayerHud, &mut Text, &mut Visibility), (Without<StageHud>, Without<BannerHud>, Without<DebugHud>)>,
    mut stage: Query<&mut Text, (With<StageHud>, Without<PlayerHud>, Without<BannerHud>)>,
    mut banner: Query<&mut Text, (With<BannerHud>, Without<PlayerHud>, Without<StageHud>)>,
    stats: Res<NetStats>,
    mode: Res<GameMode>,
    session: Option<Res<Session<BattleCityConfig>>>,
) {
    let s = &sim.0;
    let local_handles: Vec<usize> = match session.as_deref() {
        Some(Session::P2P(p)) => p.local_player_handles(),
        _ => vec![0],
    };
    for (hud, mut text, mut vis) in &mut players {
        let p = &s.players[hud.0];
        if p.status == PlayerStatus::Absent {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Inherited;
        let me = if matches!(*mode, GameMode::Local) || !local_handles.contains(&hud.0) { "" } else { " (YOU)" };
        let ai = if stats.dropped.contains(&hud.0) { " [AI]" } else { "" };
        let out = if p.status == PlayerStatus::Out { " OUT" } else { "" };
        **text = format!("P{} {}{}{}{}\nLIVES {}\nSCORE {}", hud.0 + 1, PLAYER_NAMES[hud.0], me, ai, out, p.lives, p.score);
    }
    let left = s.queue.len() + s.enemies.len();
    for mut t in &mut stage {
        **t = format!("STAGE {}\nENEMY {}", s.stage_number + 1, left);
    }
    for mut t in &mut banner {
        **t = match s.phase {
            Phase::Playing => String::new(),
            Phase::StageClear => "STAGE CLEAR".to_string(),
            Phase::GameOver => "GAME OVER\nESC - MENU".to_string(),
        };
    }
}

pub fn toggle_debug(keyboard: Res<ButtonInput<KeyCode>>, mut overlay: ResMut<DebugOverlay>) {
    if keyboard.just_pressed(KeyCode::F3) {
        overlay.0 = !overlay.0;
    }
}

pub fn update_debug(
    time: Res<Time>,
    overlay: Res<DebugOverlay>,
    sim: Res<SimRes>,
    stats: Res<NetStats>,
    mode: Res<GameMode>,
    mut fps: Local<f32>,
    mut query: Query<(&mut Text, &mut Visibility), With<DebugHud>>,
) {
    let dt = time.delta_secs().max(1e-4);
    *fps = if *fps == 0.0 { 1.0 / dt } else { *fps * 0.95 + 0.05 / dt };
    for (mut text, mut vis) in &mut query {
        *vis = if overlay.0 { Visibility::Inherited } else { Visibility::Hidden };
        if !overlay.0 {
            continue;
        }
        let pings = stats.pings.iter().map(|(h, ms)| format!("P{}:{}ms", h + 1, ms)).collect::<Vec<_>>().join(" ");
        let desync = if stats.desyncs > 0 {
            format!("DESYNC x{} (last frame {:?})", stats.desyncs, stats.last_desync_frame)
        } else {
            "in sync".to_string()
        };
        **text = format!(
            "FPS {:.0}  sim frame {}  checksum {:08x}\nmode {:?}\nnet frame {} confirmed {} ahead {}\nping {}\n{}\ndropped {:?}",
            *fps,
            sim.0.frame,
            sim.0.checksum() as u32,
            if matches!(*mode, GameMode::Local) { "offline" } else { "online" },
            stats.current_frame,
            stats.confirmed_frame,
            stats.frames_ahead,
            if pings.is_empty() { "-".to_string() } else { pings },
            desync,
            stats.dropped,
        );
    }
}

// ------------------------------------------------------------------ misc

pub fn spawn_background_music(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    interaction: Res<UserInteractionState>,
    pending: Option<Res<BackgroundMusicPending>>,
) {
    if pending.is_none() || !interaction.interacted {
        return;
    }
    commands.spawn((AudioPlayer::new(asset_server.load("levels/background.mp3")), LevelEntity)).insert(PlaybackSettings::LOOP);
    commands.remove_resource::<BackgroundMusicPending>();
}

pub fn cleanup_level(mut commands: Commands, query: Query<Entity, With<LevelEntity>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

pub fn cleanup_network_session(mut commands: Commands, mut game_mode: ResMut<GameMode>) {
    commands.remove_resource::<Session<BattleCityConfig>>();
    commands.remove_resource::<SimRes>();
    commands.remove_resource::<PendingMatch>();
    commands.remove_resource::<TerrainView>();
    commands.remove_resource::<bevy_ggrs::LocalPlayers>();
    commands.remove_resource::<crate::net::session::MatchboxRes>();
    *game_mode = GameMode::Local;
}

pub fn handle_escape_to_menu(keyboard: Res<ButtonInput<KeyCode>>, mut game_state: ResMut<NextState<GameState>>) {
    if keyboard.just_pressed(KeyCode::Escape) {
        game_state.set(GameState::Menu);
    }
}
