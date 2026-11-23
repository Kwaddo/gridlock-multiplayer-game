//! The awareness model as pure functions: how fast a bot notices someone it
//! sees, what a noise raises its awareness to, and how awareness fades. No
//! clock, no randomness, no world: everything comes in as arguments.

use super::tuning::*;

/// How an enemy carries themselves, for how easily they're seen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Posture {
    Standing,
    Sprinting,
    Crouched,
    Prone,
}

/// Whether the enemy hides in a cardboard box, and if it moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boxed {
    No,
    Still,
    Moving,
}

/// One enemy as a bot sees them at a sight check.
#[derive(Clone, Copy, Debug)]
pub struct SightInput {
    pub distance: f32,
    /// Farthest the bot sees at all.
    pub view_range: f32,
    /// Angle off the bot's view direction as a fraction of half its field of
    /// view: 0 straight ahead, 1 at the edge, more is outside it.
    pub off_axis: f32,
    /// 1 in full light, 0 in the dark.
    pub light: f32,
    pub posture: Posture,
    pub boxed: Boxed,
}

/// Awareness per second a bot with `perception` (see `tuning::perception`)
/// gains on an enemy it has a clear line to. 0 when it can't see them: too
/// far, outside its view, or a box standing still (scenery).
pub fn sight_rate(s: &SightInput, perception: f32) -> f32 {
    if s.distance > s.view_range || s.off_axis > 1.0 || s.boxed == Boxed::Still {
        return 0.0;
    }
    let near = SIGHT_RATE_AT_REF * SIGHT_REF_DIST / s.distance.max(SIGHT_NEAR_DIST);
    let edge = if s.off_axis <= EDGE_FULL_FRACTION {
        1.0
    } else {
        let t = (s.off_axis - EDGE_FULL_FRACTION) / (1.0 - EDGE_FULL_FRACTION);
        1.0 + (EDGE_MULT - 1.0) * t
    };
    let light = DARK_MULT + (1.0 - DARK_MULT) * s.light.clamp(0.0, 1.0);
    let posture = match s.posture {
        Posture::Standing => 1.0,
        Posture::Sprinting => SPRINT_SIGHT_MULT,
        Posture::Crouched => CROUCH_MULT,
        Posture::Prone => PRONE_MULT,
    };
    let boxed = if s.boxed == Boxed::Moving { BOX_MOVING_MULT } else { 1.0 };
    near * edge * light * posture * boxed * perception
}

/// How lit a spot (x, z) is: 1 within `LIT_RADIUS` of a light, 0 past
/// `DARK_RADIUS` of every light, in between linearly.
pub fn light_level(lights: impl Iterator<Item = (f32, f32)>, at: (f32, f32)) -> f32 {
    let nearest2 = lights
        .map(|(x, z)| (x - at.0).powi(2) + (z - at.1).powi(2))
        .fold(f32::INFINITY, f32::min);
    let d = nearest2.sqrt();
    if d <= LIT_RADIUS {
        1.0
    } else if d >= DARK_RADIUS {
        0.0
    } else {
        1.0 - (d - LIT_RADIUS) / (DARK_RADIUS - LIT_RADIUS)
    }
}

/// Whether `weapon` (a server weapon name) is suppressed: barely heard, and
/// no tracer or muzzle flash.
pub fn is_suppressed(weapon: &str) -> bool {
    SUPPRESSED_WEAPONS.iter().any(|w| w.eq_ignore_ascii_case(weapon))
}

/// How far a shot from `weapon` (a server weapon name) is heard.
pub fn gunfire_radius(weapon: &str) -> f32 {
    if weapon.eq_ignore_ascii_case("knife") || weapon.eq_ignore_ascii_case("punch") {
        MELEE_NOISE_RADIUS
    } else if is_suppressed(weapon) {
        SUPPRESSED_GUNFIRE_RADIUS
    } else {
        GUNFIRE_RADIUS
    }
}

/// 1 at the source of a noise heard within `radius`, falling to 0 at its edge.
pub fn falloff(distance: f32, radius: f32) -> f32 {
    if radius <= 0.0 || distance >= radius {
        0.0
    } else {
        1.0 - distance / radius
    }
}

/// The awareness a shot heard at `distance` raises the bot to (0 if it isn't
/// heard): suspicious at the edge of the radius, `HEARING_CAP` up close.
pub fn gunfire_floor(distance: f32, radius: f32) -> f32 {
    let f = falloff(distance, radius);
    if f <= 0.0 {
        return 0.0;
    }
    SUSPICIOUS_AT + (HEARING_CAP - SUSPICIOUS_AT) * f
}

/// How aware of an enemy a bot is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Alert {
    Unaware,
    Suspicious,
    Engaged,
}

/// A bot's awareness of one enemy over time.
#[derive(Clone, Copy, Debug, Default)]
pub struct Track {
    /// 0..1
    pub level: f32,
    /// Reached full awareness and hasn't fallen below `DISENGAGE_BELOW` since.
    pub engaged: bool,
    /// Seconds since the enemy was last seen or heard.
    pub since: f32,
}

impl Track {
    pub fn alert(&self) -> Alert {
        if self.engaged {
            Alert::Engaged
        } else if self.level >= SUSPICIOUS_AT {
            Alert::Suspicious
        } else {
            Alert::Unaware
        }
    }

    /// Start of a step: another `dt` since the enemy was last sensed.
    pub fn age(&mut self, dt: f32) {
        self.since += dt;
    }

    /// Sensed them: add `gain`, but not past `cap` (a level already above
    /// the cap stays where it is).
    pub fn add(&mut self, gain: f32, cap: f32) {
        if gain <= 0.0 {
            return;
        }
        if self.level < cap {
            self.level = (self.level + gain).min(cap);
        }
        self.since = 0.0;
    }

    /// Heard them: awareness is at least `floor` (0 = not heard).
    pub fn raise_to(&mut self, floor: f32) {
        if floor <= 0.0 {
            return;
        }
        self.level = self.level.max(floor.min(1.0));
        self.since = 0.0;
    }

    /// Fully aware at once (shot by them).
    pub fn engage(&mut self) {
        self.level = 1.0;
        self.engaged = true;
        self.since = 0.0;
    }

    /// End of a step: fade if not sensed for a while, latch or drop engaged.
    /// Returns the new alert level if it went up this step from `before`.
    pub fn settle(&mut self, dt: f32, before: Alert) -> Option<Alert> {
        let delay = if self.engaged { ENGAGED_DECAY_DELAY_SECS } else { DECAY_DELAY_SECS };
        let fading = (self.since - delay).clamp(0.0, dt);
        self.level = (self.level - DECAY_PER_SEC * fading).max(0.0);
        if self.level >= ENGAGED_AT - 1e-4 {
            self.engaged = true;
        } else if self.level < DISENGAGE_BELOW {
            self.engaged = false;
        }
        let after = self.alert();
        (after > before).then_some(after)
    }
}
