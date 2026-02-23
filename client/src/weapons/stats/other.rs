use super::{CrosshairStyle, FireMode, WeaponStats};

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
    reload_time: 2.23,       // matches minigun_animated.glb reload segment

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
    bullet_hole_size: 0.4,
    bullet_hole_lifetime: 5.0,
    bullet_hole_color: [200, 200, 200, 255],

    crosshair_kick_per_shot: 0.3,
    crosshair_kick_decay: 4.0,
    crosshair_kick_max: 3.5,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: None,          // minigun never reaches FSA
    fsa_suppresses_kick: false,

    fire_sound_asset_key: "ar_shot", // placeholder until we get a proper minigun sound
    icon_asset_key: "ar_icon",       // placeholder
    icon_width: 110.0,
    icon_height: 50.0,
    icon_flip_x: false,

    // Minigun model includes hands — rendered as a single animated model
    // like the sawnoff/knife. No separate hands drawn.
    viewmodel_model: Some("fp_hands_minigun"),
    viewmodel_asset_key: None,
    viewmodel_scale: 0.0300,
    viewmodel_position: (0.349, -0.446, -0.460),
    viewmodel_rotation: (-0.15, 0.0, 0.0),
    viewmodel_muzzle_offset: (0.0, 0.0, 0.0),
    viewmodel_hands_scale: 1.0,
    viewmodel_hands_offset: (0.0, 0.0, 0.0),
    viewmodel_hands_rotation: (0.0, 0.0, 0.0),
    viewmodel_pair_offset_pos: (0.0, 0.0, 0.0),
    viewmodel_pair_offset_rot: (0.0, 0.0, 0.0),
    viewmodel_ads_pos_offset: (0.0, 0.0, 0.0),
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
