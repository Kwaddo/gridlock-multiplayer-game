/*
    whether a body needs its full pose this frame, from where the camera is (see simulation/body_view.rs)
    a body the camera can't see only needs the bones its hitboxes are built from, a far one gets its full pose
    at a lower rate, staggered by player id so they don't all land on one frame
*/

// a body further than this from the camera is posed in full at FAR_POSE_HZ, metres
pub const FAR_POSE_DISTANCE: f32 = 25.0;
// how often a far body's full pose is worked out
const FAR_POSE_HZ: f32 = 20.0;
const FAR_POSE_INTERVAL: f32 = 1.0 / FAR_POSE_HZ;
// how many different phases far bodies are spread over
const FAR_PHASES: u32 = 4;

#[derive(Clone, Copy, Debug)]
pub struct BodyView {
    seen: bool,       // may be on screen (or in a mirror), so it's drawn and needs its full pose
    far: bool,        // beyond FAR_POSE_DISTANCE
    since_full: f32,  // seconds since the last full pose
}

impl Default for BodyView {
    fn default() -> Self {
        // a new body counts as seen until the camera says otherwise, so it never starts on a stale pose
        Self { seen: true, far: false, since_full: 0.0 }
    }
}

impl BodyView {
    /// What the camera test said this frame. `id` staggers far bodies.
    pub fn set(&mut self, seen: bool, far: bool, id: u32) {
        if seen && !self.seen {
            // its pose is from before it was hidden: no good to draw, far or not
            self.pose_due();
        } else if far && !self.far {
            // just went far: start part way through the interval, by id, so far bodies take turns
            // (never later than already due: a pose that old is drawn now, not after the stagger)
            let phase = (id % FAR_PHASES) as f32 / FAR_PHASES as f32 * FAR_POSE_INTERVAL;
            self.since_full = self.since_full.max(phase);
        }
        self.seen = seen;
        self.far = far;
    }

    /// Whether the body may be on screen, so it is drawn.
    pub fn seen(&self) -> bool {
        self.seen
    }

    /// Whether this frame's pose has to be the full one: seen, and near or due.
    pub fn wants_full(&self) -> bool {
        self.seen && (!self.far || self.since_full >= FAR_POSE_INTERVAL)
    }

    pub(super) fn aged(&mut self, dt: f32) {
        self.since_full += dt;
    }

    /// The next pose seen has to be the full one (it jumped, or comes back into view).
    pub(super) fn pose_due(&mut self) {
        self.since_full = self.since_full.max(FAR_POSE_INTERVAL);
    }

    pub(super) fn posed_full(&mut self) {
        self.since_full = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_bodies_never_want_the_full_pose() {
        let mut v = BodyView::default();
        v.set(false, false, 0);
        assert!(!v.wants_full());
        v.set(true, false, 0);
        assert!(v.wants_full());
    }

    // a far body gets its full pose every FAR_POSE_INTERVAL, at most
    #[test]
    fn far_bodies_pose_at_the_lower_rate() {
        let mut v = BodyView::default();
        v.set(true, true, 0);
        let mut full = 0;
        for _ in 0..120 {
            v.aged(1.0 / 120.0);
            if v.wants_full() {
                v.posed_full();
                full += 1;
            }
        }
        // 6 or 7 frames apart at 120 fps depending on float rounding, so 17 to 20 a second
        assert!((17..=20).contains(&full), "{full} full poses in a second");
    }

    // a body coming into view far away, or jumping there, is posed in full at once, not after its stagger
    #[test]
    fn a_far_body_coming_into_view_or_jumping_is_posed_at_once() {
        let mut v = BodyView::default();
        v.set(true, false, 3);
        v.posed_full();
        v.set(false, false, 3);
        v.set(true, true, 3);
        assert!(v.wants_full());
        v.posed_full();
        v.set(true, false, 3);
        v.pose_due();
        v.set(true, true, 3);
        assert!(v.wants_full());
    }

    // four far bodies starting together don't all land on the same frame
    #[test]
    fn far_bodies_are_staggered_by_id() {
        let mut views = [BodyView::default(); 4];
        for (id, v) in views.iter_mut().enumerate() {
            v.set(true, true, id as u32);
        }
        let mut first = [0; 4];
        for frame in 1..=10 {
            for (v, first) in views.iter_mut().zip(&mut first) {
                v.aged(1.0 / 120.0);
                if v.wants_full() && *first == 0 {
                    *first = frame;
                    v.posed_full();
                }
            }
        }
        let mut sorted = first;
        sorted.sort();
        sorted.windows(2).for_each(|w| assert!(w[0] < w[1], "{first:?}"));
    }
}
