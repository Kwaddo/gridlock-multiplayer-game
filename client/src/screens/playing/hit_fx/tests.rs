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

fn settle(fx: &mut HitFx, secs: f32) {
    let steps = (secs * 60.0) as usize;
    for _ in 0..steps {
        fx.tick(1.0 / 60.0);
    }
}

#[test]
fn a_hit_punches_then_springs_back_to_nothing() {
    let mut fx = HitFx::default();
    fx.punch.hit(40.0, 1.0, 1.0);
    fx.tick(1.0 / 30.0);
    let (pitch, yaw, roll, _) = fx.view_offsets();
    assert!(pitch > 0.0, "tips up");
    // shot from the right: the view turns and rolls away, to the left
    assert!(yaw < 0.0 && roll < 0.0);
    settle(&mut fx, 0.5);
    assert_eq!(fx.view_offsets(), (0.0, 0.0, 0.0, 0.0));
}

#[test]
fn the_punch_peaks_near_its_target_and_scales_with_damage() {
    let peak = |damage: f32, scale: f32| {
        let mut fx = HitFx::default();
        fx.punch.hit(damage, 0.0, scale);
        let mut best = 0.0f32;
        for _ in 0..240 {
            fx.tick(1.0 / 240.0);
            best = best.max(fx.view_offsets().0);
        }
        best
    };
    let full = peak(40.0, 1.0);
    assert!((full - punch::MAX_PITCH * 0.6).abs() < punch::MAX_PITCH * 0.05, "{full}");
    assert!(peak(10.0, 1.0) < full * 0.5);
    // aimed down sights it's a quarter
    assert!((peak(40.0, 0.25) - full * 0.25).abs() < full * 0.05);
}

#[test]
fn stacked_hits_stay_capped_and_a_long_frame_is_stable() {
    let mut fx = HitFx::default();
    for _ in 0..50 {
        fx.punch.hit(200.0, 1.0, 1.0);
        fx.tick(1.0 / 120.0);
        let (p, y, r, f) = fx.view_offsets();
        assert!(p.abs() <= punch::MAX_PITCH && y.abs() <= punch::MAX_YAW);
        assert!(r.abs() <= punch::MAX_ROLL && f <= punch::MAX_FOV);
    }
    // a half-second hitch: no blow-up, just settled
    fx.tick(0.5);
    let (p, ..) = fx.view_offsets();
    assert!(p.abs() < 1e-3);
}

#[test]
fn a_kill_slows_view_time_briefly_then_lets_go() {
    let mut fx = HitFx::default();
    assert_eq!(fx.tick(0.01), 0.01);
    fx.on_kill(false);
    let slowed = fx.tick(0.01);
    assert!((slowed - 0.01 * HITSTOP_RATE).abs() < 1e-6);
    // the hitstop counts down in real time, so it ends on schedule
    fx.tick(HITSTOP_SECS);
    assert_eq!(fx.tick(0.01), 0.01);
    // the marker kept going in real time through it
    assert!(fx.kill_marker_on());
}

#[test]
fn a_kill_kicks_the_view_and_pinches_the_fov() {
    let mut fx = HitFx::default();
    fx.on_kill(true);
    settle(&mut fx, 0.1);
    let (pitch, _, _, pinch) = fx.view_offsets();
    assert!(pitch > 0.0 && pinch > 0.0);
    settle(&mut fx, 0.6);
    assert_eq!(fx.view_offsets(), (0.0, 0.0, 0.0, 0.0));
}
