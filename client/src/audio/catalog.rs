//! Every sound the game plays, by key. This list is the authority: a sound
//! only exists if it's here and its file is in the manifest.
//!
//! Adding one: drop the file in client/assets/sounds, add
//! `"<key>": "assets/sounds/<file>.ogg"` to the `global` section of manifest.json,
//! add its cue below with a volume, and play it from one place.

/// A sound with its default mix level.
#[derive(Clone, Copy, Debug)]
pub struct Cue {
    pub key: &'static str,
    /// Volume before the master volume (0..1, may exceed 1 slightly).
    pub volume: f32,
    /// Random pitch variation per play (0.05 = ±5%), so repeats don't sound robotic.
    pub pitch_jitter: f32,
}

pub const fn cue(key: &'static str, volume: f32, pitch_jitter: f32) -> Cue {
    Cue { key, volume, pitch_jitter }
}

// every sound was removed on 2026-11-19 (they were generated tones and one shared pew),
// each one comes back on purpose, one at a time

/// smoke break: a hider taunting (Q), on repeat for the whole 5 s, everyone hears it from where they are
pub const TAUNT: Cue = cue("taunt", 0.9, 0.04);
/// how long taunt.mp3 is, so the repeats line up
pub const TAUNT_LENGTH: f32 = 0.65;
/// smoke break: any coin grabbed (the golden one too for now), everyone hears it from where it was,
/// that's how a hunter knows someone just took one
pub const COIN_PICKUP: Cue = cue("coin_pickup", 0.8, 0.05);
