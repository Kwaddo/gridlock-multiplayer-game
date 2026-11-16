//! Decides when the S&D round state is worth broadcasting. Between ticks
//! usually only the phase timer moves, and clients show it in whole seconds,
//! so the state goes out when something a client shows changes, plus a slow
//! heartbeat that covers lost packets.

/// Longest gap between two broadcasts, even when nothing visible changed.
pub(super) const HEARTBEAT_SECS: f32 = 0.25;
/// Plant and defuse progress (0..1) is compared in steps of 1/PROGRESS_STEPS,
/// about 20 sends a second while a 4 s plant runs.
const PROGRESS_STEPS: f32 = 80.0;
/// Matches the client's red-timer threshold (snd_hud.rs: `snd_phase_timer < 10.0`).
const URGENT_SECS: f32 = 10.0;

/// Everything a client shows from the round state, quantised as it shows it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct RoundStateKey {
    pub round: u8,
    pub wins: (u8, u8),
    pub phase: u8,
    pub attacking_team: u8,
    pub bomb_carrier: u32,
    pub bomb_pos: (f32, f32, f32),
    pub bomb_dropped: bool,
    pub bomb_planted: bool,
    pub plant_step: i32,
    pub defuse_step: i32,
    /// The HUD shows a plant/defuse bar as soon as progress is above 0, which
    /// the first step (below 1/80) does not tell apart from no progress.
    pub planting: bool,
    pub defusing: bool,
    /// `phase_timer` rounded up to whole seconds, as the HUD prints it.
    pub timer_secs: i32,
    /// The HUD turns the timer red below `URGENT_SECS`; ceil() alone does
    /// not change at that moment (9.99 still shows 10).
    pub urgent: bool,
    /// A player joining gets the state at once.
    pub players: usize,
}

impl RoundStateKey {
    pub fn progress_step(progress: f32) -> i32 {
        (progress * PROGRESS_STEPS).floor() as i32
    }

    pub fn timer_secs(phase_timer: f32) -> i32 {
        phase_timer.max(0.0).ceil() as i32
    }

    pub fn urgent(phase_timer: f32) -> bool {
        phase_timer < URGENT_SECS
    }
}

#[derive(Default)]
pub(super) struct RoundStateGate {
    last: Option<RoundStateKey>,
    since: f32,
}

impl RoundStateGate {
    /// Advance by `dt` and say whether to broadcast `key` now.
    pub fn should_send(&mut self, key: RoundStateKey, dt: f32) -> bool {
        self.since += dt;
        if self.last == Some(key) && self.since < HEARTBEAT_SECS {
            return false;
        }
        self.last = Some(key);
        self.since = 0.0;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(timer: f32, plant: f32) -> RoundStateKey {
        RoundStateKey {
            round: 1,
            wins: (0, 0),
            phase: 1,
            attacking_team: 1,
            bomb_carrier: 7,
            bomb_pos: (0.0, 0.0, 0.0),
            bomb_dropped: false,
            bomb_planted: false,
            plant_step: RoundStateKey::progress_step(plant),
            defuse_step: 0,
            planting: plant > 0.0,
            defusing: false,
            timer_secs: RoundStateKey::timer_secs(timer),
            urgent: RoundStateKey::urgent(timer),
            players: 4,
        }
    }

    #[test]
    fn first_state_is_sent() {
        let mut gate = RoundStateGate::default();
        assert!(gate.should_send(key(90.0, 0.0), 1.0 / 60.0));
    }

    #[test]
    fn unchanged_state_waits_for_the_heartbeat() {
        let dt = 1.0 / 60.0;
        let mut gate = RoundStateGate::default();
        let mut timer = 89.9;
        assert!(gate.should_send(key(timer, 0.0), dt));
        let mut sends = 0;
        // one second of ticks while the timer stays within the same whole second
        for _ in 0..50 {
            timer -= dt;
            if gate.should_send(key(timer.max(89.01), 0.0), dt) {
                sends += 1;
            }
        }
        // 50 ticks = 0.83 s: three heartbeats, not 50 sends
        assert_eq!(sends, 3);
    }

    #[test]
    fn visible_changes_are_sent_at_once() {
        let dt = 1.0 / 60.0;
        let mut gate = RoundStateGate::default();
        assert!(gate.should_send(key(10.5, 0.0), dt));
        // the shown second ticks over
        assert!(gate.should_send(key(9.99, 0.0), dt));
        assert!(!gate.should_send(key(9.98, 0.0), dt));
        // the timer turns red while ceil() still shows 10
        assert!(gate.should_send(key(10.0, 0.0), dt));
        assert_eq!(key(10.0, 0.0).timer_secs, key(9.999, 0.0).timer_secs);
        assert!(gate.should_send(key(9.999, 0.0), dt));
        // a plant starting shows its bar at once, even below the first step
        assert!(gate.should_send(key(9.975, 0.001), dt));
        // planting moves the bar
        assert!(gate.should_send(key(9.97, 0.1), dt));
        let mut changed = key(9.96, 0.1);
        changed.players = 5;
        assert!(gate.should_send(changed, dt));
    }

    #[test]
    fn bomb_drop_is_sent_at_once() {
        let dt = 1.0 / 60.0;
        let mut gate = RoundStateGate::default();
        assert!(gate.should_send(key(50.5, 0.0), dt));
        // dropped at the origin: only the dropped flag tells it apart
        let mut dropped = key(50.5, 0.0);
        dropped.bomb_carrier = 0;
        assert!(gate.should_send(dropped, dt));
        dropped.bomb_dropped = true;
        assert!(gate.should_send(dropped, dt));
    }
}
