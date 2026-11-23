//! Hit feedback that is only for the eyes (#63): impact puffs and the kill
//! and headshot markers. Nothing here touches the aim, the simulation or the
//! network.

mod hud;
mod particles;
#[cfg(test)]
mod tests;

pub(crate) use particles::ImpactKind;
use macroquad::prelude::Vec3;

/// How long the kill marker stays up.
pub(super) const KILL_MARKER_SECS: f32 = 0.55;

#[derive(Default)]
pub struct HitFx {
    particles: particles::Particles,
    /// Seconds left on the kill marker.
    kill_marker: f32,
    /// Whether that kill was a headshot.
    kill_headshot: bool,
}

impl HitFx {
    /// We killed someone: the kill marker replaces the hit marker for a moment.
    pub(crate) fn on_kill(&mut self, headshot: bool) {
        self.kill_marker = KILL_MARKER_SECS;
        self.kill_headshot = headshot;
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

    /// Advances the effects by a frame of `dt` real seconds.
    pub(super) fn tick(&mut self, dt: f32) {
        self.particles.update(dt);
        self.kill_marker = (self.kill_marker - dt).max(0.0);
    }
}

impl super::PlayingState {
    /// We got a kill (the server's kill feed, a practice bot dying here, or a
    /// Smoke Break find): show the kill marker.
    pub fn note_local_kill(&mut self, headshot: bool) {
        self.hit_fx.on_kill(headshot);
    }
}
