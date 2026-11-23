//! Dev-only latency check for hit feedback (#63's open question): from the
//! shot that hit a player, how long until the hit report went out, and how
//! long until the server's kill feed came back with our kill. The hit
//! marker itself is drawn on the shot's own frame (0 ms), so anything slow
//! shows up here as the network round trip.
//!
//! The victim side (shot -> their MSG_DAMAGE) can't be timed from one
//! client without synced clocks; it is roughly half this round trip.

/// Times in milliseconds, from the last shot that hit a player.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct HitTiming {
    shot_at: Option<f64>,
    /// Shot -> its hit report handed to the socket.
    pub sent_ms: Option<f32>,
    /// Shot -> the kill feed naming us as the killer.
    pub kill_ms: Option<f32>,
}

fn ms(from: f64, to: f64) -> f32 {
    ((to - from) * 1000.0).max(0.0) as f32
}

impl HitTiming {
    /// A shot hit a player at `now` (seconds): start a new measurement.
    pub fn shot(&mut self, now: f64) {
        *self = HitTiming { shot_at: Some(now), ..Default::default() };
    }

    /// A hit report went out; only the first after the shot counts.
    pub fn sent(&mut self, now: f64) {
        if let (Some(at), None) = (self.shot_at, self.sent_ms) {
            self.sent_ms = Some(ms(at, now));
        }
    }

    /// Our kill came back. Returns the shot -> kill time, if a shot was timed
    /// and this is the first kill since it.
    pub fn kill(&mut self, now: f64) -> Option<f32> {
        let at = self.shot_at?;
        if self.kill_ms.is_some() {
            return None;
        }
        let t = ms(at, now);
        self.kill_ms = Some(t);
        Some(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_a_shot_to_its_report_and_kill() {
        let mut t = HitTiming::default();
        // nothing to time before a shot
        assert_eq!(t.kill(1.0), None);
        t.shot(10.0);
        t.sent(10.016);
        t.sent(10.5); // a later report doesn't move it
        assert!((t.sent_ms.unwrap() - 16.0).abs() < 0.01);
        assert!((t.kill(10.080).unwrap() - 80.0).abs() < 0.01);
        // one kill per shot measured
        assert_eq!(t.kill(10.2), None);
        // the next shot starts afresh
        t.shot(11.0);
        assert_eq!(t.sent_ms, None);
        assert_eq!(t.kill_ms, None);
    }
}
