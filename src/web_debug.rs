//! Publishes a small read-only status object on `window.__bc` (web builds only)
//! so browser automation and DevTools can read lobby/sim state that is
//! otherwise drawn onto the canvas. Has no effect on gameplay.

use bevy::prelude::*;

pub struct WebDebugPlugin;

impl Plugin for WebDebugPlugin {
    fn build(&self, _app: &mut App) {
        #[cfg(target_arch = "wasm32")]
        _app.add_systems(Update, publish);
    }
}

#[cfg(target_arch = "wasm32")]
fn publish(
    lobby: Option<Res<crate::net::session::LobbyNet>>,
    mode: Res<crate::net::GameMode>,
    sim: Option<Res<crate::scenes::level::systems::SimRes>>,
    stats: Res<crate::net::stats::NetStats>,
    mut tick: Local<u32>,
) {
    *tick += 1;
    if *tick % 15 != 0 {
        return;
    }
    let room = match &*mode {
        crate::net::GameMode::OnlineHost(c) | crate::net::GameMode::OnlineJoin(c) => c.clone(),
        crate::net::GameMode::Local => String::new(),
    };
    let (frame, checksum, phase, players, enemies, stage) = match sim.as_deref() {
        Some(s) => (
            s.0.frame as i64,
            format!("{:016x}", s.0.checksum()),
            format!("{:?}", s.0.phase),
            s.0.players.iter().map(|p| format!("{:?}:{}:{}", p.status, p.lives, p.score)).collect::<Vec<_>>().join(","),
            s.0.enemies.len() as i64,
            s.0.stage_number as i64,
        ),
        None => (-1, String::new(), String::new(), String::new(), 0, 0),
    };
    let (roster, ready, connected) = match lobby.as_deref() {
        Some(l) => (l.roster.len() as i64, l.roster.iter().filter(|p| l.is_ready(p)).count() as i64, l.connected as i64),
        None => (-1, -1, -1),
    };
    let json = format!(
        "{{\"room\":\"{room}\",\"frame\":{frame},\"checksum\":\"{checksum}\",\"phase\":\"{phase}\",\"players\":\"{players}\",\
         \"enemies\":{enemies},\"stage\":{stage},\"roster\":{roster},\"ready\":{ready},\"connected\":{connected},\
         \"desyncs\":{},\"dropped\":{},\"netFrame\":{},\"confirmed\":{}}}",
        stats.desyncs,
        stats.dropped.len(),
        stats.current_frame,
        stats.confirmed_frame,
    );
    if let Some(window) = web_sys::window() {
        let _ = js_sys::Reflect::set(&window, &"__bc".into(), &js_sys::JSON::parse(&json).unwrap_or(wasm_bindgen::JsValue::NULL));
    }
}
