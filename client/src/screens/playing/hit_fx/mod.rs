//! Hit feedback that is only for the eyes (#63): the kill and headshot
//! markers. Nothing here touches the aim, the simulation or the network.

mod hud;
#[cfg(test)]
mod tests;

/// How long the kill marker stays up.
pub(super) const KILL_MARKER_SECS: f32 = 0.55;

#[derive(Default)]
pub struct HitFx {
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

    /// Advances the effects by a frame of `dt` real seconds.
    pub(super) fn tick(&mut self, dt: f32) {
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
