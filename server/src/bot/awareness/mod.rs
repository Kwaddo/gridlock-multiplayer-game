//! What a match bot knows about its enemies (issue #61). Each bot keeps an
//! awareness of 0..1 per enemy, built up by sight (`model::sight_rate`) and
//! hearing (gunfire, sprinting, landings) and fading slowly once they're out
//! of its senses. Suspicious (`SUSPICIOUS_AT`) makes it turn toward and go
//! check where it last noticed them; engaged (full) is the fight bots always
//! had. Bots only target, hunt and shoot enemies they're engaged with.
//!
//! Server side only; the sensing step is in `sense`, the maths in `model`,
//! every tunable in `tuning`.

pub mod model;
pub mod sense;
pub mod tuning;
#[cfg(test)]
mod tests;

pub use model::Alert;
pub use sense::Noise;

use model::Track;
use tuning::*;

/// One enemy a bot has noticed (or at least had in range of its senses).
#[derive(Clone, Debug)]
pub struct Known {
    pub id: u32,
    pub track: Track,
    /// Where the bot last saw or heard them.
    pub last_pos: (f32, f32, f32),
    /// Where they were at the last sight check, to tell a moving box from a still one.
    seen_pos: Option<(f32, f32, f32)>,
    /// In the air at the last step: landing makes a noise.
    was_airborne: bool,
    /// Still around this step; gone ones are dropped.
    fresh: bool,
    /// The level last announced to them, and seconds since (see `announce`).
    announced: (Alert, f32),
}

#[derive(Clone, Debug)]
pub struct Awareness {
    known: Vec<Known>,
    /// Seconds until the next sight check.
    sight_timer: f32,
    /// Seconds since the last sight check: what a sight check's gain covers.
    sight_elapsed: f32,
    /// Seconds since this bot last sensed any enemy (anti-stall).
    stall: f32,
    /// A rough spot near an enemy, and seconds left to head for it.
    hint: Option<((f32, f32, f32), f32)>,
    /// Hints given since the bot last sensed anyone: each is tighter.
    hints_given: i32,
    /// (enemy id, level) the bot's awareness just went up to, for the
    /// spotted player's "?" / "!" (taken by `BotManager`).
    alerts: Vec<(u32, Alert)>,
}

impl Default for Awareness {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl Awareness {
    /// `stagger` (0..1) offsets the first sight check so bots spawned
    /// together don't all raycast on the same tick.
    pub fn new(stagger: f32) -> Self {
        Self {
            known: Vec::new(),
            sight_timer: stagger.clamp(0.0, 1.0) * SIGHT_CHECK_SECS,
            sight_elapsed: 0.0,
            stall: 0.0,
            hint: None,
            hints_given: 0,
            alerts: Vec::new(),
        }
    }

    /// Forget everything (death, a new round, frozen at a spawn).
    pub fn reset(&mut self) {
        self.known.clear();
        self.alerts.clear();
        self.stall = 0.0;
        self.hint = None;
        self.hints_given = 0;
    }

    fn get(&self, id: u32) -> Option<&Known> {
        self.known.iter().find(|k| k.id == id)
    }

    fn entry(&mut self, id: u32, pos: (f32, f32, f32)) -> &mut Known {
        match self.known.iter().position(|k| k.id == id) {
            Some(i) => &mut self.known[i],
            None => {
                self.known.push(Known {
                    id,
                    track: Track::default(),
                    last_pos: pos,
                    seen_pos: None,
                    was_airborne: false,
                    fresh: true,
                    announced: (Alert::Unaware, 0.0),
                });
                self.known.last_mut().unwrap()
            }
        }
    }

    #[cfg(test)]
    pub fn level(&self, id: u32) -> f32 {
        self.get(id).map_or(0.0, |k| k.track.level)
    }

    pub fn alert(&self, id: u32) -> Alert {
        self.get(id).map_or(Alert::Unaware, |k| k.track.alert())
    }

    /// Engaged: the bot may target and shoot them.
    pub fn is_engaged(&self, id: u32) -> bool {
        self.alert(id) == Alert::Engaged
    }

    /// Hasn't noticed them at all: a knife in the back gets the backstab bonus.
    pub fn unaware_of(&self, id: u32) -> bool {
        self.alert(id) == Alert::Unaware
    }

    /// Engaged and sensed them moments ago: the bot goes after them where
    /// they are, not just where it last saw them.
    pub fn tracks(&self, id: u32) -> bool {
        self.get(id).is_some_and(|k| k.track.engaged && k.track.since <= ENGAGED_TRACK_SECS)
    }

    /// Where to go and check: the last known spot of the enemy the bot is
    /// most aware of without tracking them, unless it's already there.
    pub fn suspicion(&self, bot_pos: (f32, f32, f32)) -> Option<(f32, f32, f32)> {
        self.strongest(|k| {
            k.track.alert() >= Alert::Suspicious
                && !(k.track.engaged && k.track.since <= ENGAGED_TRACK_SECS)
                && flat_dist(k.last_pos, bot_pos) > INVESTIGATED_DIST
        })
    }

    /// Something noticed but not yet engaged: whether the bot should move warily.
    pub fn is_wary(&self) -> bool {
        self.known.iter().any(|k| k.track.alert() == Alert::Suspicious)
    }

    /// Where a suspicious bot turns to look: what it noticed moments ago.
    pub fn look_at(&self, bot_pos: (f32, f32, f32)) -> Option<(f32, f32, f32)> {
        self.strongest(|k| {
            k.track.alert() == Alert::Suspicious
                && k.track.since <= SUSPICIOUS_LOOK_SECS
                && flat_dist(k.last_pos, bot_pos) > INVESTIGATED_DIST
        })
    }

    /// The anti-stall hint, if the bot has one and isn't there yet.
    pub fn hint(&self, bot_pos: (f32, f32, f32)) -> Option<(f32, f32, f32)> {
        self.hint.map(|(pos, _)| pos).filter(|pos| flat_dist(*pos, bot_pos) > INVESTIGATED_DIST)
    }

    /// Shot by `id` standing at `pos`: fully aware of them at once.
    pub fn hurt_by(&mut self, id: u32, pos: (f32, f32, f32)) {
        let k = self.entry(id, pos);
        let before = k.track.alert();
        k.track.engage();
        k.last_pos = pos;
        if before != Alert::Engaged {
            self.announce(id, Alert::Engaged);
        }
        self.stall = 0.0;
    }

    /// Awareness of `id` went up to `level`: queue an alert for them, unless
    /// they were told this level (or higher) moments ago.
    fn announce(&mut self, id: u32, level: Alert) {
        let Some(k) = self.known.iter_mut().find(|k| k.id == id) else { return };
        let (told, ago) = k.announced;
        if level > told || ago >= ALERT_REPEAT_SECS {
            k.announced = (level, 0.0);
            self.alerts.push((id, level));
        }
    }

    /// The alerts queued since the last call: (enemy id, level).
    pub fn take_alerts(&mut self) -> std::vec::Drain<'_, (u32, Alert)> {
        self.alerts.drain(..)
    }

    fn strongest(&self, keep: impl Fn(&Known) -> bool) -> Option<(f32, f32, f32)> {
        self.known
            .iter()
            .filter(|k| keep(k))
            .max_by(|a, b| a.track.level.total_cmp(&b.track.level))
            .map(|k| k.last_pos)
    }
}

pub(crate) fn flat_dist(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.2 - b.2).powi(2)).sqrt()
}
