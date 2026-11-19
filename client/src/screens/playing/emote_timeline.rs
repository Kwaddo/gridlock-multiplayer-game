//! An emote's life after it is picked: the camera blends out to third person,
//! the emote runs up to `MAX_SECS`, then the camera blends back in. The emote
//! id stays set (so others keep seeing the dance) until the view is first
//! person again.

use crate::net::protocols::EMOTE_NONE;

/// Longest an emote plays before it ends by itself.
const MAX_SECS: f32 = 5.0;
/// Seconds the camera takes to move between first and third person.
const TRANSITION_SECS: f32 = 0.35;

#[derive(Default)]
pub struct EmoteTimeline {
    /// Emote id this timeline is tracking; a change restarts it.
    id: u8,
    elapsed: f32,
    /// Camera blend, 0 = first person, 1 = third person.
    cam_t: f32,
    /// Stopped, blending back to first person.
    ending: bool,
}

impl EmoteTimeline {
    /// Advance by `dt` for the emote currently `playing`. Returns true once the
    /// camera is back in first person after an end, so the caller clears the id.
    pub fn tick(&mut self, dt: f32, playing: u8) -> bool {
        if playing != self.id {
            self.id = playing;
            self.elapsed = 0.0;
            self.ending = false;
        }
        if playing == EMOTE_NONE {
            self.cam_t = 0.0;
            return false;
        }
        let step = dt / TRANSITION_SECS;
        if self.ending {
            self.cam_t = (self.cam_t - step).max(0.0);
            return self.cam_t == 0.0;
        }
        self.cam_t = (self.cam_t + step).min(1.0);
        self.elapsed += dt;
        if self.elapsed >= MAX_SECS {
            self.ending = true;
        }
        false
    }

    /// End the emote early; the view blends back before the id is cleared.
    pub fn stop(&mut self) {
        self.ending = true;
    }

    pub fn is_ending(&self) -> bool {
        self.ending
    }

    /// Eased camera blend, 0 = first person, 1 = third person.
    pub fn view_t(&self) -> f32 {
        let t = self.cam_t;
        t * t * (3.0 - 2.0 * t) // smoothstep: eases out of and into both ends
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: u8 = 1;

    /// Tick in small steps for `secs`, returning whether the outro finished.
    fn run(t: &mut EmoteTimeline, secs: f32, playing: u8) -> bool {
        let mut done = false;
        for _ in 0..(secs / 0.01).round() as usize {
            done |= t.tick(0.01, playing);
        }
        done
    }

    #[test]
    fn camera_blends_out_then_ends_after_max_secs() {
        let mut t = EmoteTimeline::default();
        t.tick(TRANSITION_SECS / 2.0, ID);
        assert!(t.view_t() > 0.0 && t.view_t() < 1.0); // in between, no snap
        assert!(!run(&mut t, MAX_SECS - 0.3, ID));
        assert_eq!(t.view_t(), 1.0);
        assert!(run(&mut t, 0.5 + TRANSITION_SECS, ID)); // hit the cap, blended back
        assert_eq!(t.view_t(), 0.0);
    }

    #[test]
    fn early_stop_keeps_emote_until_first_person() {
        let mut t = EmoteTimeline::default();
        run(&mut t, 1.0, ID);
        t.stop();
        assert!(!t.tick(0.01, ID)); // still blending back: id must stay set
        assert!(run(&mut t, TRANSITION_SECS, ID));
        t.tick(0.01, EMOTE_NONE);
        assert!(!t.is_ending()); // cleared, ready for the next emote
    }

    #[test]
    fn switching_emote_restarts_the_clock() {
        let mut t = EmoteTimeline::default();
        run(&mut t, MAX_SECS - 0.5, ID);
        run(&mut t, 1.0, ID + 1); // a new pick gets its own full run
        assert!(!t.is_ending());
    }
}
