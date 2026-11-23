//! The camera punch: a small, quick tilt of the view when you get hit (and a
//! kick on a kill). It is an offset on the camera only. The aim (`yaw`,
//! `pitch`) and every shot are untouched, and it springs back to nothing in
//! a couple of tenths of a second.

/// How stiff the spring back is (rad/s). Critically damped, so it settles
/// in about 5/OMEGA seconds without overshooting.
const OMEGA: f32 = 28.0;
const E: f32 = std::f32::consts::E;

/// Most the view ever tilts, however many hits stack up (radians).
pub(super) const MAX_PITCH: f32 = 2.0 * std::f32::consts::PI / 180.0;
pub(super) const MAX_YAW: f32 = 1.2 * std::f32::consts::PI / 180.0;
pub(super) const MAX_ROLL: f32 = 4.0 * std::f32::consts::PI / 180.0;
/// Most the field of view narrows (a fraction of it).
pub(super) const MAX_FOV: f32 = 0.05;

/// One offset springing back to 0, solved exactly so a long frame can't
/// make it blow up.
#[derive(Clone, Copy, Default, Debug)]
pub(super) struct Spring {
    x: f32,
    v: f32,
}

impl Spring {
    pub(super) fn step(&mut self, dt: f32) {
        if self.x == 0.0 && self.v == 0.0 {
            return;
        }
        let e = (-OMEGA * dt).exp();
        let b = self.v + OMEGA * self.x;
        self.x = (self.x + b * dt) * e;
        self.v = (self.v - OMEGA * b * dt) * e;
        if self.x.abs() < 1e-6 && self.v.abs() < 1e-4 {
            *self = Spring::default();
        }
    }

    /// A kick that, from rest, peaks at `peak` after 1/OMEGA seconds. The
    /// speed is capped so stacked kicks stay near `cap`.
    pub(super) fn kick(&mut self, peak: f32, cap: f32) {
        let v_max = cap * OMEGA * E;
        self.v = (self.v + peak * OMEGA * E).clamp(-v_max, v_max);
    }

    pub(super) fn value(&self, cap: f32) -> f32 {
        self.x.clamp(-cap, cap)
    }
}

/// The view's offsets: pitch, yaw, roll and a field-of-view pinch.
#[derive(Default)]
pub(super) struct Punch {
    pub(super) pitch: Spring,
    pub(super) yaw: Spring,
    pub(super) roll: Spring,
    pub(super) fov: Spring,
}

impl Punch {
    /// Took `damage`. `side` is where it came from across the view: 1 from
    /// the right, -1 from the left, 0 straight ahead/behind or unknown.
    /// `scale` (0..1) softens it, e.g. while aiming down sights.
    pub(super) fn hit(&mut self, damage: f32, side: f32, scale: f32) {
        let s = (damage / 40.0).clamp(0.2, 1.0) * scale.clamp(0.0, 1.0);
        let side = side.clamp(-1.0, 1.0);
        // knocked back: the view tips up and away from the shot
        self.pitch.kick(MAX_PITCH * 0.6 * s, MAX_PITCH);
        self.roll.kick(-side * MAX_ROLL * 0.6 * s, MAX_ROLL);
        self.yaw.kick(-side * MAX_YAW * 0.5 * s, MAX_YAW);
    }

    /// A kill: a small upward kick and a quick zoom pinch.
    pub(super) fn kill(&mut self) {
        self.pitch.kick(MAX_PITCH * 0.2, MAX_PITCH);
        self.fov.kick(MAX_FOV * 0.6, MAX_FOV);
    }

    pub(super) fn step(&mut self, dt: f32) {
        for s in [&mut self.pitch, &mut self.yaw, &mut self.roll, &mut self.fov] {
            s.step(dt);
        }
    }

    /// (pitch, yaw, roll, fov pinch) right now.
    pub(super) fn offsets(&self) -> (f32, f32, f32, f32) {
        (
            self.pitch.value(MAX_PITCH),
            self.yaw.value(MAX_YAW),
            self.roll.value(MAX_ROLL),
            self.fov.value(MAX_FOV).max(0.0),
        )
    }
}
