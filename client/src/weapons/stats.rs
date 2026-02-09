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
    pub ads_zoom_fov: f32,   // target FOV when ADS (lower = more zoom)
    pub ads_time: f32,       // seconds to reach full ADS

    // --- Bullet visual
    pub bullet_speed: f32,   // tracer speed (units/s), purely visual
    pub bullet_range: f32,   // max hitscan distance (units)

    // --- Crosshair appearance 
    pub crosshair_style: CrosshairStyle,
    pub crosshair_hip_key: &'static str,  // asset key for hip-fire crosshair
    pub crosshair_ads_key: &'static str,  // asset key for ADS crosshair
    pub crosshair_base_size: f32,         // base draw size in pixels (at min spread)
    // --- HUD icon
    pub icon_asset_key: &'static str, // key used to look up texture in assets HashMap
    pub icon_width: f32,              // display width in pixels (tweak per weapon)
    pub icon_height: f32,             // display height in pixels
    pub icon_flip_x: bool,            // true if the source image faces left

    // --- Viewmodel (FPS Weapon sprite)
    pub viewmodel_frame_keys: &'static [&'static str], // asset keys for animation frames (idle, fire, reload)
    pub viewmodel_frame_time: f32, // time per frame in seconds
} 



// --- Weapon definitions 

pub const PISTOL: WeaponStats = WeaponStats {
    name: "Pistol",
    fire_mode: FireMode::SemiAuto,

    damage: 25.0,
    headshot_mult: 2.0,
    pellet_count: 1,

    fire_rate: 4.0,       // 4 shots/sec
    magazine_size: 12,
    max_reserve: 48,
    reload_time: 1.5,

    recoil_vertical: 0.02,
    recoil_horizontal: 0.005,
    recoil_recovery: 4.0,

    hip_spread: 0.015,
    ads_spread: 0.005,
    spread_per_shot: 0.025,
    spread_recovery: 1.5,
    move_spread_mult: 1.4,
    sprint_spread_mult: 1.8,
    crouch_spread_mult: 0.8,

    ads_zoom_fov: 65.0,
    ads_time: 0.2,

    bullet_speed: 200.0,
    bullet_range: 50.0,

    crosshair_style: CrosshairStyle::Standard,
    crosshair_hip_key: "xhair_pistol_hip",   // crosshair062
    crosshair_ads_key: "xhair_pistol_ads",   // crosshair063
    crosshair_base_size: 32.0,

    icon_asset_key: "weapon_pistol",
    icon_width: 80.0,
    icon_height: 50.0,
    icon_flip_x: false,

     viewmodel_frame_keys: &[
        "pistol_frame_1",
        "pistol_frame_2",
        "pistol_frame_3",
        "pistol_frame_4",
        "pistol_frame_5",
    ],
    viewmodel_frame_time: 0.06,
};

pub const SMG: WeaponStats = WeaponStats {
    name: "SMG",
    fire_mode: FireMode::FullAuto,

    damage: 18.0,
    headshot_mult: 1.5,
    pellet_count: 1,

    fire_rate: 12.0,      // fast fire
    magazine_size: 30,
    max_reserve: 120,
    reload_time: 2.0,

    recoil_vertical: 0.015,
    recoil_horizontal: 0.012,
    recoil_recovery: 5.0,

    hip_spread: 0.03,
    ads_spread: 0.015,
    spread_per_shot: 0.018,
    spread_recovery: 2.0,
    move_spread_mult: 1.2,
    sprint_spread_mult: 1.6,
    crouch_spread_mult: 0.85,

    ads_zoom_fov: 70.0,
    ads_time: 0.18,

    bullet_speed: 180.0,
    bullet_range: 35.0,

    crosshair_style: CrosshairStyle::Standard,
    crosshair_hip_key: "xhair_smg_hip",     // crosshair179
    crosshair_ads_key: "xhair_smg_ads",     // crosshair180
    crosshair_base_size: 36.0,

    icon_asset_key: "weapon_smg",
    icon_width: 90.0,
    icon_height: 45.0,
    icon_flip_x: true,
    viewmodel_frame_keys: &[],
    viewmodel_frame_time: 0.0,
};

pub const SHOTGUN: WeaponStats = WeaponStats {
    name: "Shotgun",
    fire_mode: FireMode::SemiAuto,

    damage: 12.0,          // per pellet — 8 pellets = 96 max
    headshot_mult: 1.5,
    pellet_count: 8,

    fire_rate: 1.2,        // slow pump
    magazine_size: 6,
    max_reserve: 24,
    reload_time: 2.8,      // per-shell in reality, simplified for now

    recoil_vertical: 0.06,
    recoil_horizontal: 0.02,
    recoil_recovery: 3.0,

    hip_spread: 0.07,
    ads_spread: 0.045,
    spread_per_shot: 0.035,
    spread_recovery: 1.0,
    move_spread_mult: 1.1,
    sprint_spread_mult: 1.3,
    crouch_spread_mult: 0.9,

    ads_zoom_fov: 75.0,
    ads_time: 0.25,

    bullet_speed: 150.0,
    bullet_range: 15.0,

    crosshair_style: CrosshairStyle::Standard,
    crosshair_hip_key: "xhair_shotgun_hip", // crosshair039
    crosshair_ads_key: "xhair_shotgun_ads", // crosshair037
    crosshair_base_size: 48.0,

    icon_asset_key: "weapon_shotgun",
    icon_width: 100.0,
    icon_height: 50.0,
    icon_flip_x: true,
    viewmodel_frame_keys: &[],
    viewmodel_frame_time: 0.0,
};

pub const ASSAULT_RIFLE: WeaponStats = WeaponStats {
    name: "Assault Rifle",
    fire_mode: FireMode::FullAuto,

    damage: 22.0,
    headshot_mult: 1.8,
    pellet_count: 1,

    fire_rate: 8.0,
    magazine_size: 25,
    max_reserve: 100,
    reload_time: 2.2,

    recoil_vertical: 0.025,
    recoil_horizontal: 0.008,
    recoil_recovery: 3.5,

    hip_spread: 0.025,
    ads_spread: 0.008,
    spread_per_shot: 0.022,
    spread_recovery: 1.8,
    move_spread_mult: 1.3,
    sprint_spread_mult: 1.7,
    crouch_spread_mult: 0.8,

    ads_zoom_fov: 55.0,
    ads_time: 0.25,

    bullet_speed: 220.0,
    bullet_range: 60.0,

    crosshair_style: CrosshairStyle::Standard,
    crosshair_hip_key: "xhair_ar_hip",      // crosshair010
    crosshair_ads_key: "xhair_ar_ads",      // crosshair011
    crosshair_base_size: 34.0,

    icon_asset_key: "weapon_ar",
    icon_width: 100.0,
    icon_height: 45.0,
    icon_flip_x: false,
    viewmodel_frame_keys: &[],
    viewmodel_frame_time: 0.0,
};

pub const SNIPER: WeaponStats = WeaponStats {
    name: "Sniper",
    fire_mode: FireMode::BoltAction,

    damage: 90.0, 
    headshot_mult: 2.5,
    pellet_count: 1,

    fire_rate: 0.8,        // bolt action — very slow
    magazine_size: 5,
    max_reserve: 20,
    reload_time: 4.0,

    recoil_vertical: 0.08,
    recoil_horizontal: 0.01,
    recoil_recovery: 2.0,

    hip_spread: 0.08,       // terrible from hip
    ads_spread: 0.002,      // pinpoint when scoped
    spread_per_shot: 0.05,
    spread_recovery: 0.8,
    move_spread_mult: 2.0,  // heavy penalty
    sprint_spread_mult: 3.0,
    crouch_spread_mult: 0.6,

    ads_zoom_fov: 15.0,     // high zoom scope
    ads_time: 0.35,

    bullet_speed: 300.0,
    bullet_range: 100.0,

    crosshair_style: CrosshairStyle::Scope,
    crosshair_hip_key: "xhair_sniper_hip",  // crosshair048
    crosshair_ads_key: "xhair_sniper_ads",  // crosshair049
    crosshair_base_size: 28.0,

    icon_asset_key: "weapon_sniper",
    icon_width: 110.0,
    icon_height: 40.0,
    icon_flip_x: false,

    viewmodel_frame_keys: &[],
    viewmodel_frame_time: 0.0,
};
