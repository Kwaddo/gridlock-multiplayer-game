//! Every awareness tunable in one place. Distances are metres (a maze cell is
//! `MAZE_CELL_SIZE`, 1.8 m), rates are awareness (0..1) per second.

use crate::bot::personality::{BotArchetype, BotDifficulty, BotPersonality};
use crate::server::game::MAZE_CELL_SIZE;

// ── Thresholds ──────────────────────────────────────────────────────────────

/// From here on the bot is suspicious: it turns toward what it noticed, slows
/// down and goes to look.
pub const SUSPICIOUS_AT: f32 = 0.3;
/// Full awareness: the bot is engaged and fights as it always has.
pub const ENGAGED_AT: f32 = 1.0;
/// An engaged bot stays engaged until its awareness has fallen below this.
pub const DISENGAGE_BELOW: f32 = 0.5;

// ── Sight ───────────────────────────────────────────────────────────────────

/// Seconds between a bot's sight checks (each one a line-of-sight raycast per
/// enemy in its view cone). Bots start at different points, so they stagger.
pub const SIGHT_CHECK_SECS: f32 = 0.15;
/// At this distance an enemy in plain view (lit, standing, centre of view)
/// fills awareness at `SIGHT_RATE_AT_REF`; the rate scales with 1 / distance.
pub const SIGHT_REF_DIST: f32 = 12.0;
pub const SIGHT_RATE_AT_REF: f32 = 1.0;
/// Closer than this counts as this close: the fastest a bot notices anyone
/// is `SIGHT_RATE_AT_REF * SIGHT_REF_DIST / SIGHT_NEAR_DIST` (6 a second).
pub const SIGHT_NEAR_DIST: f32 = 2.0;
/// Farthest a bot sees: its personality's fighting range times this, at least
/// `SIGHT_MIN_RANGE`.
pub const SIGHT_RANGE_MULT: f32 = 1.5;
pub const SIGHT_MIN_RANGE: f32 = 30.0;
/// The inner part of the view cone (as a fraction of half the field of view)
/// that counts as straight ahead; past it the rate falls to `EDGE_MULT` at
/// the edge.
pub const EDGE_FULL_FRACTION: f32 = 0.4;
pub const EDGE_MULT: f32 = 0.35;
/// Within `LIT_RADIUS` of a ceiling light an enemy is in full light; past
/// `DARK_RADIUS` in the dark, where the rate is `DARK_MULT` of the lit one.
pub const LIT_RADIUS: f32 = 2.5 * MAZE_CELL_SIZE;
pub const DARK_RADIUS: f32 = 5.0 * MAZE_CELL_SIZE;
pub const DARK_MULT: f32 = 0.45;
/// Crouched and prone enemies are harder to spot; sprinting ones easier.
pub const CROUCH_MULT: f32 = 0.55;
pub const PRONE_MULT: f32 = 0.3;
pub const SPRINT_SIGHT_MULT: f32 = 1.25;

// ── Cardboard boxes ─────────────────────────────────────────────────────────

/// A box slower than this (m/s) is standing still: scenery, never noticed.
pub const BOX_STILL_SPEED: f32 = 0.4;
/// A moving box builds awareness at this fraction of a person's rate...
pub const BOX_MOVING_MULT: f32 = 0.25;
/// ...and only up to this: a bot gets suspicious of a box and goes to look,
/// but has nobody to fight until they come out.
pub const BOX_AWARENESS_CAP: f32 = 0.9;

// ── Hearing ─────────────────────────────────────────────────────────────────

/// Gunfire is heard this far (through walls too)...
pub const GUNFIRE_RADIUS: f32 = 30.0;
/// ...a suppressed weapon only this far...
pub const SUPPRESSED_GUNFIRE_RADIUS: f32 = 6.0;
/// ...and melee not at all.
pub const MELEE_NOISE_RADIUS: f32 = 0.0;
/// Weapons that count as suppressed (server weapon names).
pub const SUPPRESSED_WEAPONS: &[&str] = &["Suppressed Pistol", "AS VAL"];
/// Hearing alone never engages: it raises awareness to at most this.
pub const HEARING_CAP: f32 = 0.9;
/// A sprinting enemy is heard within this, at up to this rate (falling off
/// to nothing at the edge).
pub const SPRINT_NOISE_RADIUS: f32 = 7.0;
pub const SPRINT_NOISE_RATE: f32 = 0.8;
/// A landing is heard within this, adding up to this much at once.
pub const LANDING_NOISE_RADIUS: f32 = 6.0;
pub const LANDING_NOISE_BUMP: f32 = 0.35;

// ── Memory ──────────────────────────────────────────────────────────────────

/// Awareness holds for this long after the enemy was last sensed, then decays.
pub const DECAY_DELAY_SECS: f32 = 2.0;
/// The same, once engaged: a bot doesn't forget a fight that quickly.
pub const ENGAGED_DECAY_DELAY_SECS: f32 = 4.0;
pub const DECAY_PER_SEC: f32 = 0.08;
/// An engaged bot follows the enemy itself (not just where it last saw
/// them) for this long after last sensing them.
pub const ENGAGED_TRACK_SECS: f32 = 3.0;
/// A suspicious bot turns toward what it noticed for this long after.
pub const SUSPICIOUS_LOOK_SECS: f32 = 4.0;
/// A bot this close to where it last noticed something has checked it.
pub const INVESTIGATED_DIST: f32 = 1.5 * MAZE_CELL_SIZE;
/// Seconds an investigation of something noticed goes on before a re-plan.
pub const INVESTIGATE_SECS: f32 = 8.0;

// ── Anti-stall ──────────────────────────────────────────────────────────────

/// A bot that has sensed nobody for this long gets a rough hint...
pub const STALL_HINT_SECS: f32 = 15.0;
/// ...a spot up to this far from the nearest enemy...
pub const HINT_SPREAD: f32 = 4.0 * MAZE_CELL_SIZE;
/// ...which it heads for, for up to this long.
pub const HINT_SECS: f32 = 30.0;
/// Each further hint in the same stall is this much tighter, down to
/// `HINT_MIN_SPREAD`: a long stall ends with the bot close by.
pub const HINT_NARROWING: f32 = 0.6;
pub const HINT_MIN_SPREAD: f32 = 1.5 * MAZE_CELL_SIZE;

// ── Personality and difficulty ──────────────────────────────────────────────

/// How fast a bot notices people, by difficulty: a balance knob that doesn't
/// touch aim.
pub fn difficulty_perception(d: BotDifficulty) -> f32 {
    match d {
        BotDifficulty::Recruit => 0.6,
        BotDifficulty::Easy => 0.8,
        BotDifficulty::Normal => 1.0,
        BotDifficulty::Hard => 1.2,
        BotDifficulty::Nightmare => 1.5,
    }
}

/// How watchful each archetype is: angle holders watch, rushers don't.
pub fn archetype_perception(a: BotArchetype) -> f32 {
    match a {
        BotArchetype::Sniper => 1.15,
        BotArchetype::Camper => 1.1,
        BotArchetype::AllRounder => 1.0,
        BotArchetype::Flanker => 0.95,
        BotArchetype::Rusher => 0.85,
    }
}

/// Sight rate multiplier for a bot.
pub fn perception(p: &BotPersonality, d: BotDifficulty) -> f32 {
    difficulty_perception(d) * archetype_perception(p.archetype)
}

/// Hearing radius multiplier: bots keen on investigating noises hear farther.
pub fn hearing(p: &BotPersonality, d: BotDifficulty) -> f32 {
    (0.8 + 0.4 * p.noise_investigation_weight) * difficulty_perception(d).sqrt()
}
