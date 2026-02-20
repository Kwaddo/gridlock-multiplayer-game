/*
 * Weapon stats and constants.
 * These are all the tunable numbers that define how each weapon behaves.
 * They are used by the client for visuals and feel, but the authoritative
 * logic lives on the server (see `server/src/weapons.rs`).
 *
 * ARCHITECTURE:
 *   WeaponCategory — the broad class (Pistol, Smg, Shotgun, AssaultRifle, Sniper, Other)
 *   WeaponId       — a specific gun (M1911, Deagle, Ak, Mp5, …)
 *
 * To add a new gun:
 *   1. Add a variant to `WeaponId`
 *   2. Create a `const YOUR_GUN: WeaponStats = WeaponStats { name: "Your Gun", … };`
 *   3. Wire it in `WeaponId::stats()` and `WeaponId::category()`
 *   4. Add the model path to manifest.json
 */

// --- weapon category (broad class) ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WeaponCategory {
    Pistol,
    Smg,
    Shotgun,
    AssaultRifle,
    Sniper,
    Other,
}

impl WeaponCategory {
    pub const ALL: [WeaponCategory; 6] = [
        WeaponCategory::Pistol,
        WeaponCategory::Smg,
        WeaponCategory::Shotgun,
        WeaponCategory::AssaultRifle,
        WeaponCategory::Sniper,
        WeaponCategory::Other,
    ];

    pub fn display_name(self) -> &'static str {
        match self {
            WeaponCategory::Pistol => "Pistols",
            WeaponCategory::Smg => "SMGs",
            WeaponCategory::Shotgun => "Shotguns",
            WeaponCategory::AssaultRifle => "Assault Rifles",
            WeaponCategory::Sniper => "Snipers",
            WeaponCategory::Other => "Other",
        }
    }
}

// --- weapon id (specific gun) ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WeaponId {
    // Pistols
    M1911,
    Deagle,
    Revolver,
    // SuppressedPistol,
    // TaurusRaging,

    // SMGs
    Mp5,
    // P90,
    // HoneyBadger,
    // TommyGun,

    // Shotguns
    // Benelli,
    Spas,

    // Assault Rifles
    // M4a1,
    // Ak,
    // Famas,
    M16,
    // Scar,
    // AsVal,

    // Snipers
    BoltAction,
    // HuntingRifle,
    // SemiAutoSniper,

    // Other
    // GrenadeLauncher,
    Minigun,
    // Rpg,
}

impl WeaponId {
    pub const ALL: &[WeaponId] = &[
        // Pistols
        WeaponId::M1911,
        WeaponId::Deagle,
        WeaponId::Revolver,
        // WeaponId::SuppressedPistol,
        // WeaponId::TaurusRaging,

        // SMGs
        WeaponId::Mp5,
        // WeaponId::P90,
        // WeaponId::HoneyBadger,
        // WeaponId::TommyGun,

        // Shotguns
        // WeaponId::Benelli,
        WeaponId::Spas,

        // Assault Rifles
        // WeaponId::M4a1,
        // WeaponId::Ak,
        // WeaponId::Famas,
        WeaponId::M16,
        // WeaponId::Scar,
        // WeaponId::AsVal,

        // Snipers
        WeaponId::BoltAction,
        // WeaponId::HuntingRifle,
        // WeaponId::SemiAutoSniper,

        // Other
        // WeaponId::GrenadeLauncher,
        WeaponId::Minigun,
        // WeaponId::Rpg,
    ];

    pub fn category(self) -> WeaponCategory {
        match self {
            WeaponId::M1911 | WeaponId::Deagle | WeaponId::Revolver
            // | WeaponId::SuppressedPistol | WeaponId::TaurusRaging 
            => WeaponCategory::Pistol,

            WeaponId::Mp5 
            // | WeaponId::P90 | WeaponId::HoneyBadger
            // | WeaponId::TommyGun 
            => WeaponCategory::Smg,

            // WeaponId::Benelli | 
            WeaponId::Spas 
            => WeaponCategory::Shotgun,

            WeaponId::M16 
            // | WeaponId::M4a1 | WeaponId::Ak | WeaponId::Famas | 
            // | WeaponId::Scar | WeaponId::AsVal 
            => WeaponCategory::AssaultRifle,

            WeaponId::BoltAction 
            // | WeaponId::HuntingRifle | WeaponId::SemiAutoSniper 
            => WeaponCategory::Sniper,

            // WeaponId::GrenadeLauncher | WeaponId::Rpg |
            WeaponId::Minigun
            => WeaponCategory::Other,
        }
    }

    pub fn stats(self) -> &'static WeaponStats {
        match self {
            // Pistols
            WeaponId::M1911 => &M1911,
            WeaponId::Deagle => &DEAGLE,
            WeaponId::Revolver => &REVOLVER,
            // WeaponId::SuppressedPistol => &SUPPRESSED_PISTOL,
            // WeaponId::TaurusRaging => &TAURUS_RAGING,

            // SMGs
            WeaponId::Mp5 => &MP5,
            // WeaponId::P90 => &P90,
            // WeaponId::HoneyBadger => &HONEY_BADGER,
            // WeaponId::TommyGun => &TOMMY_GUN,

            // Shotguns
            // WeaponId::Benelli => &BENELLI,
            WeaponId::Spas => &SPAS,

            // Assault Rifles
            // WeaponId::M4a1 => &M4A1,
            // WeaponId::Ak => &AK,
            // WeaponId::Famas => &FAMAS,
            WeaponId::M16 => &M16_STATS,
            // WeaponId::Scar => &SCAR,
            // WeaponId::AsVal => &AS_VAL,

            // Snipers
            WeaponId::BoltAction => &BOLT_ACTION,
            // WeaponId::HuntingRifle => &HUNTING_RIFLE,
            // WeaponId::SemiAutoSniper => &SEMI_AUTO_SNIPER,

            // Other
            // WeaponId::GrenadeLauncher => &GRENADE_LAUNCHER,
            WeaponId::Minigun => &MINIGUN,
            // WeaponId::Rpg => &RPG,
        }
    }

    pub fn display_name(self) -> &'static str {
        self.stats().name
    }

    /// All weapon IDs belonging to a given category.
    pub fn all_in(cat: WeaponCategory) -> Vec<WeaponId> {
        Self::ALL.iter().copied().filter(|w| w.category() == cat).collect()
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
    SemiAuto,   // one shot per click
    FullAuto,   // hold to keep firing
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
    pub pellet_spread: Option<&'static [(f32, f32)]>, // (radian around circle, how far it deviates from center)

    // --- Fire rate & ammo
    pub fire_rate: f32, // rounds per second
    pub magazine_size: u32,
    pub max_reserve: u32, // total spare ammo
    pub reload_time: f32, // seconds

    // --- Recoil
    pub recoil_vertical: f32,   // pitch kick per shot (radians)
    pub recoil_horizontal: f32, // max random yaw kick per shot (radians)
    pub recoil_recovery: f32,   // how fast recoil settles back (rad/s)

    // --- Spread / bloom
    pub hip_spread: f32,         // base spread at hip (radians)
    pub ads_spread: f32,         // base spread when aimed (radians)
    pub spread_per_shot: f32,    // bloom added per shot (radians)
    pub spread_recovery: f32,    // bloom decay rate when not firing (rad/s)
    pub move_spread_mult: f32,   // multiplier applied to spread when moving
    pub sprint_spread_mult: f32, // multiplier when sprinting (stacks with move)
    pub crouch_spread_mult: f32, // multiplier when crouching (<1 = tighter)

    // --- ADS (aim down sights)
    pub ads_zoom_fov: f32,            // target FOV when ADS (lower = more zoom)
    pub ads_time: f32,                // seconds to reach full ADS
    pub ads_sensitivity: Option<f32>, // custom sensitivity multiplier when ADS (None = use default), will only use this for sniper scope

    // --- Bullet visual
    pub bullet_speed: f32,     // tracer speed (units/s), purely visual
    pub bullet_range: f32,     // max hitscan distance (units)
    pub tracer_length: f32,    // visible tracer trail length (units)
    pub tracer_color: [u8; 4], // color of the bullet tracer (for visual variety, not necessarily tied to damage or team color)
    pub tracer_thickness: f32, // thickness of the bullet tracer line (units)
    pub tracer_lifetime: f32,  // how long the tracer trail lingers (seconds)

    // --- Crosshair kick (visual gap expansion on fire, not user-editable)
    pub crosshair_kick_per_shot: f32, // how much gap to add per shot (% of screen height)
    pub crosshair_kick_decay: f32,    // how quickly the gap recovers (units per second)
    pub crosshair_kick_max: f32,      // maximum kick gap

    // --- Crosshair
    pub crosshair_style: CrosshairStyle, // which render path (Standard = cross/texture, Scope = sniper overlay)
    pub fsa_time: Option<f32>,           // seconds to reach first shot accuracy, None = never (SMG)
    // visual crosshair config (color, gap, thickness, textures) lives on CrosshairConfig in state.rs

    // --- HUD icon
    pub fire_sound_asset_key: &'static str, // key used to look up shot sound in manifest-loaded audio map
    pub icon_asset_key: &'static str,       // key used to look up texture in assets HashMap
    pub icon_width: f32,                    // display width in pixels (tweak per weapon)
    pub icon_height: f32,                   // display height in pixels
    pub icon_flip_x: bool,                  // true if the source image faces left

    // --- First-person viewmodel (3D model or 2D sprite fallback)
    pub viewmodel_model: Option<&'static str>, // .obj model path
    pub viewmodel_asset_key: Option<&'static str>, // 2D sprite fallback
    pub viewmodel_scale: f32,                  // multiplier for model size
    pub viewmodel_position: (f32, f32, f32),   // position offset (right, up, forward)
    pub viewmodel_rotation: (f32, f32, f32),   // rotation (pitch, yaw, roll) in radians
    pub viewmodel_muzzle_offset: (f32, f32, f32), // muzzle in normalized model space
    pub viewmodel_hands_scale: f32,            // per-weapon hands scale multiplier
    pub viewmodel_hands_offset: (f32, f32, f32), // per-weapon hand position offset
    pub viewmodel_hands_rotation: (f32, f32, f32), // per-weapon hand rotation (pitch, yaw, roll)
    pub viewmodel_pair_offset_pos: (f32, f32, f32), // hand+gun locked-pair position offset
    pub viewmodel_pair_offset_rot: (f32, f32, f32), // hand+gun locked-pair rotation offset (pitch, yaw, roll)
    pub viewmodel_ads_pos_offset: (f32, f32, f32),        // gun position delta at full ADS
    pub viewmodel_ads_rot_offset: (f32, f32, f32),        // gun rotation delta at full ADS
    pub viewmodel_ads_hands_pos_offset: (f32, f32, f32),  // hands position delta at full ADS
    pub viewmodel_ads_hands_rot_offset: (f32, f32, f32),  // hands rotation delta at full ADS

    // --- damage falloff
    pub falloff_start: f32, // distance at which damage falloff begins (units)
    pub falloff_end: f32,   // distance at which damage reaches minimum (units)
    pub falloff_min_mult: f32, // minimum damage multiplier at max falloff (0..1)

    // --- spray pattern
    pub recoil_pattern: &'static [(f32, f32)], // list of (vertical, horizontal) recoil offsets for each shot in the magazine, applied in order and looping if you exceed the magazine size (like real guns with repeating patterns) rather than randomness shit
    pub pattern_randomness: f32, // how much random variance to add to the recoil pattern (radians),
    pub randomness_growth: f32, // additional randomness per shot to make the pattern less predictable the longer you fire (radians)
    pub pattern_reset_time: f32, // time in seconds to fully reset after releasing trigger
}

const VM_BASE_SCALE: f32 = 0.003;
const VM_BASE_POS: (f32, f32, f32) = (0.30, 0.08, -0.37);
const VM_BASE_ROT: (f32, f32, f32) = (0.05, -1.45, 0.0);

// ============================================================================
//  PISTOLS
// ============================================================================

/// M1911 — classic reliable sidearm, consistent and dependable
pub const M1911: WeaponStats = WeaponStats {
    name: "M1911",
    fire_mode: FireMode::SemiAuto,

    damage: 22.0,
    headshot_mult: 2.0,
    pellet_count: 1,
    pellet_spread: None,

    fire_rate: 5.0,
    magazine_size: 15,
    max_reserve: 60,
    reload_time: 1.3,

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
    ads_time: 0.15,
    ads_sensitivity: None,

    bullet_speed: 200.0,
    bullet_range: 40.0,
    tracer_length: 2.0,
    tracer_color: [255, 200, 80, 255],
    tracer_thickness: 0.01,
    tracer_lifetime: 1.5,

    crosshair_kick_per_shot: 1.5,
    crosshair_kick_decay: 8.0,
    crosshair_kick_max: 1.5,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: Some(0.25),

    fire_sound_asset_key: "pistol_shot",
    icon_asset_key: "pistol_icon",
    icon_width: 80.0,
    icon_height: 50.0,
    icon_flip_x: false,

    viewmodel_model: Some("m1911_model"),
    viewmodel_asset_key: Some("weapon_fp_pistol"),
    viewmodel_scale: VM_BASE_SCALE,
    viewmodel_position: VM_BASE_POS,
    viewmodel_rotation: VM_BASE_ROT,
    viewmodel_muzzle_offset: (0.26, 0.32, -0.20),
    viewmodel_hands_scale: 1.93,
    viewmodel_hands_offset: (0.0, 0.0, 0.0),
    viewmodel_hands_rotation: (-0.15, 0.65, -0.05),
    viewmodel_pair_offset_pos: (0.0, 0.0, 0.0),
    viewmodel_pair_offset_rot: (0.0, 0.0, 0.0),
    viewmodel_ads_pos_offset: (0.08, 0.03, 0.0),
    viewmodel_ads_rot_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_pos_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_rot_offset: (0.0, 0.0, 0.0),

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

// Desert Eagle — hand cannon, sniper pistol with massive recoil
pub const DEAGLE: WeaponStats = WeaponStats {
    name: "Desert Eagle",
    fire_mode: FireMode::SemiAuto,

    damage: 75.0,
    headshot_mult: 2.0,
    pellet_count: 1,
    pellet_spread: None,

    fire_rate: 1.2,
    magazine_size: 6,
    max_reserve: 35,
    reload_time: 2.0,

    recoil_horizontal: 0.045,
    recoil_vertical: 0.015,
    recoil_recovery: 3.0,

    hip_spread: 0.025,
    ads_spread: 0.005,
    spread_per_shot: 0.03,
    spread_recovery: 1.5,
    move_spread_mult: 1.4,
    sprint_spread_mult: 1.8,
    crouch_spread_mult: 0.7,

    ads_zoom_fov: 60.0,
    ads_time: 0.2,
    ads_sensitivity: None,

    bullet_speed: 220.0,
    bullet_range: 50.0,
    tracer_length: 3.0,
    tracer_color: [255, 210, 90, 255],
    tracer_thickness: 0.02,
    tracer_lifetime: 1.8,

    crosshair_kick_per_shot: 2.0,
    crosshair_kick_decay: 6.0,
    crosshair_kick_max: 3.0,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: Some(0.3),

    fire_sound_asset_key: "deagle_shot",
    icon_asset_key: "pistol_icon",
    icon_width: 90.0,
    icon_height: 50.0,
    icon_flip_x: false,

    viewmodel_model: Some("deagle_model"),
    viewmodel_asset_key: None,
    viewmodel_scale: VM_BASE_SCALE,
    viewmodel_position: VM_BASE_POS,
    viewmodel_rotation: VM_BASE_ROT,
    viewmodel_muzzle_offset: (0.26, 0.32, -0.20),
    viewmodel_hands_scale: 1.93,
    viewmodel_hands_offset: (0.0, 0.0, 0.0),
    viewmodel_hands_rotation: (-0.15, 0.65, -0.05),
    viewmodel_pair_offset_pos: (0.0, 0.0, 0.0),
    viewmodel_pair_offset_rot: (0.0, 0.0, 0.0),
    viewmodel_ads_pos_offset: (0.08, 0.03, 0.0),
    viewmodel_ads_rot_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_pos_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_rot_offset: (0.0, 0.0, 0.0),

    falloff_start: 20.0,
    falloff_end: 60.0,
    falloff_min_mult: 0.75,

    recoil_pattern: &[
        (0.045, 0.0),
        (0.040, 0.08),
        (0.038, -0.10),
        (0.035, 0.12),
        (0.032, -0.08),
        (0.030, 0.05),
        (0.028, -0.05),
    ],
    pattern_randomness: 0.2,
    randomness_growth: 0.1,
    pattern_reset_time: 1.2,
};

//  Revolver — slow high damage precision sidearm
pub const REVOLVER: WeaponStats = WeaponStats {
    name: "Revolver",
    fire_mode: FireMode::SemiAuto,

    damage: 45.0,
    headshot_mult: 2.5, // = 112.5
    pellet_count: 1,
    pellet_spread: None,

    fire_rate: 1.6,
    magazine_size: 6,
    max_reserve: 36,
    reload_time: 2.2,

    recoil_horizontal: 0.035,
    recoil_vertical: 0.02,
    recoil_recovery: 3.5,
    hip_spread: 0.03,
    ads_spread: 0.006,
    spread_per_shot: 0.025,
    spread_recovery: 1.8,
    move_spread_mult: 1.3,
    sprint_spread_mult: 1.6,
    crouch_spread_mult: 0.75,
    ads_zoom_fov: 60.0,
    ads_time: 0.25,
    ads_sensitivity: None,

    bullet_speed: 210.0,
    bullet_range: 45.0,
    tracer_length: 2.5,
    tracer_color: [255, 205, 85, 255],
    tracer_thickness: 0.015,
    tracer_lifetime: 1.7,

    crosshair_kick_per_shot: 1.8,
    crosshair_kick_decay: 5.0,
    crosshair_kick_max: 2.5,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: Some(0.35),

    fire_sound_asset_key: "revolver_shot",
    icon_asset_key: "pistol_icon",
    icon_width: 85.0,
    icon_height: 50.0,
    icon_flip_x: false,

    viewmodel_model: Some("revolver_model"),
    viewmodel_asset_key: None,

    viewmodel_scale: VM_BASE_SCALE,
    viewmodel_position: VM_BASE_POS,
    viewmodel_rotation: VM_BASE_ROT,
    viewmodel_muzzle_offset: (0.26, 0.32, -0.20),
    viewmodel_hands_scale: 1.93,
    viewmodel_hands_offset: (0.0, 0.0, 0.0),
    viewmodel_hands_rotation: (-0.15, 0.65, -0.05),
    viewmodel_pair_offset_pos: (0.0, 0.0, 0.0),
    viewmodel_pair_offset_rot: (0.0, 0.0, 0.0),
    viewmodel_ads_pos_offset: (0.08, 0.03, 0.0),
    viewmodel_ads_rot_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_pos_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_rot_offset: (0.0, 0.0, 0.0),

    falloff_start: 80.0,
    falloff_end: 200.0,
    falloff_min_mult: 0.9,

    recoil_pattern: &[
        (0.035, 0.0),
        (0.030, 0.1),
        (0.028, -0.1),
        (0.025, 0.15),
        (0.022, -0.15),
        (0.020, 0.1),
        (0.018, -0.1),
    ],
    pattern_randomness: 0.25,
    randomness_growth: 0.12,
    pattern_reset_time: 0.85,
};

// TODO: Suppressed Pistol — stealth pistol, silent assassin
pub const SUPPRESSED_PISTOL: WeaponStats = WeaponStats {
    name: "Suppressed Pistol",
    viewmodel_model: Some("suppressed_pistol_model"),
    viewmodel_asset_key: None,
    ..M1911
};

// TODO: Taurus Raging Judge — hybrid revolver shotgun, alt-fire shotgun blast
pub const TAURUS_RAGING: WeaponStats = WeaponStats {
    name: "Taurus Raging",
    viewmodel_model: Some("taurus_raging_model"),
    viewmodel_asset_key: None,
    ..M1911
};

// ============================================================================
//  SMGS
// ============================================================================

/// MP5 — control SMG, laser accuracy spray
pub const MP5: WeaponStats = WeaponStats {
    name: "MP5",
    fire_mode: FireMode::FullAuto,

    damage: 20.0,
    headshot_mult: 1.5,
    pellet_count: 1,
    pellet_spread: None,
    fire_rate: 11.0,
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
    move_spread_mult: 1.1,
    sprint_spread_mult: 1.4,
    crouch_spread_mult: 0.85,

    ads_zoom_fov: 70.0,
    ads_time: 0.18,
    ads_sensitivity: None,

    bullet_speed: 180.0,
    bullet_range: 45.0,
    tracer_length: 2.5,
    tracer_color: [255, 200, 80, 255],
    tracer_thickness: 0.01,
    tracer_lifetime: 1.2,

    crosshair_kick_per_shot: 0.8,
    crosshair_kick_decay: 6.0,
    crosshair_kick_max: 2.5,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: None,

    fire_sound_asset_key: "smg_shot",
    icon_asset_key: "smg_icon",
    icon_width: 90.0,
    icon_height: 45.0,
    icon_flip_x: true,

    viewmodel_model: Some("mp5_model"),
    viewmodel_asset_key: None,
    viewmodel_scale: VM_BASE_SCALE,
    viewmodel_position: VM_BASE_POS,
    viewmodel_rotation: VM_BASE_ROT,
    viewmodel_muzzle_offset: (-0.90, 0.02, -0.22),
    viewmodel_hands_scale: 1.93,
    viewmodel_hands_offset: (0.0, 0.0, 0.0),
    viewmodel_hands_rotation: (-0.15, 0.65, -0.05),
    viewmodel_pair_offset_pos: (0.0, 0.0, 0.0),
    viewmodel_pair_offset_rot: (0.0, 0.0, 0.0),
    viewmodel_ads_pos_offset: (0.08, 0.03, 0.0),
    viewmodel_ads_rot_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_pos_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_rot_offset: (0.0, 0.0, 0.0),

    falloff_start: 10.0,
    falloff_end: 30.0,
    falloff_min_mult: 0.4,

    recoil_pattern: &[
        (0.012, 0.0),
        (0.012, 0.0),
        (0.012, 0.05),
        (0.011, 0.15),
        (0.011, 0.25),
        (0.010, 0.3),
        (0.010, 0.1),
        (0.009, -0.2),
        (0.009, -0.35),
        (0.008, -0.25),
        (0.008, -0.1),
        (0.007, 0.0),
    ],
    pattern_randomness: 0.3,
    randomness_growth: 0.04,
    pattern_reset_time: 0.35,
};

// TODO: P90 — bullet hose, huge mag + high fire rate
pub const P90: WeaponStats = WeaponStats {
    name: "P90",
    viewmodel_model: Some("p90_model"),
    viewmodel_asset_key: None,
    ..MP5
};

// TODO: Honey Badger — stealth assault SMG hybrid, suppressed + rifle damage feel
pub const HONEY_BADGER: WeaponStats = WeaponStats {
    name: "Honey Badger",
    viewmodel_model: Some("honey_badger_model"),
    viewmodel_asset_key: None,
    ..MP5
};

// TODO: Tommy Gun — vintage SMG, heavy recoil but strong close range
pub const TOMMY_GUN: WeaponStats = WeaponStats {
    name: "Tommy Gun",
    viewmodel_model: Some("tommy_gun_model"),
    viewmodel_asset_key: None,
    ..MP5
};

// ============================================================================
//  SHOTGUNS
// ============================================================================

// Benelli is a stub — Spas is the default shotgun
pub const BENELLI: WeaponStats = WeaponStats {
    name: "Benelli",
    viewmodel_model: Some("benelli_model"),
    viewmodel_asset_key: None,
    ..SPAS
};

/// SPAS-12 — tactical shotgun, mid-range consistency
pub const SPAS: WeaponStats = WeaponStats {
    name: "SPAS-12",
    fire_mode: FireMode::SemiAuto,

    damage: 12.0,
    headshot_mult: 1.5,
    pellet_count: 9,
    pellet_spread: Some(&[
        (0.0, 0.0),
        (0.4363, 0.9),
        (1.1345, 0.9),
        (2.0071, 0.9),
        (2.7053, 0.9),
        (3.5779, 0.9),
        (4.2761, 0.9),
        (5.1487, 0.9),
        (5.8469, 0.9),
    ]),

    fire_rate: 1.0,
    magazine_size: 7,
    max_reserve: 28,
    reload_time: 2.2,

    recoil_vertical: 0.07,
    recoil_horizontal: 0.025,
    recoil_recovery: 2.5,

    hip_spread: 0.06,
    ads_spread: 0.035,
    spread_per_shot: 0.04,
    spread_recovery: 0.8,
    move_spread_mult: 1.3,
    sprint_spread_mult: 1.1,
    crouch_spread_mult: 0.85,

    ads_zoom_fov: 75.0,
    ads_time: 0.25,
    ads_sensitivity: None,

    bullet_speed: 150.0,
    bullet_range: 20.0,
    tracer_length: 1.5,
    tracer_color: [255, 140, 60, 255],
    tracer_thickness: 0.008,
    tracer_lifetime: 0.8,

    crosshair_kick_per_shot: 2.5,
    crosshair_kick_decay: 8.0,
    crosshair_kick_max: 4.0,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: None,

    fire_sound_asset_key: "shotgun_shot",
    icon_asset_key: "shotgun_icon",
    icon_width: 100.0,
    icon_height: 50.0,
    icon_flip_x: true,

    viewmodel_model: Some("spas_model"),
    viewmodel_asset_key: Some("weapon_fp_shotgun"),
    viewmodel_scale: 0.0020,
    viewmodel_position: (0.220, 0.130, -0.440),
    viewmodel_rotation: (-0.100, -1.450, 0.000),
    viewmodel_muzzle_offset: (-1.100, 0.020, -0.250),
    viewmodel_hands_scale: 1.93,
    viewmodel_hands_offset: (0.180, -0.120, -0.466),
    viewmodel_hands_rotation: (-0.050, 0.900, 0.100),
    viewmodel_pair_offset_pos: (0.044, 0.000, 0.000),
    viewmodel_pair_offset_rot: (0.100, 0.000, 0.000),
    viewmodel_ads_pos_offset: (0.125, 0.025, -0.060),
    viewmodel_ads_rot_offset: (0.275, -0.130, 0.125),
    viewmodel_ads_hands_pos_offset: (0.134, 0.026, -0.537),
    viewmodel_ads_hands_rot_offset: (0.100, 0.000, 0.085),

    falloff_start: 5.0,
    falloff_end: 15.0,
    falloff_min_mult: 0.2,

    recoil_pattern: &[],
    pattern_randomness: 0.0,
    randomness_growth: 0.0,
    pattern_reset_time: 0.0,
};

// ============================================================================
//  ASSAULT RIFLES
// ============================================================================

// M4A1 - versatile assault rifle, balanced stats all around, good for learning the game but its mainly here cause the model was cool
pub const M4A1: WeaponStats = WeaponStats {
    name: "M4A1",
    viewmodel_model: Some("m4a1_model"),
    viewmodel_asset_key: None,
    ..M16_STATS
};

// TODO: AK-47 — high damage / high recoil, strongest per shot AR, hardest to control
pub const AK: WeaponStats = WeaponStats {
    name: "AK-47",
    viewmodel_model: Some("ak_model"),
    viewmodel_asset_key: None,
    ..M16_STATS
};

// TODO: FAMAS — burst rifle, burst accuracy reward weapon
pub const FAMAS: WeaponStats = WeaponStats {
    name: "FAMAS",
    viewmodel_model: Some("famas_model"),
    viewmodel_asset_key: None,
    ..M16_STATS
};

/// M16 — precision burst rifle, tap/burst accuracy king
pub const M16_STATS: WeaponStats = WeaponStats {
    name: "M16",
    fire_mode: FireMode::FullAuto,

    damage: 27.0,
    headshot_mult: 1.8,
    pellet_count: 1,
    pellet_spread: None,

    fire_rate: 7.0,
    magazine_size: 30,
    max_reserve: 120,
    reload_time: 2.5,

    recoil_vertical: 0.01,
    recoil_horizontal: 0.008,
    recoil_recovery: 3.5,

    hip_spread: 0.03,
    ads_spread: 0.006,
    spread_per_shot: 0.02,
    spread_recovery: 1.8,
    move_spread_mult: 1.5,
    sprint_spread_mult: 2.0,
    crouch_spread_mult: 0.75,

    ads_zoom_fov: 55.0,
    ads_time: 0.25,
    ads_sensitivity: None,

    bullet_speed: 250.0,
    bullet_range: 70.0,
    tracer_length: 3.5,
    tracer_color: [255, 230, 150, 255],
    tracer_thickness: 0.015,
    tracer_lifetime: 1.5,

    crosshair_kick_per_shot: 0.8,
    crosshair_kick_decay: 5.0,
    crosshair_kick_max: 2.3,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: Some(0.35),

    fire_sound_asset_key: "ar_shot",
    icon_asset_key: "ar_icon",
    icon_width: 100.0,
    icon_height: 45.0,
    icon_flip_x: false,

    viewmodel_model: Some("m16_model"),
    viewmodel_asset_key: None,
    viewmodel_scale: 0.003,
    viewmodel_position: (0.22, 0.08, -0.31),
    viewmodel_rotation: (0.05, -1.75, 0.0),
    viewmodel_muzzle_offset: (-1.00, 0.02, -0.25),
    viewmodel_hands_scale: 1.93,
    viewmodel_hands_offset: (0.16, -0.16, -0.28),
    viewmodel_hands_rotation: (-0.15, 0.45, -0.05),
    viewmodel_pair_offset_pos: (0.0, 0.0, 0.0),
    viewmodel_pair_offset_rot: (0.0, 0.0, 0.0),
    viewmodel_ads_pos_offset: (0.08, 0.03, 0.0),
    viewmodel_ads_rot_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_pos_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_rot_offset: (0.0, 0.0, 0.0),

    falloff_start: 25.0,
    falloff_end: 70.0,
    falloff_min_mult: 0.6,

    recoil_pattern: &[
        (0.010, 0.0),
        (0.011, 0.0),
        (0.012, 0.1),
        (0.012, 0.2),
        (0.011, 0.3),
        (0.010, 0.1),
        (0.010, -0.2),
        (0.009, -0.4),
        (0.008, -0.3),
        (0.007, -0.1),
    ],

    pattern_randomness: 0.1,
    randomness_growth: 0.06,
    pattern_reset_time: 0.5,
};

// TODO: SCAR — heavy rifle, slower fire rate but hits harder than M4
pub const SCAR: WeaponStats = WeaponStats {
    name: "SCAR",
    viewmodel_model: Some("scar_model"),
    viewmodel_asset_key: None,
    ..M16_STATS
};

// TODO: AS VAL — suppressed close-range rifle, stealth + fast TTK but short range
pub const AS_VAL: WeaponStats = WeaponStats {
    name: "AS VAL",
    viewmodel_model: Some("as_val_model"),
    viewmodel_asset_key: None,
    ..M16_STATS
};

// ============================================================================
//  SNIPERS
// ============================================================================

/// Bolt Action — classic sniper, one shot one kill
pub const BOLT_ACTION: WeaponStats = WeaponStats {
    name: "Bolt Action",
    fire_mode: FireMode::BoltAction,

    damage: 105.0,
    headshot_mult: 2.5,
    pellet_count: 1,
    pellet_spread: None,
    fire_rate: 0.8,
    magazine_size: 1,
    max_reserve: 8,
    reload_time: 3.7,

    recoil_vertical: 0.08,
    recoil_horizontal: 0.01,
    recoil_recovery: 2.0,

    hip_spread: 0.1,
    ads_spread: 0.001,
    spread_per_shot: 0.06,
    spread_recovery: 0.8,
    move_spread_mult: 2.0,
    sprint_spread_mult: 3.0,
    crouch_spread_mult: 0.5,

    ads_zoom_fov: 15.0,
    ads_time: 0.05,
    ads_sensitivity: Some(0.2),

    bullet_speed: 350.0,
    bullet_range: 120.0,
    tracer_length: 8.0,
    tracer_color: [180, 200, 255, 255],
    tracer_thickness: 0.03,
    tracer_lifetime: 3.5,

    crosshair_kick_per_shot: 0.0,
    crosshair_kick_decay: 0.0,
    crosshair_kick_max: 0.0,

    crosshair_style: CrosshairStyle::Scope,
    fsa_time: None,

    fire_sound_asset_key: "sniper_shot",
    icon_asset_key: "sniper_icon",
    icon_width: 110.0,
    icon_height: 40.0,
    icon_flip_x: false,

    viewmodel_model: Some("bolt_action_model"),
    viewmodel_asset_key: None,
    viewmodel_scale: VM_BASE_SCALE,
    viewmodel_position: VM_BASE_POS,
    viewmodel_rotation: VM_BASE_ROT,
    viewmodel_muzzle_offset: (-1.20, 0.02, -0.28),
    viewmodel_hands_scale: 1.93,
    viewmodel_hands_offset: (0.0, 0.0, 0.0),
    viewmodel_hands_rotation: (-0.15, 0.65, -0.05),
    viewmodel_pair_offset_pos: (0.0, 0.0, 0.0),
    viewmodel_pair_offset_rot: (0.0, 0.0, 0.0),
    viewmodel_ads_pos_offset: (0.08, 0.03, 0.0),
    viewmodel_ads_rot_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_pos_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_rot_offset: (0.0, 0.0, 0.0),

    falloff_start: 1000.0,
    falloff_end: 10000.0,
    falloff_min_mult: 1.0,

    recoil_pattern: &[],
    pattern_randomness: 0.0,
    randomness_growth: 0.0,
    pattern_reset_time: 0.0,
};

// TODO: Hunting Rifle — fill in unique sniper stats
pub const HUNTING_RIFLE: WeaponStats = WeaponStats {
    name: "Hunting Rifle",
    viewmodel_model: Some("hunting_rifle_model"),
    viewmodel_asset_key: None,
    ..BOLT_ACTION
};

// TODO: Semi-Auto Sniper — fill in unique sniper stats
pub const SEMI_AUTO_SNIPER: WeaponStats = WeaponStats {
    name: "Semi-Auto Sniper",
    viewmodel_model: Some("semi_auto_sniper_model"),
    viewmodel_asset_key: None,
    ..BOLT_ACTION
};

// ============================================================================
//  OTHER
// ============================================================================

/// Minigun — sustained fire monster, spin-up mechanic + suppression
pub const MINIGUN: WeaponStats = WeaponStats {
    name: "Minigun",
    fire_mode: FireMode::FullAuto,

    damage: 14.0,           // low per bullet but insane volume
    headshot_mult: 1.3,     // not rewarding headshots much — it's a spray weapon
    pellet_count: 1,
    pellet_spread: None,

    fire_rate: 20.0,        // absurd fire rate
    magazine_size: 100,
    max_reserve: 200,
    reload_time: 5.0,       // long reload — you pay for the mag size

    recoil_vertical: 0.005, // low per shot but adds up fast at 20 rps
    recoil_horizontal: 0.008,
    recoil_recovery: 3.0,

    hip_spread: 0.06,       // terrible accuracy from hip
    ads_spread: 0.03,       // still not great even aimed
    spread_per_shot: 0.008, // bloom grows fast with the fire rate
    spread_recovery: 1.5,
    move_spread_mult: 2.0,  // you're basically a turret — moving wrecks accuracy
    sprint_spread_mult: 3.0,
    crouch_spread_mult: 0.6, // crouching helps a lot

    ads_zoom_fov: 70.0,
    ads_time: 0.4,          // slow to aim — heavy weapon
    ads_sensitivity: None,

    bullet_speed: 200.0,
    bullet_range: 50.0,     // medium range — not a sniper
    tracer_length: 2.0,
    tracer_color: [255, 160, 60, 255],
    tracer_thickness: 0.012,
    tracer_lifetime: 0.8,

    crosshair_kick_per_shot: 0.3,
    crosshair_kick_decay: 4.0,
    crosshair_kick_max: 3.5,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: None,          // minigun never reaches FSA

    fire_sound_asset_key: "ar_shot", // placeholder until we get a proper minigun sound
    icon_asset_key: "ar_icon",       // placeholder
    icon_width: 110.0,
    icon_height: 50.0,
    icon_flip_x: false,

    viewmodel_model: Some("minigun_model"),
    viewmodel_asset_key: None,
    viewmodel_scale: VM_BASE_SCALE,
    viewmodel_position: VM_BASE_POS,
    viewmodel_rotation: VM_BASE_ROT,
    viewmodel_muzzle_offset: (-1.00, 0.02, -0.25),
    viewmodel_hands_scale: 1.93,
    viewmodel_hands_offset: (0.0, 0.0, 0.0),
    viewmodel_hands_rotation: (-0.15, 0.65, -0.05),
    viewmodel_pair_offset_pos: (0.0, 0.0, 0.0),
    viewmodel_pair_offset_rot: (0.0, 0.0, 0.0),
    viewmodel_ads_pos_offset: (0.08, 0.03, 0.0),
    viewmodel_ads_rot_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_pos_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_rot_offset: (0.0, 0.0, 0.0),

    falloff_start: 15.0,
    falloff_end: 45.0,
    falloff_min_mult: 0.3,

    recoil_pattern: &[
        (0.005, 0.0),
        (0.005, 0.02),
        (0.005, -0.02),
        (0.004, 0.03),
        (0.004, -0.03),
        (0.004, 0.01),
    ],
    pattern_randomness: 0.4,  // very random — you're spraying
    randomness_growth: 0.02,
    pattern_reset_time: 0.3,
};

// TODO: Grenade Launcher — area denial, splash damage zoning tool
pub const GRENADE_LAUNCHER: WeaponStats = WeaponStats {
    name: "Grenade Launcher",
    viewmodel_model: Some("grenade_launcher_model"),
    viewmodel_asset_key: None,
    ..MINIGUN
};

// TODO: RPG — high burst damage explosive, anti-group nuke weapon
pub const RPG: WeaponStats = WeaponStats {
    name: "RPG",
    viewmodel_model: Some("rpg_model"),
    viewmodel_asset_key: None,
    ..MINIGUN
};
