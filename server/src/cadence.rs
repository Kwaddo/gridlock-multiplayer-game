//! An even send rate from a caller ticked at its own, uneven rate.
//!
//! The browser host ticks once per render frame (60 Hz, 144 Hz, ...), the
//! native host after every packet. Gating sends on "33 ms since the last one"
//! against 16.7 ms frames lands each send on the 2nd or 3rd frame depending on
//! a fraction of a millisecond, so the rate wanders between 20 and 30 Hz.
//! A [`Cadence`] keeps a deadline instead and steps it by whole intervals, so
//! the long-run rate is the asked one and the spacing stays even.

use std::time::Duration;

use platform::Instant;

/// Exactly 30 a second: a whole number of 60 Hz frames, so it doesn't drift
/// against them.
pub const HZ30: Duration = Duration::from_nanos(33_333_333);

/// A deadline-stepping rate gate.
#[derive(Clone, Copy, Debug)]
pub struct Cadence {
    interval: Duration,
    next: Option<Instant>,
}

impl Cadence {
    pub const fn new(interval: Duration) -> Self {
        Cadence { interval, next: None }
    }

    /// Whether a send is due at `now`; if so, the next deadline moves on.
    ///
    /// - A send may go out up to an eighth of an interval early, so a frame
    ///   that lands a hair before the deadline doesn't wait a whole frame
    ///   (which would turn frame jitter into 2-or-3-frame spacing).
    /// - The next deadline is one interval after the last one, so a caller
    ///   that falls slightly behind catches up, but by at most a quarter of
    ///   an interval per send: a stall or a slow caller (59 Hz) never gives
    ///   two sends on back-to-back frames.
    /// - The first call is always due.
    pub fn due(&mut self, now: Instant) -> bool {
        let early = self.interval / 8;
        let max_catch_up = self.interval / 4;
        match self.next {
            Some(next) if now + early < next => false,
            Some(next) => {
                let earliest = now + (self.interval - max_catch_up);
                self.next = Some((next + self.interval).max(earliest));
                true
            }
            None => {
                self.next = Some(now + self.interval);
                true
            }
        }
    }

    /// A send went out off-cadence at `now`: the next one is a full interval
    /// later, as if it had been due.
    pub fn restart(&mut self, now: Instant) {
        self.next = Some(now + self.interval);
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs `secs` of frames whose lengths come from `frame_len`, and returns
    /// the gaps between sends, in frames, and the sends per second.
    fn run(secs: f64, mut frame_len: impl FnMut(u64) -> Duration) -> (Vec<u64>, f64) {
        let mut c = Cadence::new(HZ30);
        let start = Instant::now();
        let mut now = start;
        let mut frame = 0u64;
        let mut last = None;
        let mut gaps = Vec::new();
        let mut sends = 0u64;
        while now.duration_since(start).as_secs_f64() < secs {
            if c.due(now) {
                sends += 1;
                if let Some(l) = last {
                    gaps.push(frame - l);
                }
                last = Some(frame);
            }
            now += frame_len(frame);
            frame += 1;
        }
        (gaps, sends as f64 / now.duration_since(start).as_secs_f64())
    }

    fn hz(rate: f64) -> Duration {
        Duration::from_secs_f64(1.0 / rate)
    }

    fn count(gaps: &[u64], n: u64) -> usize {
        gaps.iter().filter(|&&g| g == n).count()
    }

    #[test]
    fn steady_60hz_sends_every_other_frame() {
        let (gaps, rate) = run(30.0, |_| hz(60.0));
        assert!(gaps.iter().all(|&g| g == 2), "gaps: {:?}", &gaps[..20]);
        assert!((rate - 30.0).abs() < 0.1, "rate {rate}");
    }

    #[test]
    fn jittery_59_to_61hz_stays_even() {
        let mut rng = 0x9e37_79b9_7f4a_7c15u64;
        let (gaps, rate) = run(30.0, |_| {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let t = (rng % 1000) as f64 / 999.0;
            hz(59.0 + 2.0 * t)
        });
        assert_eq!(count(&gaps, 1), 0, "no back-to-back sends");
        let threes = count(&gaps, 3);
        assert!(threes * 50 < gaps.len(), "{threes} 3-frame gaps of {}", gaps.len());
        assert!((rate - 30.0).abs() < 0.3, "rate {rate}");
    }

    #[test]
    fn slow_59hz_never_doubles_up() {
        let (gaps, rate) = run(30.0, |_| hz(59.0));
        assert!(gaps.iter().all(|&g| g == 2));
        assert!((rate - 29.5).abs() < 0.1, "rate {rate}");
    }

    #[test]
    fn fast_61hz_keeps_30_a_second() {
        let (gaps, rate) = run(30.0, |_| hz(61.0));
        assert_eq!(count(&gaps, 1), 0);
        assert!(gaps.iter().all(|&g| g == 2 || g == 3));
        assert!((rate - 30.0).abs() < 0.2, "rate {rate}");
    }

    #[test]
    fn high_refresh_144hz_keeps_30_a_second() {
        // the old gate sent every 5th frame: 28.8 a second
        let (gaps, rate) = run(30.0, |_| hz(144.0));
        assert!(gaps.iter().all(|&g| g == 4 || g == 5), "gaps {:?}", &gaps[..20]);
        assert!((rate - 30.0).abs() < 0.1, "rate {rate}");
    }

    #[test]
    fn a_stall_does_not_burst() {
        // 60 Hz with one 500 ms hitch: one send right after it, then every
        // other frame again, never two in a row
        let mut c = Cadence::new(HZ30);
        let mut now = Instant::now();
        let mut sends = Vec::new();
        for frame in 0..200u64 {
            if c.due(now) {
                sends.push(frame);
            }
            now += if frame == 100 { Duration::from_millis(500) } else { hz(60.0) };
        }
        assert!(sends.contains(&101), "sends right after the hitch: {sends:?}");
        // (100 to 101 spans the hitch)
        let after: Vec<u64> = sends.iter().copied().filter(|&f| f >= 101).collect();
        assert!(after.windows(2).all(|w| w[1] - w[0] == 2), "sends: {sends:?}");
    }

    #[test]
    fn restart_pushes_the_next_send_a_full_interval_out() {
        let mut c = Cadence::new(HZ30);
        let t0 = Instant::now();
        assert!(c.due(t0));
        c.restart(t0 + hz(60.0));
        assert!(!c.due(t0 + hz(60.0) * 2));
        assert!(c.due(t0 + hz(60.0) * 3));
    }
}
