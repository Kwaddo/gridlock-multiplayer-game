//! Hit feedback that is only for the eyes (#63): impact puffs, the kill and
//! headshot markers, the camera punch when you're hit and the kick and
//! hitstop on a kill. Nothing here touches the aim, the simulation or the
//! network: the punch is an offset on the camera only, shots still go where
//! `yaw`/`pitch` point.

mod hud;
mod particles;
mod punch;
#[cfg(test)]
mod tests;

pub(crate) use particles::ImpactKind;
use macroquad::prelude::Vec3;

/// How long the kill marker stays up.
pub(super) const KILL_MARKER_SECS: f32 = 0.55;
/// A kill's hitstop: this long (real time) with view effects at `HITSTOP_RATE` speed.
pub(super) const HITSTOP_SECS: f32 = 0.06;
pub(super) const HITSTOP_RATE: f32 = 0.1;

#[derive(Default)]
pub struct HitFx {
    particles: particles::Particles,
    punch: punch::Punch,
    /// Seconds left on the kill marker.
    kill_marker: f32,
    /// Whether that kill was a headshot.
    kill_headshot: bool,
    /// Real seconds left in the kill's hitstop.
    hitstop: f32,
}

impl HitFx {
    /// We killed someone: the kill marker replaces the hit marker for a moment.
    pub(crate) fn on_kill(&mut self, headshot: bool) {
        self.kill_marker = KILL_MARKER_SECS;
        self.kill_headshot = headshot;
        self.hitstop = HITSTOP_SECS;
        self.punch.kill();
    }

    /// Whether the kill marker is up (the plain hit marker gives way to it).
    pub(super) fn kill_marker_on(&self) -> bool {
        self.kill_marker > 0.0
    }

    /// A puff where a shot landed; `normal` points back out of what was hit.
    pub(crate) fn impact(&mut self, at: Vec3, normal: Vec3, kind: ImpactKind) {
        self.particles.spawn(at, normal, kind);
    }

    /// The impact puffs, in the 3D pass, facing `eye`.
    pub(super) fn draw_world(&self, eye: Vec3) {
        self.particles.draw(eye);
    }

    /// The camera's offsets right now: (pitch, yaw, roll, fov pinch), the
    /// pinch a fraction to narrow the field of view by.
    pub(super) fn view_offsets(&self) -> (f32, f32, f32, f32) {
        self.punch.offsets()
    }

    /// Advances the effects by a frame of `dt` real seconds. Returns the
    /// frame's view time: `dt`, but slowed to a crawl during a kill's
    /// hitstop, for the other view-only motion (head bob) to use. The game
    /// itself (movement, shots, network) never sees it.
    pub(super) fn tick(&mut self, dt: f32) -> f32 {
        let view_dt = if self.hitstop > 0.0 {
            self.hitstop = (self.hitstop - dt).max(0.0);
            dt * HITSTOP_RATE
        } else {
            dt
        };
        self.particles.update(view_dt);
        self.punch.step(view_dt);
        self.kill_marker = (self.kill_marker - dt).max(0.0);
        view_dt
    }
}

impl super::PlayingState {
    /// We got a kill (the server's kill feed, a practice bot dying here, or a
    /// Smoke Break find): show the kill marker.
    pub fn note_local_kill(&mut self, headshot: bool) {
        self.hit_fx.on_kill(headshot);
    }

    /// How much of the punch shows: a quarter of it fully aimed down sights,
    /// where a steady view matters most.
    fn punch_scale(&self) -> f32 {
        1.0 - 0.75 * self.weapons.active().ads_progress
    }

    /// We took `damage` from someone standing at `source` (if we know where):
    /// a short camera punch, bigger for bigger hits, tipping away from them.
    pub(super) fn punch_from_hit(&mut self, damage: f32, source: Option<Vec3>) {
        let right = Vec3::new(-self.yaw.sin(), 0.0, self.yaw.cos());
        let side = source.map_or(0.0, |s| {
            let mut to = s - self.position;
            to.y = 0.0;
            right.dot(to.normalize_or_zero())
        });
        let scale = self.punch_scale();
        self.hit_fx.punch.hit(damage, side, scale);
    }
}
