use battle_city::scenes::tank::components::*;

#[test]
fn tank_state_default_is_spawning() {
    assert_eq!(TankState::default(), TankState::Spawning);
}

#[test]
fn tank_state_variants_are_distinct() {
    assert_ne!(TankState::Spawning, TankState::Active);
}

#[test]
fn spawn_animation_new_starts_at_frame_zero() {
    let anim = SpawnAnimation::new();
    assert_eq!(anim.frame, 0);
}

#[test]
fn spawn_animation_has_4_frames() {
    assert_eq!(SpawnAnimation::TOTAL_FRAMES, 4);
}

#[test]
fn spawn_animation_frame_duration_is_100ms() {
    let anim = SpawnAnimation::new();
    let duration_ms = anim.timer.duration().as_secs_f64() * 1000.0;
    assert!((duration_ms - 100.0).abs() < 0.01, "expected ~100ms, got {duration_ms}ms");
}
