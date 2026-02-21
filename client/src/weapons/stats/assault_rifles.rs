use super::{ CrosshairStyle, FireMode, WeaponStats };

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
    bullet_hole_size: 0.25,
    bullet_hole_lifetime: 5.0,
    bullet_hole_color: [200, 200, 200, 255],

    crosshair_kick_per_shot: 0.8,
    crosshair_kick_decay: 5.0,
    crosshair_kick_max: 2.3,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: Some(0.35),
    fsa_suppresses_kick: false, 
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
    viewmodel_muzzle_offset: (-1.0, 0.02, -0.25),
    viewmodel_hands_scale: 1.93,
    viewmodel_hands_offset: (0.16, -0.16, -0.28),
    viewmodel_hands_rotation: (-0.15, 0.45, -0.05),
    viewmodel_pair_offset_pos: (0.0, 0.0, 0.0),
    viewmodel_pair_offset_rot: (0.0, 0.0, 0.0),
    viewmodel_ads_pos_offset: (0.132, 0.025, -0.151),
    viewmodel_ads_rot_offset: (0.015, 0.195, -0.02),
    viewmodel_ads_hands_pos_offset: (0.0, 0.0, -0.704),
    viewmodel_ads_hands_rot_offset: (0.0, 0.0, 0.0),

    falloff_start: 25.0,
    falloff_end: 70.0,
    falloff_min_mult: 0.6,

    recoil_pattern: &[
        (0.01, 0.0),
        (0.011, 0.0),
        (0.012, 0.1),
        (0.012, 0.2),
        (0.011, 0.3),
        (0.01, 0.1),
        (0.01, -0.2),
        (0.009, -0.4),
        (0.008, -0.3),
        (0.007, -0.1),
    ],
    pattern_randomness: 0.1,
    randomness_growth: 0.06,
    pattern_reset_time: 0.5,
};

/// M4A1 — balanced automatic rifle, lowest recoil baseline gun
pub const M4A1: WeaponStats = WeaponStats {
    name: "M4A1",
    damage: 25.0,
    headshot_mult: 1.8,
    fire_rate: 8.0,
    magazine_size: 30,
    max_reserve: 90,
    reload_time: 2.2,

    recoil_vertical: 0.008,
    recoil_horizontal: 0.005,
    recoil_recovery: 8.0,

    hip_spread: 0.025,
    ads_spread: 0.008,
    spread_per_shot: 0.012,
    spread_recovery: 5.5,
    move_spread_mult: 1.3,
    sprint_spread_mult: 1.8,
    crouch_spread_mult: 0.75,

    ads_zoom_fov: 50.0,
    ads_time: 0.22,

    bullet_speed: 250.0,
    bullet_range: 80.0,
    tracer_length: 3.5,
    tracer_color: [255, 230, 150, 255],
    tracer_thickness: 0.015,
    tracer_lifetime: 1.5,

    crosshair_kick_per_shot: 0.6,
    crosshair_kick_decay: 6.0,
    crosshair_kick_max: 2.0,

    fsa_time: Some(0.3),

    fire_sound_asset_key: "ar_shot",
    icon_asset_key: "ar_icon",

    viewmodel_model: Some("m4a1_model"),
    viewmodel_asset_key: None,

    falloff_start: 30.0,
    falloff_end: 75.0,
    falloff_min_mult: 0.55,

    // Clean, easy-to-learn pattern — slight rightward drift then centers
    recoil_pattern: &[
        (0.008, 0.0),
        (0.008, 0.0),
        (0.009, 0.05),
        (0.009, 0.1),
        (0.010, 0.15),
        (0.010, 0.1),
        (0.009, -0.05),
        (0.008, -0.15),
        (0.008, -0.1),
        (0.007, 0.0),
    ],
    pattern_randomness: 0.08,
    randomness_growth: 0.04,
    pattern_reset_time: 0.4,

    ..M16_STATS
};

/// AK-47 — strongest per-shot AR, hardest to control. High risk high reward.
pub const AK: WeaponStats = WeaponStats {
    name: "AK-47",
    damage: 34.0,
    headshot_mult: 1.8,
    fire_rate: 6.0,
    magazine_size: 30,
    max_reserve: 90,
    reload_time: 2.5,

    recoil_vertical: 0.025,
    recoil_horizontal: 0.018,
    recoil_recovery: 5.0,

    hip_spread: 0.04,
    ads_spread: 0.012,
    spread_per_shot: 0.018,
    spread_recovery: 4.0,
    move_spread_mult: 1.5,
    sprint_spread_mult: 2.2,
    crouch_spread_mult: 0.7,

    ads_zoom_fov: 50.0,
    ads_time: 0.28,

    bullet_speed: 230.0,
    bullet_range: 75.0,
    tracer_length: 3.5,
    tracer_color: [255, 210, 100, 255],
    tracer_thickness: 0.018,
    tracer_lifetime: 1.5,

    crosshair_kick_per_shot: 1.2,
    crosshair_kick_decay: 4.0,
    crosshair_kick_max: 3.0,

    fsa_time: Some(0.4),
    fsa_suppresses_kick: false, // AK always kicks hard

    fire_sound_asset_key: "ar_shot",
    icon_asset_key: "ar_icon",

    viewmodel_model: Some("ak_model"),
    viewmodel_asset_key: None,

    falloff_start: 25.0,
    falloff_end: 70.0,
    falloff_min_mult: 0.6,

    // Wild pattern — heavy vertical climb with unpredictable horizontal sway
    recoil_pattern: &[
        (0.020, 0.0),
        (0.022, 0.0),
        (0.025, 0.15),
        (0.025, 0.3),
        (0.024, 0.2),
        (0.022, -0.1),
        (0.020, -0.35),
        (0.018, -0.5),
        (0.016, -0.3),
        (0.015, 0.1),
    ],
    pattern_randomness: 0.2,
    randomness_growth: 0.08,
    pattern_reset_time: 0.6,

    ..M16_STATS
};

/// FAMAS — 3-round burst, rewards precise timing and aim
pub const FAMAS: WeaponStats = WeaponStats {
    name: "FAMAS",
    fire_mode: FireMode::Burst(3),
    damage: 24.0,
    headshot_mult: 1.8,
    fire_rate: 8.0, // speed within burst — extra pause after last shot added in fire logic
    magazine_size: 30,
    max_reserve: 90,
    reload_time: 2.3,

    recoil_vertical: 0.012,
    recoil_horizontal: 0.006,
    recoil_recovery: 10.0, // fast reset between bursts

    hip_spread: 0.032,
    ads_spread: 0.006,
    spread_per_shot: 0.005, // low bloom within burst
    spread_recovery: 8.0,  // resets fast between bursts
    move_spread_mult: 1.4,
    sprint_spread_mult: 2.0,
    crouch_spread_mult: 0.75,

    ads_zoom_fov: 50.0,
    ads_time: 0.23,

    bullet_speed: 240.0,
    bullet_range: 70.0,
    tracer_length: 3.5,
    tracer_color: [255, 230, 150, 255],
    tracer_thickness: 0.015,
    tracer_lifetime: 1.5,

    crosshair_kick_per_shot: 0.7,
    crosshair_kick_decay: 7.0,
    crosshair_kick_max: 2.0,

    fsa_time: Some(0.25), // fast FSA between bursts
    fsa_suppresses_kick: false,

    fire_sound_asset_key: "ar_shot",
    icon_asset_key: "ar_icon",

    viewmodel_model: Some("famas_model"),
    viewmodel_asset_key: None,

    falloff_start: 25.0,
    falloff_end: 65.0,
    falloff_min_mult: 0.55,

    // Tight 3-shot climb — predictable within each burst
    recoil_pattern: &[
        (0.010, 0.0),
        (0.012, 0.05),
        (0.014, 0.1),
        (0.010, -0.05),
        (0.012, -0.1),
        (0.014, 0.0),
        (0.010, 0.1),
        (0.012, -0.05),
        (0.014, -0.1),
        (0.010, 0.0),
    ],
    pattern_randomness: 0.06,
    randomness_growth: 0.03,
    pattern_reset_time: 0.35,

    ..M16_STATS
};

/// SCAR-H — heavy rifle, slower fire rate but hits harder than M4
pub const SCAR: WeaponStats = WeaponStats {
    name: "SCAR-H",
    damage: 30.0,
    headshot_mult: 1.8,
    fire_rate: 6.5,
    magazine_size: 25,
    max_reserve: 75,
    reload_time: 2.4,

    recoil_vertical: 0.018,
    recoil_horizontal: 0.01,
    recoil_recovery: 6.5,

    hip_spread: 0.035,
    ads_spread: 0.009,
    spread_per_shot: 0.014,
    spread_recovery: 5.0,
    move_spread_mult: 1.4,
    sprint_spread_mult: 2.0,
    crouch_spread_mult: 0.75,

    ads_zoom_fov: 50.0,
    ads_time: 0.27,

    bullet_speed: 240.0,
    bullet_range: 80.0,
    tracer_length: 3.5,
    tracer_color: [255, 220, 130, 255],
    tracer_thickness: 0.016,
    tracer_lifetime: 1.5,

    crosshair_kick_per_shot: 1.0,
    crosshair_kick_decay: 5.0,
    crosshair_kick_max: 2.5,

    fsa_time: Some(0.35),

    fire_sound_asset_key: "ar_shot",
    icon_asset_key: "ar_icon",

    viewmodel_model: Some("scar_model"),
    viewmodel_asset_key: None,

    falloff_start: 30.0,
    falloff_end: 80.0,
    falloff_min_mult: 0.6,

    // Moderate controlled climb — heavier than M4 but more predictable than AK
    recoil_pattern: &[
        (0.015, 0.0),
        (0.016, 0.0),
        (0.018, 0.1),
        (0.018, 0.2),
        (0.017, 0.15),
        (0.016, -0.05),
        (0.015, -0.2),
        (0.014, -0.25),
        (0.013, -0.15),
        (0.012, 0.0),
    ],
    pattern_randomness: 0.12,
    randomness_growth: 0.05,
    pattern_reset_time: 0.5,

    ..M16_STATS
};

/// AS VAL — suppressed close-range rifle, stealth + fast TTK but short range
pub const AS_VAL: WeaponStats = WeaponStats {
    name: "AS VAL",
    damage: 22.0,
    headshot_mult: 1.8,
    fire_rate: 10.0,
    magazine_size: 20,
    max_reserve: 80,
    reload_time: 2.0,

    recoil_vertical: 0.009,
    recoil_horizontal: 0.006,
    recoil_recovery: 7.0,

    hip_spread: 0.02,
    ads_spread: 0.005,
    spread_per_shot: 0.01,
    spread_recovery: 6.0,
    move_spread_mult: 1.2,
    sprint_spread_mult: 1.6,
    crouch_spread_mult: 0.8,

    ads_zoom_fov: 55.0,
    ads_time: 0.2,

    // Suppressed — invisible tracers, no muzzle flash
    bullet_speed: 200.0,
    bullet_range: 40.0,
    tracer_length: 0.0,
    tracer_color: [40, 40, 40, 0],
    tracer_thickness: 0.0,
    tracer_lifetime: 0.0,

    crosshair_kick_per_shot: 0.5,
    crosshair_kick_decay: 7.0,
    crosshair_kick_max: 1.8,

    fsa_time: Some(0.25),

    fire_sound_asset_key: "ar_shot",
    icon_asset_key: "ar_icon",

    viewmodel_model: Some("as_val_model"),
    viewmodel_asset_key: None,

    falloff_start: 15.0,
    falloff_end: 40.0,
    falloff_min_mult: 0.4,

    // Tight, controllable pattern — rewards sustained fire at close range
    recoil_pattern: &[
        (0.009, 0.0),
        (0.009, 0.0),
        (0.010, 0.05),
        (0.010, 0.1),
        (0.009, 0.05),
        (0.008, -0.05),
        (0.008, -0.1),
        (0.007, -0.05),
        (0.007, 0.0),
        (0.006, 0.0),
    ],
    pattern_randomness: 0.06,
    randomness_growth: 0.03,
    pattern_reset_time: 0.35,

    ..M16_STATS
};
