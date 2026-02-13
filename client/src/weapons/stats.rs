/*
 * Weapon stats and constants.
 * These are all the tunable numbers that define how each weapon behaves.
 * They are used by the client for visuals and feel, but the authoritative
 * logic lives on the server (see `server/src/weapons.rs`).
 */

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WeaponKind {
    Pistol,
    Smg,
    Shotgun,
    AssaultRifle,
    Sniper,
}

impl WeaponKind {
    // all weapon kinds in order (used for scroll-switching, or gun level based on kill etc.)
    pub const ALL: [WeaponKind; 5] = [
        WeaponKind::Pistol,
        WeaponKind::Smg,
        WeaponKind::Shotgun,
        WeaponKind::AssaultRifle,
        WeaponKind::Sniper,
    ];

    // for now i am making it slot based to simplify testing
    pub fn slot(self) -> usize {
        match self {
            WeaponKind::Pistol => 1,
            WeaponKind::Smg => 2,
            WeaponKind::Shotgun => 3,
            WeaponKind::AssaultRifle => 4,
            WeaponKind::Sniper => 5,
        }
    }

    pub fn from_slot(slot: usize) -> Option<Self> {
        WeaponKind::ALL.get(slot - 1).copied()
    }

    pub fn stats(self) -> &'static WeaponStats {
        match self {
            WeaponKind::Pistol => &PISTOL,
            WeaponKind::Smg => &SMG,
            WeaponKind::Shotgun => &SHOTGUN,
            WeaponKind::AssaultRifle => &ASSAULT_RIFLE,
            WeaponKind::Sniper => &SNIPER,
        }
    }
}

// crosshair visual style (drives how `crosshair.rs` renders).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrosshairStyle {
    // Standard weapon — hip-fire and ADS are texture crosshairs
    Standard,
    // Sniper — hip-fire is no crosshair, ADS is a full scope overlay
    Scope,
}

// fire mode determines input behaviour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FireMode {
    SemiAuto, // one shot per click
    FullAuto, // hold to keep firing
    BoltAction, // delay after each shot
}

// all the numbers that define how a weapon behaves and feels.
#[derive(Clone)]
pub struct WeaponStats {
    pub name: &'static str,
    pub fire_mode: FireMode,

    // --- Damage
    pub damage: f32,
    pub headshot_mult: f32,
    pub pellet_count: u32, // 1 for most guns, >1 for shotgun
    pub pellet_spread: Option<&'static [(f32,f32)]>, // (radian around circle, how far it deviates from center)

    // --- Fire rate & ammo
    pub fire_rate: f32, // rounds per second
    pub magazine_size: u32,
    pub max_reserve: u32, // total spare ammo
    pub reload_time: f32, // seconds

    // --- Recoil
    pub recoil_vertical: f32, // pitch kick per shot (radians)
    pub recoil_horizontal: f32, // max random yaw kick per shot (radians)
    pub recoil_recovery: f32, // how fast recoil settles back (rad/s)

    // --- Spread / bloom
    pub hip_spread: f32, // base spread at hip (radians)
    pub ads_spread: f32, // base spread when aimed (radians)
    pub spread_per_shot: f32, // bloom added per shot (radians)
    pub spread_recovery: f32, // bloom decay rate when not firing (rad/s)
    pub move_spread_mult: f32, // multiplier applied to spread when moving
    pub sprint_spread_mult: f32, // multiplier when sprinting (stacks with move)
    pub crouch_spread_mult: f32, // multiplier when crouching (<1 = tighter)

    // --- ADS (aim down sights)
    pub ads_zoom_fov: f32, // target FOV when ADS (lower = more zoom)
    pub ads_time: f32, // seconds to reach full ADS
    pub ads_sensitivity: Option<f32>, // custom sensitivity multiplier when ADS (None = use default), will only use this for sniper scope

    // --- Bullet visual
    pub bullet_speed: f32, // tracer speed (units/s), purely visual
    pub bullet_range: f32, // max hitscan distance (units)
    pub tracer_length: f32, // visible tracer trail length (units)
    pub tracer_color: [u8; 4], // color of the bullet tracer (for visual variety, not necessarily tied to damage or team color)
    pub tracer_thickness: f32, // thickness of the bullet tracer line (units)
    pub tracer_lifetime: f32, // how long the tracer trail lingers (seconds)

    // --- Crosshair kick (visual gap expansion on fire, not user-editable)
    pub crosshair_kick_per_shot: f32, // how much gap to add per shot (% of screen height)
    pub crosshair_kick_decay: f32, // how quickly the gap recovers (units per second)
    pub crosshair_kick_max: f32, // maximum kick gap

    // --- Crosshair
    pub crosshair_style: CrosshairStyle, // which render path (Standard = cross/texture, Scope = sniper overlay)
    pub fsa_time: Option<f32>, // seconds to reach first shot accuracy, None = never (SMG)
    // visual crosshair config (color, gap, thickness, textures) lives on CrosshairConfig in state.rs

    // --- HUD icon
    pub icon_asset_key: &'static str, // key used to look up texture in assets HashMap
    pub icon_width: f32, // display width in pixels (tweak per weapon)
    pub icon_height: f32, // display height in pixels
    pub icon_flip_x: bool, // true if the source image faces left

    // --- damage falloff
    pub falloff_start: f32, // distance at which damage falloff begins (units)
    pub falloff_end: f32, // distance at which damage reaches minimum (units)
    pub falloff_min_mult: f32, // minimum damage multiplier at max falloff (0..1)

    // --- spray pattern
    pub recoil_pattern: &'static [(f32, f32)], // list of (vertical, horizontal) recoil offsets for each shot in the magazine, applied in order and looping if you exceed the magazine size (like real guns with repeating patterns) rather than randomness shit
    pub pattern_randomness: f32, // how much random variance to add to the recoil pattern (radians),
    pub randomness_growth: f32, // additional randomness per shot to make the pattern less predictable the longer you fire (radians)
    pub pattern_reset_time: f32, // time in seconds to fully reset after releasing trigger



}

// --- Weapon definitions

pub const PISTOL: WeaponStats = WeaponStats {
    name: "Pistol",
    fire_mode: FireMode::SemiAuto,

    damage: 22.0,
    headshot_mult: 2.0,
    pellet_count: 1,
    pellet_spread: None,

    fire_rate: 5.0, // fast semi-auto sidearm
    magazine_size: 15,
    max_reserve: 60,
    reload_time: 1.3, // quick reload

    recoil_vertical: 0.018,
    recoil_horizontal: 0.005,
    recoil_recovery: 5.0,

    hip_spread: 0.012,
    ads_spread: 0.004,
    spread_per_shot: 0.02,
    spread_recovery: 2.0,
    move_spread_mult: 1.2,
    sprint_spread_mult: 1.5,
    crouch_spread_mult: 0.8,

    ads_zoom_fov: 65.0,
    ads_time: 0.15, // fastest ADS — it's a pistol
    ads_sensitivity: None,

    bullet_speed: 200.0,
    bullet_range: 40.0,
    tracer_length: 2.0,
    tracer_color:[255,200,80,255],
    tracer_thickness: 0.01,
    tracer_lifetime: 1.5,

    crosshair_kick_per_shot: 1.5,
    crosshair_kick_decay: 8.0,
    crosshair_kick_max: 1.5,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: Some(0.25),

    icon_asset_key: "weapon_pistol",
    icon_width: 80.0,
    icon_height: 50.0,
    icon_flip_x: false,

    falloff_start: 15.0,
    falloff_end: 35.0,
    falloff_min_mult: 0.5,

    recoil_pattern: &[
        (0.018, 0.0), 
        (0.018, 0.05),
        (0.017, -0.05),
        (0.016, 0.1),
        (0.015, -0.1),
    ],
    pattern_randomness: 0.15,
    randomness_growth: 0.08,
    pattern_reset_time: 0.4,
};

pub const SMG: WeaponStats = WeaponStats {
    name: "SMG",
    fire_mode: FireMode::FullAuto,

    damage: 20.0,
    headshot_mult: 1.5,
    pellet_count: 1,
    pellet_spread: None,
    fire_rate: 11.0, // high fire rate spray weapon
    magazine_size: 30,
    max_reserve: 120,
    reload_time: 1.8,

    recoil_vertical: 0.012,
    recoil_horizontal: 0.01,
    recoil_recovery: 5.0,

    hip_spread: 0.025,
    ads_spread: 0.012,
    spread_per_shot: 0.015,
    spread_recovery: 2.5,
    move_spread_mult: 1.1, // good run-and-gun weapon
    sprint_spread_mult: 1.4,
    crouch_spread_mult: 0.85,

    ads_zoom_fov: 70.0,
    ads_time: 0.18,
    ads_sensitivity: None,

    bullet_speed: 180.0,
    bullet_range: 45.0,
    tracer_length: 2.5,
    tracer_color:[255,200,80,255],
    tracer_thickness: 0.01,
    tracer_lifetime: 1.2,

    crosshair_kick_per_shot: 0.8,
    crosshair_kick_decay: 6.0,
    crosshair_kick_max: 2.5,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: None, // SMG never reaches FSA

    icon_asset_key: "weapon_smg",
    icon_width: 90.0,
    icon_height: 45.0,
    icon_flip_x: true,

    falloff_start: 10.0,
    falloff_end: 30.0,
    falloff_min_mult: 0.4,

    recoil_pattern: &[
    (0.012, 0.0),    // 1-3: straight up
    (0.012, 0.0),
    (0.012, 0.05),
    (0.011, 0.15),   // 4-6: drift right
    (0.011, 0.25),
    (0.010, 0.3),
    (0.010, 0.1),    // 7-9: swing left
    (0.009, -0.2),
    (0.009, -0.35),
    (0.008, -0.25),  // 10-12: settle back
    (0.008, -0.1),
    (0.007, 0.0),
],
    pattern_randomness: 0.3,
    randomness_growth: 0.04,
    pattern_reset_time: 0.35,
};

pub const SHOTGUN: WeaponStats = WeaponStats {
    name: "Shotgun",
    fire_mode: FireMode::SemiAuto,

    damage: 12.0, // per pellet — 8 pellets = 120 max (devastating up close)
    headshot_mult: 1.5,
    pellet_count: 9,
    pellet_spread: Some(&[
        (0.0,0.0), // center pellet
        (0.4363, 0.9), 
        (1.1345, 0.9),
        (2.0071, 0.9),
        (2.7053, 0.9),
        (3.5779, 0.9),
        (4.2761, 0.9),
        (5.1487, 0.9),
        (5.8469, 0.9),
    ]), 

    fire_rate: 1.0, // slow pump action
    magazine_size: 7,
    max_reserve: 28,
    reload_time: 2.2,

    recoil_vertical: 0.07,
    recoil_horizontal: 0.025,
    recoil_recovery: 2.5,

    hip_spread: 0.06, // decent hip-fire (shotguns don't need ADS as much)
    ads_spread: 0.035,
    spread_per_shot: 0.04,
    spread_recovery: 0.8,
    move_spread_mult: 1.3,
    sprint_spread_mult: 1.1,
    crouch_spread_mult: 0.85,

    ads_zoom_fov: 75.0,
    ads_time: 0.25,
    ads_sensitivity: None, // no sensitivity change when aiming — shotgun ADS is just for tighter spread, not zoom

    bullet_speed: 150.0,
    bullet_range: 20.0, // short range but not useless
    tracer_length: 1.5, // short trails for pellets
    tracer_color:[255,140,60,255],
    tracer_thickness: 0.008,
    tracer_lifetime: 0.8,

    crosshair_kick_per_shot: 2.5,
    crosshair_kick_decay: 8.0,
    crosshair_kick_max: 4.0,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: None, // shotgun has no FSA

    icon_asset_key: "weapon_shotgun",
    icon_width: 100.0,
    icon_height: 50.0,
    icon_flip_x: true,

    falloff_start: 5.0,
    falloff_end: 15.0,
    falloff_min_mult: 0.2,

    recoil_pattern: &[], // invalid for shotgun
    pattern_randomness: 0.0,
    randomness_growth: 0.0, // no pattern, so randomness doesnt increase
    pattern_reset_time: 0.0, // no pattern to reset
};

pub const ASSAULT_RIFLE: WeaponStats = WeaponStats {
    name: "Assault Rifle",
    fire_mode: FireMode::FullAuto,

    damage: 27.0, // 4-shot body kill (108), 3-shot headshot (27*1.8=48.6)
    headshot_mult: 1.8,
    pellet_count: 1,
    pellet_spread: None,

    fire_rate: 7.0, // slower than SMG but hits harder
    magazine_size: 30,
    max_reserve: 120,
    reload_time: 2.5, // heavier weapon = slower reload

    recoil_vertical: 0.01,
    recoil_horizontal: 0.008,
    recoil_recovery: 3.5,

    hip_spread: 0.03, // worse hipfire than SMG (heavier weapon)
    ads_spread: 0.006, // but better ADS accuracy
    spread_per_shot: 0.02,
    spread_recovery: 1.8,
    move_spread_mult: 1.5,
    sprint_spread_mult: 2.0,
    crouch_spread_mult: 0.75,

    ads_zoom_fov: 55.0,
    ads_time: 0.25,
    ads_sensitivity: None,

    bullet_speed: 250.0,
    bullet_range: 70.0, // best range of the auto weapons
    tracer_length: 3.5,
    tracer_color:[255,230,150,255],
    tracer_thickness: 0.015,
    tracer_lifetime: 1.5,

    crosshair_kick_per_shot: 0.8,
    crosshair_kick_decay: 5.0,
    crosshair_kick_max: 2.3,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: Some(0.35),

    icon_asset_key: "weapon_ar",
    icon_width: 100.0,
    icon_height: 45.0,
    icon_flip_x: false,

    falloff_start: 25.0,
    falloff_end: 70.0,
    falloff_min_mult: 0.6,

    recoil_pattern: &[
    (0.010, 0.0),    // shot 1: straight up
    (0.011, 0.0),    // shot 2: straight up
    (0.012, 0.1),    // shot 3: up, slight right
    (0.012, 0.2),    // shot 4: up, more right  
    (0.011, 0.3),    // shot 5: up, right
    (0.010, 0.1),    // shot 6: up, slight right
    (0.010, -0.2),   // shot 7: up, pulls left
    (0.009, -0.4),   // shot 8: up, more left
    (0.008, -0.3),   // shot 9: less kick, left
    (0.007, -0.1),   // shot 10: settling
],

    pattern_randomness: 0.1, // early on its very predictable,
    randomness_growth: 0.06, // but the longer you hold it, the more random it gets to prevent pure pattern memorization
    pattern_reset_time: 0.5,
};

pub const SNIPER: WeaponStats = WeaponStats {
    name: "Sniper",
    fire_mode: FireMode::BoltAction,

    damage: 105.0, // 1-shot body kill (105), 1-shot headshot (105*2.5=262)
    headshot_mult: 2.5,
    pellet_count: 1,
    pellet_spread: None,
    fire_rate: 0.8, // bolt action — very slow doesnt matter cause u have one magazine anyways lol
    magazine_size: 1,
    max_reserve: 8,
    reload_time: 4.3,

    recoil_vertical: 0.08,
    recoil_horizontal: 0.01,
    recoil_recovery: 2.0,

    hip_spread: 0.1, // terrible from hip
    ads_spread: 0.001, // pinpoint when scoped
    spread_per_shot: 0.06,
    spread_recovery: 0.8,
    move_spread_mult: 2.0, // heavy penalty
    sprint_spread_mult: 3.0,
    crouch_spread_mult: 0.5, // crouching + scoped = laser

    ads_zoom_fov: 15.0, // high zoom scope
    ads_time: 0.05,
    ads_sensitivity: Some(0.2), // much lower sensitivity when scoped for precision aiming

    bullet_speed: 350.0,
    bullet_range: 120.0, // longest range in the game
    tracer_length: 8.0, // long visible trail — sniper rounds are iconic
    tracer_color:[180, 200, 255, 255],
    tracer_thickness: 0.03,
    tracer_lifetime: 3.5,

    crosshair_kick_per_shot: 0.0,
    crosshair_kick_decay: 0.0,
    crosshair_kick_max: 0.0,

    crosshair_style: CrosshairStyle::Scope,
    fsa_time: None, // sniper doesnt use FSA (scope handles accuracy)

    icon_asset_key: "weapon_sniper",
    icon_width: 110.0,
    icon_height: 40.0,
    icon_flip_x: false,

    falloff_start: 1000.0,
    falloff_end: 10000.0,
    falloff_min_mult: 1.0, // sniper damage does not fall off — it's a long-range specialist, so it should be rewarded for landing shots at max range

    recoil_pattern: &[],
    pattern_randomness: 0.0, // sniper has no recoil pattern, just a single strong kick with some randomness
    randomness_growth: 0.0, // no pattern, so randomness doesnt increase
    pattern_reset_time: 0.0, // no pattern to reset
};


