use super::*;

#[test]
fn kill_marker_shows_then_expires() {
    let mut fx = HitFx::default();
    assert!(!fx.kill_marker_on());
    fx.on_kill(true);
    assert!(fx.kill_marker_on());
    assert!(fx.kill_headshot);
    fx.tick(KILL_MARKER_SECS * 0.5);
    assert!(fx.kill_marker_on());
    fx.tick(KILL_MARKER_SECS);
    assert!(!fx.kill_marker_on());
}

#[test]
fn a_new_kill_restarts_the_marker() {
    let mut fx = HitFx::default();
    fx.on_kill(true);
    fx.tick(KILL_MARKER_SECS * 0.9);
    fx.on_kill(false);
    assert!(!fx.kill_headshot);
    fx.tick(KILL_MARKER_SECS * 0.5);
    assert!(fx.kill_marker_on());
}

#[test]
fn particles_stay_in_a_fixed_pool() {
    let mut fx = HitFx::default();
    fx.impact(Vec3::ZERO, Vec3::Y, ImpactKind::Surface);
    let cap = fx.particles.capacity();
    assert!(cap >= particles::MAX_PARTICLES);
    for _ in 0..100 {
        fx.impact(Vec3::ZERO, Vec3::Y, ImpactKind::Surface);
        fx.impact(Vec3::ZERO, Vec3::X, ImpactKind::Body);
    }
    assert_eq!(fx.particles.len(), particles::MAX_PARTICLES);
    // full and recycling: never grew past the first reservation
    assert_eq!(fx.particles.capacity(), cap);
}

#[test]
fn particles_die_out() {
    let mut fx = HitFx::default();
    fx.impact(Vec3::ZERO, Vec3::Y, ImpactKind::Surface);
    fx.impact(Vec3::ZERO, Vec3::X, ImpactKind::Body);
    assert!(fx.particles.len() > 0);
    for _ in 0..60 {
        fx.tick(1.0 / 60.0);
    }
    assert_eq!(fx.particles.len(), 0);
}
