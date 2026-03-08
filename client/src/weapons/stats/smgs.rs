use super::{ CrosshairStyle, FireMode, WeaponStats, VM_BASE_POS, VM_BASE_ROT, VM_BASE_SCALE };

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
    reload_time: 1.61,

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
    bullet_hole_size: 0.18,
    bullet_hole_lifetime: 5.0,
    bullet_hole_color: [200, 200, 200, 255],

    crosshair_kick_per_shot: 0.8,
    crosshair_kick_decay: 6.0,
    crosshair_kick_max: 2.5,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: None,
    fsa_suppresses_kick: false,

    fire_sound_asset_key: "smg_shot",
    icon_asset_key: "smg_icon",
    icon_width: 90.0,
    icon_height: 45.0,
    icon_flip_x: true,

    viewmodel_model: Some("mp5_model"),
    viewmodel_asset_key: None,

    viewmodel_scale: 0.0041,
    viewmodel_ads_scale: 0.003,
    viewmodel_position: (0.176, 0.052, -0.3),
    viewmodel_rotation: (-0.1, -1.75, 0.0),
    viewmodel_muzzle_offset: (-1.1, 0.02, -0.25),
    viewmodel_hands_scale: 0.074,
    viewmodel_hands_offset: (0.086, 0.041, -0.401),
    viewmodel_hands_rotation: (-0.245, 0.275, -0.135),
    viewmodel_pair_offset_pos: (0.049, -0.003, -0.003),
    viewmodel_pair_offset_rot: (0.1, 0.175, 0.0),
    viewmodel_ads_pos_offset: (0.174, 0.05, -0.147),
    viewmodel_ads_rot_offset: (0.0, 0.175, -0.15),
    viewmodel_ads_hands_pos_offset: (0.0, 0.0, -0.69),
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
        (0.01, 0.3),
        (0.01, 0.1),
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

/// P90 — bullet hose, huge mag + highest fire rate SMG
pub const P90: WeaponStats = WeaponStats {
    name: "P90",
    damage: 17.0,
    headshot_mult: 1.5,
    fire_rate: 13.0,
    magazine_size: 50,
    max_reserve: 150,
    reload_time: 1.61,

    recoil_vertical: 0.014,
    recoil_horizontal: 0.012,
    recoil_recovery: 4.5,

    hip_spread: 0.03,
    ads_spread: 0.015,
    spread_per_shot: 0.018,
    spread_recovery: 2.0,
    move_spread_mult: 1.1,
    sprint_spread_mult: 1.5,
    crouch_spread_mult: 0.85,

    ads_zoom_fov: 70.0,
    ads_time: 0.18,

    bullet_speed: 190.0,
    bullet_range: 40.0,
    tracer_length: 2.0,
    tracer_color: [255, 200, 80, 255],
    tracer_thickness: 0.008,
    tracer_lifetime: 1.0,

    crosshair_kick_per_shot: 0.6,
    crosshair_kick_decay: 5.0,
    crosshair_kick_max: 3.0,

    fsa_time: None,

    fire_sound_asset_key: "smg_shot",
    icon_asset_key: "smg_icon",

    viewmodel_model: Some("p90_model"),
    viewmodel_asset_key: None,

    falloff_start: 8.0,
    falloff_end: 25.0,
    falloff_min_mult: 0.35,

    // Loose spray — volume over precision
    recoil_pattern: &[
        (0.014, 0.0),
        (0.014, 0.05),
        (0.013, 0.15),
        (0.013, 0.25),
        (0.012, 0.2),
        (0.012, 0.0),
        (0.011, -0.15),
        (0.011, -0.3),
        (0.01, -0.2),
        (0.01, -0.05),
        (0.009, 0.1),
        (0.009, 0.0),
    ],
    pattern_randomness: 0.35,
    randomness_growth: 0.05,
    pattern_reset_time: 0.3,

    ..MP5
};

/// Honey Badger — suppressed AR/SMG hybrid, good damage with stealth tracers
pub const HONEY_BADGER: WeaponStats = WeaponStats {
    name: "Honey Badger",
    fire_mode: FireMode::FullAuto,

    damage: 26.0,
    headshot_mult: 1.8,
    pellet_count: 1,
    pellet_spread: None,
    fire_rate: 8.0,
    magazine_size: 30,
    max_reserve: 90,
    reload_time: 1.75,

    recoil_vertical: 0.016,
    recoil_horizontal: 0.008,
    recoil_recovery: 4.5,

    hip_spread: 0.022,
    ads_spread: 0.008,
    spread_per_shot: 0.014,
    spread_recovery: 2.8,
    move_spread_mult: 1.15,
    sprint_spread_mult: 1.4,
    crouch_spread_mult: 0.8,

    ads_zoom_fov: 65.0,
    ads_time: 0.2,
    ads_sensitivity: None,

    // Suppressed — faint tracers
    bullet_speed: 190.0,
    bullet_range: 50.0,
    tracer_length: 0.5,
    tracer_color: [80, 80, 80, 80],
    tracer_thickness: 0.005,
    tracer_lifetime: 0.4,
    bullet_hole_size: 0.18,
    bullet_hole_lifetime: 5.0,
    bullet_hole_color: [200, 200, 200, 255],

    crosshair_kick_per_shot: 0.9,
    crosshair_kick_decay: 7.0,
    crosshair_kick_max: 2.2,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: Some(0.22),
    fsa_suppresses_kick: false,

    fire_sound_asset_key: "smg_shot",
    icon_asset_key: "smg_icon",
    icon_width: 95.0,
    icon_height: 45.0,
    icon_flip_x: true,

    viewmodel_model: Some("honey_badger_model"),
    viewmodel_asset_key: None,
    viewmodel_scale: 0.0041,
    viewmodel_ads_scale: 0.003,
    viewmodel_position: (0.176, 0.052, -0.222),
    viewmodel_rotation: (-0.1, -1.675, 0.0),
    viewmodel_muzzle_offset: (-1.1, 0.02, -0.25),
    viewmodel_hands_scale: 0.074,
    viewmodel_hands_offset: (0.075, 0.04, -0.392),
    viewmodel_hands_rotation: (-0.245, 0.375, -0.135),
    viewmodel_pair_offset_pos: (0.049, -0.003, -0.037),
    viewmodel_pair_offset_rot: (0.1, 0.175, 0.0),
    viewmodel_ads_pos_offset: (0.173, 0.05, -0.147),
    viewmodel_ads_rot_offset: (0.0, 0.105, -0.15),
    viewmodel_ads_hands_pos_offset: (0.174, 0.05, -0.147),
    viewmodel_ads_hands_rot_offset: (0.0, 0.175, -0.15),

    falloff_start: 12.0,
    falloff_end: 35.0,
    falloff_min_mult: 0.45,

    recoil_pattern: &[
        (0.016, 0.0),
        (0.015, 0.03),
        (0.014, 0.08),
        (0.014, 0.15),
        (0.013, 0.1),
        (0.012, -0.05),
        (0.011, -0.15),
        (0.011, -0.1),
        (0.01, 0.0),
        (0.009, 0.05),
    ],
    pattern_randomness: 0.2,
    randomness_growth: 0.04,
    pattern_reset_time: 0.35,
};

/// Tommy Gun — vintage SMG, heavy recoil but strong close range, drum mag
pub const TOMMY_GUN: WeaponStats = WeaponStats {
    name: "Tommy Gun",
    damage: 23.0,
    headshot_mult: 1.5,
    fire_rate: 9.0,
    magazine_size: 40,
    max_reserve: 120,
    reload_time: 1.61,

    recoil_vertical: 0.02,
    recoil_horizontal: 0.016,
    recoil_recovery: 3.5,

    hip_spread: 0.035,
    ads_spread: 0.018,
    spread_per_shot: 0.02,
    spread_recovery: 1.8,
    move_spread_mult: 1.2,
    sprint_spread_mult: 1.6,
    crouch_spread_mult: 0.8,

    ads_zoom_fov: 70.0,
    ads_time: 0.22,

    bullet_speed: 170.0,
    bullet_range: 35.0,
    tracer_length: 2.5,
    tracer_color: [255, 190, 70, 255],
    tracer_thickness: 0.012,
    tracer_lifetime: 1.0,

    crosshair_kick_per_shot: 1.2,
    crosshair_kick_decay: 4.0,
    crosshair_kick_max: 3.5,

    fsa_time: None,

    fire_sound_asset_key: "smg_shot",
    icon_asset_key: "smg_icon",

    viewmodel_model: Some("tommy_gun_model"),
    viewmodel_asset_key: None,

    // Tommy Gun viewmodel
    viewmodel_scale: 0.0041,
    viewmodel_ads_scale: 0.003,
    viewmodel_position: (0.176, 0.052, -0.276),
    viewmodel_rotation: (-0.1, -1.75, 0.0),
    viewmodel_muzzle_offset: (-1.1, 0.02, -0.25),
    viewmodel_hands_scale: 0.074,
    viewmodel_hands_offset: (0.086, 0.041, -0.401),
    viewmodel_hands_rotation: (-0.245, 0.275, -0.135),
    viewmodel_pair_offset_pos: (0.049, -0.003, -0.003),
    viewmodel_pair_offset_rot: (0.1, 0.175, 0.0),
    viewmodel_ads_pos_offset: (0.174, 0.05, -0.147),
    viewmodel_ads_rot_offset: (0.0, 0.175, -0.15),
    viewmodel_ads_hands_pos_offset: (0.0, 0.0, -0.69),
    viewmodel_ads_hands_rot_offset: (0.0, 0.0, 0.0),

    falloff_start: 8.0,
    falloff_end: 25.0,
    falloff_min_mult: 0.35,

    // Wild pattern — heavy climb with wide horizontal sway
    recoil_pattern: &[
        (0.02, 0.0),
        (0.02, 0.0),
        (0.022, 0.1),
        (0.022, 0.25),
        (0.021, 0.35),
        (0.02, 0.15),
        (0.019, -0.15),
        (0.018, -0.35),
        (0.017, -0.25),
        (0.016, -0.1),
        (0.015, 0.1),
        (0.014, 0.0),
    ],
    pattern_randomness: 0.35,
    randomness_growth: 0.06,
    pattern_reset_time: 0.4,

    ..MP5
};
