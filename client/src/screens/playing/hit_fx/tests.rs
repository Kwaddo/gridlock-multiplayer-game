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
