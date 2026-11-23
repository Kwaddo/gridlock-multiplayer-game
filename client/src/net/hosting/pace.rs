//! When a server hosted in the browser tab ticks, and how much it keeps for
//! the hosting player while that player's frames stall. Pure logic, built on
//! every target so it is tested natively (the host itself is wasm-only).

use std::ops::Deref;
use std::time::Duration;

use platform::Instant;

/// The server's tick, as a dedicated one runs it (server/src/udp_host.rs).
pub(crate) const TICK_INTERVAL: Duration = Duration::from_micros(16_667);
/// Packets for the hosting player held while its frames are stalled; past
/// this, the oldest small ones go (as a full socket buffer would drop them).
pub(crate) const TO_PLAYER_MAX: usize = 4096;
/// Bigger packets than this go over WebRTC's reliable channel, so the game
/// counts on them arriving (RELIABLE_MIN_BYTES in web/gridlock_web/constants.js):
/// never dropped here either.
pub(crate) const SMALL_PACKET: usize = 1100;

/// A 60 Hz tick schedule stepped by calls that come at any rate.
pub(crate) struct TickClock {
    next: Instant,
}

impl TickClock {
    pub(crate) fn new(now: Instant) -> Self {
        TickClock { next: now }
    }

    /// Whether a tick is due at `now`; if so, it counts as done. Ticks keep
    /// to a steady grid whatever the calls' spacing; more than a tick behind
    /// (a long frame, a hidden tab), the grid restarts from `now` instead of
    /// ticking to catch up: the server's timers run on real time either way.
    pub(crate) fn due(&mut self, now: Instant) -> bool {
        if now < self.next {
            return false;
        }
        self.next += TICK_INTERVAL;
        if self.next <= now {
            self.next = now + TICK_INTERVAL;
        }
        true
    }
}

/// While the hosting player's frames stall the server keeps sending: past
/// [`TO_PLAYER_MAX`] packets, drop the oldest small ones down to 3/4 of it
/// (so this runs rarely). Big ones stay, as their channel would deliver them.
pub(crate) fn trim_backlog<P: Deref<Target = [u8]>>(queue: &mut Vec<P>) {
    if queue.len() <= TO_PLAYER_MAX {
        return;
    }
    let mut excess = queue.len() - TO_PLAYER_MAX * 3 / 4;
    queue.retain(|p| {
        let drop = excess > 0 && p.len() <= SMALL_PACKET;
        excess -= drop as usize;
        !drop
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ticks in one second of calls spaced `step` apart.
    fn ticks_in_a_second(step: Duration) -> usize {
        let start = Instant::now();
        let mut clock = TickClock::new(start);
        let mut at = start;
        let mut ticks = 0;
        while at < start + Duration::from_secs(1) {
            ticks += clock.due(at) as usize;
            at += step;
        }
        ticks
    }

    #[test]
    fn ticks_at_60_hz_from_fast_calls() {
        assert_eq!(ticks_in_a_second(Duration::from_millis(4)), 60);
        assert_eq!(ticks_in_a_second(Duration::from_millis(1)), 60);
    }

    #[test]
    fn slow_calls_tick_once_each() {
        assert_eq!(ticks_in_a_second(Duration::from_millis(50)), 20);
    }

    #[test]
    fn a_stall_costs_one_tick_then_resumes_without_bursting() {
        let start = Instant::now();
        let mut clock = TickClock::new(start);
        assert!(clock.due(start));
        let after = start + Duration::from_millis(500);
        assert!(clock.due(after));
        // Calls queued behind the stall, all at once: no catching up.
        assert!(!clock.due(after));
        assert!(!clock.due(after + Duration::from_millis(16)));
        assert!(clock.due(after + TICK_INTERVAL));
    }

    #[test]
    fn trimming_drops_the_oldest_small_packets_only() {
        let big = vec![0u8; SMALL_PACKET + 1].into_boxed_slice();
        let mut queue: Vec<Box<[u8]>> = vec![big.clone()];
        queue.extend((0..TO_PLAYER_MAX).map(|i| vec![(i % 251) as u8; 8].into_boxed_slice()));
        assert_eq!(queue.len(), TO_PLAYER_MAX + 1);
        trim_backlog(&mut queue);
        assert_eq!(queue.len(), TO_PLAYER_MAX * 3 / 4);
        assert_eq!(queue[0], big);
        // The newest small packet survived, the oldest did not.
        assert_eq!(queue.last().unwrap()[0], ((TO_PLAYER_MAX - 1) % 251) as u8);
        assert_eq!(queue[1][0], ((TO_PLAYER_MAX / 4 + 1) % 251) as u8);

        let mut short: Vec<Box<[u8]>> = (0..10).map(|_| vec![0u8; 8].into_boxed_slice()).collect();
        trim_backlog(&mut short);
        assert_eq!(short.len(), 10);
    }
}
