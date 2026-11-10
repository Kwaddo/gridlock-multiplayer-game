//! Every sound the game plays, by key.
//!
//! A key is also the manifest key that replaces its synthesized placeholder
//! with a real file: add `"<key>": "assets/sounds/<file>.ogg"` to the
//! `global` section of manifest.json and drop the file in client/assets/sounds.
//! Volumes live here so a new file can be balanced without touching game code.

/// A sound with its default mix level.
#[derive(Clone, Copy, Debug)]
pub struct Cue {
    pub key: &'static str,
    /// Volume before the master volume (0..1, may exceed 1 slightly).
    pub volume: f32,
    /// Random pitch variation per play (0.05 = ±5%), so repeats don't sound robotic.
    pub pitch_jitter: f32,
}

const fn cue(key: &'static str, volume: f32, pitch_jitter: f32) -> Cue {
    Cue { key, volume, pitch_jitter }
}

// ── Movement ──
pub const FOOTSTEP: Cue = cue("footstep", 0.45, 0.08);
pub const SLIDE: Cue = cue("slide", 0.6, 0.05);
pub const JUMP: Cue = cue("jump", 0.35, 0.06);
pub const LAND: Cue = cue("land", 0.5, 0.06);

// ── Weapons (shots use each weapon's `fire_sound_asset_key`) ──
/// Volume of a gunshot at distance 0, before the master volume. Shots fire
/// many times a second, so they sit well below the other cues.
pub const SHOT_VOLUME: f32 = 0.22;

pub const DRY_FIRE: Cue = cue("dry_fire", 0.55, 0.03);
pub const RELOAD_START: Cue = cue("reload_start", 0.5, 0.03);
pub const RELOAD_END: Cue = cue("reload_end", 0.55, 0.03);
pub const WEAPON_SWITCH: Cue = cue("weapon_switch", 0.4, 0.05);
pub const WALL_IMPACT: Cue = cue("wall_impact", 0.35, 0.12);
pub const WALL_BREAK: Cue = cue("wall_break", 0.9, 0.06);

// ── Combat feedback ──
pub const HIT_MARKER: Cue = cue("hit_marker", 0.45, 0.02);
pub const HEADSHOT: Cue = cue("headshot", 0.55, 0.02);
pub const KILL_CONFIRM: Cue = cue("kill_confirm", 0.6, 0.0);
pub const HURT: Cue = cue("hurt", 0.6, 0.08);
pub const DEATH: Cue = cue("death", 0.7, 0.0);
pub const RESPAWN: Cue = cue("respawn", 0.5, 0.0);

// ── Match flow ──
pub const TELEPORT: Cue = cue("teleport", 0.55, 0.0);
pub const COUNTDOWN_TICK: Cue = cue("countdown_tick", 0.5, 0.0);
pub const MATCH_START: Cue = cue("match_start", 0.6, 0.0);
pub const MATCH_END: Cue = cue("match_end", 0.6, 0.0);
pub const LEVEL_UP: Cue = cue("level_up", 0.55, 0.0);
pub const CHAT: Cue = cue("chat", 0.35, 0.0);

/// Every key that gets a synthesized placeholder at startup.
pub const ALL_KEYS: &[&str] = &[
    FOOTSTEP.key,
    SLIDE.key,
    JUMP.key,
    LAND.key,
    DRY_FIRE.key,
    RELOAD_START.key,
    RELOAD_END.key,
    WEAPON_SWITCH.key,
    WALL_IMPACT.key,
    WALL_BREAK.key,
    HIT_MARKER.key,
    HEADSHOT.key,
    KILL_CONFIRM.key,
    HURT.key,
    DEATH.key,
    RESPAWN.key,
    TELEPORT.key,
    COUNTDOWN_TICK.key,
    MATCH_START.key,
    MATCH_END.key,
    LEVEL_UP.key,
    CHAT.key,
    // Weapon shots (`WeaponStats::fire_sound_asset_key`), mixed by the
    // shooting code rather than by a cue.
    "pistol_shot",
    "revolver_shot",
    "deagle_shot",
    "smg_shot",
    "ar_shot",
    "shotgun_shot",
    "sniper_shot",
    "knife_slash",
];
