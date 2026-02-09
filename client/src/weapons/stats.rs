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
    // Sniper — hip-fire is a crosshair, ADS is a full scope overlay
    Scope,
}

// fire mode determines input behaviour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FireMode {
    SemiAuto,    // one shot per click
    FullAuto,    // hold to keep firing
    BoltAction,  // delay after each shot
}

// all the numbers that define how a weapon behaves and feels.
pub struct WeaponStats {
    pub name: &'static str,
    pub fire_mode: FireMode,

    // --- Damage 
    pub damage: f32,
    pub headshot_mult: f32,
    pub pellet_count: u32, // 1 for most guns, >1 for shotgun

    // --- Fire rate & ammo 
    pub fire_rate: f32,      // rounds per second
    pub magazine_size: u32,
    pub max_reserve: u32,    // total spare ammo
    pub reload_time: f32,    // seconds

    // --- Recoil
    pub recoil_vertical: f32,   // pitch kick per shot (radians)
    pub recoil_horizontal: f32, // max random yaw kick per shot (radians)
    pub recoil_recovery: f32,   // how fast recoil settles back (rad/s)

    // --- Spread / bloom 
    pub hip_spread: f32,        // base spread at hip (radians)
    pub ads_spread: f32,        // base spread when aimed (radians)
    pub spread_per_shot: f32,   // bloom added per shot (radians)
    pub spread_recovery: f32,   // bloom decay rate when not firing (rad/s)
    pub move_spread_mult: f32,  // multiplier applied to spread when moving
    pub sprint_spread_mult: f32,// multiplier when sprinting (stacks with move)
    pub crouch_spread_mult: f32,// multiplier when crouching (<1 = tighter)

    // --- ADS (aim down sights)
    pub ads_zoom_fov: f32,            // target FOV when ADS (lower = more zoom)
    pub ads_time: f32,                // seconds to reach full ADS
    pub ads_sensitivity: Option<f32>, // custom sensitivity multiplier when ADS (None = use default), will only use this for sniper scope  

    // --- Bullet visual
    pub bullet_speed: f32,   // tracer speed (units/s), purely visual
    pub bullet_range: f32,   // max hitscan distance (units)

    // --- Crosshair appearance 
    pub crosshair_style: CrosshairStyle,
    pub crosshair_hip_key: &'static str,  // asset key for hip-fire crosshair
    pub crosshair_ads_key: &'static str,  // asset key for ADS crosshair
    pub crosshair_base_size: f32,         // hip-fire size as % of screen height (bloom scales this up)
    pub crosshair_ads_size: f32,          // ADS size as % of screen height (fixed, no bloom)
    // --- HUD icon
    pub icon_asset_key: &'static str, // key used to look up texture in assets HashMap
    pub icon_width: f32,              // display width in pixels (tweak per weapon)
    pub icon_height: f32,             // display height in pixels
    pub icon_flip_x: bool,            // true if the source image faces left

}



// --- Weapon definitions 

pub const PISTOL: WeaponStats = WeaponStats {
    name: "Pistol",
    fire_mode: FireMode::SemiAuto,

    damage: 22.0,
    headshot_mult: 2.0,
    pellet_count: 1,

    fire_rate: 5.0,       // fast semi-auto sidearm
    magazine_size: 15,
    max_reserve: 60,
    reload_time: 1.3,     // quick reload

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
    ads_time: 0.15,       // fastest ADS — it's a pistol
    ads_sensitivity: None, 

    bullet_speed: 200.0,
    bullet_range: 40.0,

    crosshair_style: CrosshairStyle::Standard,
    crosshair_hip_key: "xhair_pistol_hip",   // crosshair062
    crosshair_ads_key: "xhair_pistol_ads",   // crosshair063
    crosshair_base_size: 4.0,                // % of screen height — small, precise sidearm
    crosshair_ads_size: 3.3,                 // % of screen height — tighter when aiming

    icon_asset_key: "weapon_pistol",
    icon_width: 80.0,
    icon_height: 50.0,
    icon_flip_x: false,
};

pub const SMG: WeaponStats = WeaponStats {
    name: "SMG",
    fire_mode: FireMode::FullAuto,

    damage: 20.0,
    headshot_mult: 1.5,
    pellet_count: 1,

    fire_rate: 11.0,      // high fire rate spray weapon
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
    move_spread_mult: 1.1,    // good run-and-gun weapon
    sprint_spread_mult: 1.4,
    crouch_spread_mult: 0.85,

    ads_zoom_fov: 70.0,
    ads_time: 0.18,
    ads_sensitivity: None,

    bullet_speed: 180.0,
    bullet_range: 45.0,

    crosshair_style: CrosshairStyle::Standard,
    crosshair_hip_key: "xhair_smg_hip",     // crosshair179
    crosshair_ads_key: "xhair_smg_ads",     // crosshair180
    crosshair_base_size: 5.2,               // % of screen height — wider for spray weapon
    crosshair_ads_size: 4.0,                // % of screen height — tighter when aiming

    icon_asset_key: "weapon_smg",
    icon_width: 90.0,
    icon_height: 45.0,
    icon_flip_x: true,
};

pub const SHOTGUN: WeaponStats = WeaponStats {
    name: "Shotgun",
    fire_mode: FireMode::SemiAuto,

    damage: 12.0,          // per pellet — 8 pellets = 120 max (devastating up close)
    headshot_mult: 1.5,    
    pellet_count: 8,

    fire_rate: 1.0,        // slow pump action
    magazine_size: 7,
    max_reserve: 28,
    reload_time: 2.2,

    recoil_vertical: 0.07,
    recoil_horizontal: 0.025,
    recoil_recovery: 2.5,

    hip_spread: 0.06,      // decent hip-fire (shotguns don't need ADS as much)
    ads_spread: 0.035,
    spread_per_shot: 0.04,
    spread_recovery: 0.8,
    move_spread_mult: 1.1,
    sprint_spread_mult: 1.3,
    crouch_spread_mult: 0.85,

    ads_zoom_fov: 75.0,
    ads_time: 0.25,
    ads_sensitivity: None, // no sensitivity change when aiming — shotgun ADS is just for tighter spread, not zoom

    bullet_speed: 150.0,
    bullet_range: 20.0,    // short range but not useless

    crosshair_style: CrosshairStyle::Standard,
    crosshair_hip_key: "xhair_shotgun_hip", // crosshair039
    crosshair_ads_key: "xhair_shotgun_ads", // crosshair037
    crosshair_base_size: 4.7,               // % of screen height — widest, pellet spread cone
    crosshair_ads_size: 5.2,                // % of screen height — tighter when aiming

    icon_asset_key: "weapon_shotgun",
    icon_width: 100.0,
    icon_height: 50.0,
    icon_flip_x: true,
};

pub const ASSAULT_RIFLE: WeaponStats = WeaponStats {
    name: "Assault Rifle",
    fire_mode: FireMode::FullAuto,

    damage: 27.0,          // 4-shot body kill (108), 3-shot headshot (27*1.8=48.6)
    headshot_mult: 1.8,
    pellet_count: 1,

    fire_rate: 7.0,        // slower than SMG but hits harder
    magazine_size: 30,
    max_reserve: 120,
    reload_time: 2.5,      // heavier weapon = slower reload

    recoil_vertical: 0.028,
    recoil_horizontal: 0.008,
    recoil_recovery: 3.5,

    hip_spread: 0.03,      // worse hipfire than SMG (heavier weapon)
    ads_spread: 0.006,     // but better ADS accuracy
    spread_per_shot: 0.02,
    spread_recovery: 1.8,
    move_spread_mult: 1.3,
    sprint_spread_mult: 1.7,
    crouch_spread_mult: 0.75,

    ads_zoom_fov: 55.0,
    ads_time: 0.25,
    ads_sensitivity: None,

    bullet_speed: 250.0,
    bullet_range: 70.0,    // best range of the auto weapons

    crosshair_style: CrosshairStyle::Standard,
    crosshair_hip_key: "xhair_ar_hip",      // crosshair010
    crosshair_ads_key: "xhair_ar_ads",      // crosshair011
    crosshair_base_size: 4.4,               // % of screen height — medium, balanced weapon
    crosshair_ads_size: 3.5,                // % of screen height — precise when aiming

    icon_asset_key: "weapon_ar",
    icon_width: 100.0,
    icon_height: 45.0,
    icon_flip_x: false,
};

pub const SNIPER: WeaponStats = WeaponStats {
    name: "Sniper",
    fire_mode: FireMode::BoltAction,

    damage: 105.0,         // 1-shot body kill (105), 1-shot headshot (105*2.5=262)
    headshot_mult: 2.5,
    pellet_count: 1,

    fire_rate: 0.8,        // bolt action — very slow
    magazine_size: 5,
    max_reserve: 25,
    reload_time: 3.2,      // was 4.0 — less punishing for 5 rounds

    recoil_vertical: 0.08,
    recoil_horizontal: 0.01,
    recoil_recovery: 2.0,

    hip_spread: 0.10,       // terrible from hip
    ads_spread: 0.001,      // pinpoint when scoped
    spread_per_shot: 0.06,
    spread_recovery: 0.8,
    move_spread_mult: 2.0,  // heavy penalty
    sprint_spread_mult: 3.0,
    crouch_spread_mult: 0.5, // crouching + scoped = laser

    ads_zoom_fov: 15.0,     // high zoom scope
    ads_time: 0.05,
    ads_sensitivity: Some(0.2), // much lower sensitivity when scoped for precision aiming

    bullet_speed: 350.0,
    bullet_range: 120.0,    // longest range in the game

    crosshair_style: CrosshairStyle::Scope,
    crosshair_hip_key: "xhair_sniper_hip",  // no crosshair when hip-firing a sniper
    crosshair_ads_key: "xhair_sniper_ads",  
    crosshair_base_size: 0.0,               // % — sniper has no hip crosshair (Scope style)
    crosshair_ads_size: 0.0,                // % — scope uses procedural reticle, not texture

    icon_asset_key: "weapon_sniper",
    icon_width: 110.0,
    icon_height: 40.0,
    icon_flip_x: false,
};
