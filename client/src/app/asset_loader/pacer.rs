//! How background loads are spaced out while playing.

/// Share of the time while playing that background loads may take.
const LOAD_SHARE: f32 = 0.25;
/// A frame this long (ms) is already over budget: no load starts after one.
const SLOW_FRAME_MS: f32 = 20.0;
/// However slow the frames, a load starts at least this often (ms), so the
/// queue never stalls.
const MAX_GAP_MS: f32 = 500.0;

/// When the next background load may start while playing. A load of `x` ms
/// leaves a debt of `x` that every frame pays back by [`LOAD_SHARE`] of its
/// length, so loads take about that share of the time, spread out rather
/// than back to back. None starts after a frame already over
/// [`SLOW_FRAME_MS`] (which includes the frame a big load just made slow),
/// and one always may once [`MAX_GAP_MS`] passed since the last. A model
/// loaded a step at a time (`GlbJob`) is paced a step at a time: each step
/// counts as a load here.
#[derive(Default)]
pub(super) struct LoadPacer {
    debt_ms: f32,
    since_load_ms: f32,
    load_in_next_frame_ms: f32,
}

impl LoadPacer {
    /// Called once per frame with the previous frame's length: may a load
    /// start in this one?
    pub fn ready(&mut self, prev_frame_ms: f32) -> bool {
        let frame = prev_frame_ms.max(0.0);
        self.since_load_ms += (frame - std::mem::take(&mut self.load_in_next_frame_ms)).max(0.0);
        self.debt_ms = (self.debt_ms - frame * LOAD_SHARE).max(0.0);
        self.since_load_ms >= MAX_GAP_MS || (self.debt_ms <= 0.0 && frame <= SLOW_FRAME_MS)
    }

    /// A load just took `took_ms`.
    pub fn loaded(&mut self, took_ms: f32) {
        self.debt_ms = took_ms.max(0.0);
        // The gap counts from the load's end. The next frame's length
        // includes a load done in one go, so that much of it is left out (or
        // a long one would start the next right after); a load that awaited
        // a download over several frames is over by then, and leaves out
        // no more than that one frame.
        self.since_load_ms = 0.0;
        self.load_in_next_frame_ms = self.debt_ms;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Frames of `frame_ms` (plus the load's own time on the frame after a
    /// load, as a real frame would measure it) where every load takes
    /// `load_ms`: the frames on which a load started.
    fn paced(frame_ms: f32, load_ms: f32, frames: usize) -> Vec<usize> {
        let mut pacer = LoadPacer::default();
        let mut prev = frame_ms;
        let mut starts = Vec::new();
        for f in 0..frames {
            if pacer.ready(prev) {
                pacer.loaded(load_ms);
                starts.push(f);
                prev = frame_ms + load_ms;
            } else {
                prev = frame_ms;
            }
        }
        starts
    }

    #[test]
    fn small_loads_go_every_frame() {
        // a 1 ms sound: nothing to pay back by the next frame
        assert_eq!(paced(16.7, 1.0, 10), (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn big_loads_are_spread_to_their_share() {
        for &(frame, load) in &[(16.7, 40.0), (16.7, 120.0), (8.3, 60.0)] {
            let starts = paced(frame, load, 2000);
            assert!(starts.windows(2).all(|w| w[1] - w[0] > 1), "back to back: {frame} {load}");
            let wall = 2000.0 * frame + starts.len() as f32 * load;
            let share = starts.len() as f32 * load / wall;
            assert!(share <= LOAD_SHARE + 0.02 && share >= LOAD_SHARE - 0.08, "share {share}: {frame} {load}");
        }
    }

    #[test]
    fn slow_frames_wait_but_never_forever() {
        // already over budget every frame (30 fps): only the guarantee loads
        let starts = paced(33.3, 5.0, 300);
        assert!(!starts.is_empty());
        let max_gap_frames = (MAX_GAP_MS / 33.3).ceil() as usize;
        assert!(starts.windows(2).all(|w| w[1] - w[0] <= max_gap_frames), "{starts:?}");
        assert!(starts[0] <= max_gap_frames);
        // a load so long its debt would outlast the guarantee
        let starts = paced(16.7, 5000.0, 400);
        assert!(starts.len() >= 2);
        assert!(starts.windows(2).all(|w| w[1] - w[0] <= (MAX_GAP_MS / 16.7).ceil() as usize + 1));
    }

    // a load that waited on a download over several short frames: the next
    // still comes within the guarantee of its end, not that plus the wait
    #[test]
    fn a_long_wait_on_the_network_does_not_push_the_next_load_back() {
        let mut pacer = LoadPacer::default();
        assert!(pacer.ready(16.7));
        pacer.loaded(3000.0);
        let frames = (1..100).position(|_| pacer.ready(16.7)).unwrap() + 1;
        assert!(frames as f32 * 16.7 <= MAX_GAP_MS + 2.0 * 16.7, "{frames} frames");
    }

    #[test]
    fn one_slow_frame_skips_one_load() {
        let mut pacer = LoadPacer::default();
        assert!(pacer.ready(16.7));
        pacer.loaded(0.5);
        assert!(!pacer.ready(25.0));
        assert!(pacer.ready(16.7));
    }
}
