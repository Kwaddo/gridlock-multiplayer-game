use super::{CrosshairStyle, FireMode, WeaponStats};

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
    reload_time: 1.88,

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
    bullet_hole_size: 0.2,
    bullet_hole_lifetime: 5.0,
    bullet_hole_color: [200, 200, 200, 255],

    crosshair_kick_per_shot: 1.5,
    crosshair_kick_decay: 8.0,
    crosshair_kick_max: 1.5,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: Some(0.25),
    fsa_suppresses_kick: false,

    fire_sound_asset_key: "pistol_shot",
    icon_asset_key: "pistol_icon",
    icon_width: 80.0,
    icon_height: 50.0,
    icon_flip_x: false,

    viewmodel_model: Some("m1911_model"),
    viewmodel_asset_key: Some("weapon_fp_pistol"),
    viewmodel_scale: 0.0019,
    viewmodel_position: (0.171, 0.152, -0.300),
    viewmodel_rotation: (-0.100, -1.645, 0.000),
    viewmodel_muzzle_offset: (-1.100, 0.020, -0.250),
    viewmodel_hands_scale: 0.074,
    viewmodel_hands_offset: (0.181, 0.111, -0.390),
    viewmodel_hands_rotation: (-0.490, -0.175, -0.135),
    viewmodel_pair_offset_pos: (0.049, -0.003, -0.006),
    viewmodel_pair_offset_rot: (0.100, 0.015, 0.000),
    viewmodel_ads_pos_offset: (0.179, -0.009, -0.145),
    viewmodel_ads_rot_offset: (0.455, 0.080, 0.215),
    viewmodel_ads_hands_pos_offset: (0.132, 0.035, -0.427),
    viewmodel_ads_hands_rot_offset: (0.105, 0.225, 0.085),

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
    reload_time: 1.88,

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
    bullet_hole_size: 0.25,
    bullet_hole_lifetime: 5.0,
    bullet_hole_color: [200, 200, 200, 255],

    crosshair_kick_per_shot: 2.0,
    crosshair_kick_decay: 6.0,
    crosshair_kick_max: 3.0,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: Some(0.3),
    fsa_suppresses_kick: false,

    fire_sound_asset_key: "deagle_shot",
    icon_asset_key: "pistol_icon",
    icon_width: 90.0,
    icon_height: 50.0,
    icon_flip_x: false,

    viewmodel_model: Some("deagle_model"),
    viewmodel_asset_key: None,
    viewmodel_scale: 0.0019,
    viewmodel_position: (0.171, 0.152, -0.300),
    viewmodel_rotation: (-0.100, -1.645, 0.000),
    viewmodel_muzzle_offset: (-1.100, 0.020, -0.250),
    viewmodel_hands_scale: 0.074,
    viewmodel_hands_offset: (0.181, 0.111, -0.390),
    viewmodel_hands_rotation: (-0.490, -0.175, -0.135),
    viewmodel_pair_offset_pos: (0.049, -0.003, -0.006),
    viewmodel_pair_offset_rot: (0.100, 0.015, 0.000),
    viewmodel_ads_pos_offset: (0.179, -0.009, -0.145),
    viewmodel_ads_rot_offset: (0.455, 0.080, 0.215),
    viewmodel_ads_hands_pos_offset: (0.132, 0.035, -0.427),
    viewmodel_ads_hands_rot_offset: (0.105, 0.225, 0.085),

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
    reload_time: 1.88,

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
    bullet_hole_size: 0.22,
    bullet_hole_lifetime: 5.0,
    bullet_hole_color: [200, 200, 200, 255],

    crosshair_kick_per_shot: 1.8,
    crosshair_kick_decay: 5.0,
    crosshair_kick_max: 2.5,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: Some(0.35),
    fsa_suppresses_kick: false,

    fire_sound_asset_key: "revolver_shot",
    icon_asset_key: "pistol_icon",
    icon_width: 85.0,
    icon_height: 50.0,
    icon_flip_x: false,

    viewmodel_model: Some("revolver_model"),
    viewmodel_asset_key: None,
    viewmodel_scale: 0.0019,
    viewmodel_position: (0.171, 0.152, -0.300),
    viewmodel_rotation: (-0.100, -1.645, 0.000),
    viewmodel_muzzle_offset: (-1.100, 0.020, -0.250),
    viewmodel_hands_scale: 0.074,
    viewmodel_hands_offset: (0.181, 0.111, -0.390),
    viewmodel_hands_rotation: (-0.490, -0.175, -0.135),
    viewmodel_pair_offset_pos: (0.049, -0.003, -0.006),
    viewmodel_pair_offset_rot: (0.100, 0.015, 0.000),
    viewmodel_ads_pos_offset: (0.179, -0.009, -0.145),
    viewmodel_ads_rot_offset: (0.455, 0.080, 0.215),
    viewmodel_ads_hands_pos_offset: (0.132, 0.035, -0.427),
    viewmodel_ads_hands_rot_offset: (0.105, 0.225, 0.085),

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

/// Suppressed Pistol — stealth sidearm, tight spread + invisible tracers
pub const SUPPRESSED_PISTOL: WeaponStats = WeaponStats {
    name: "Suppressed Pistol",
    damage: 24.0,
    headshot_mult: 2.0,
    fire_rate: 4.5,
    magazine_size: 15,
    max_reserve: 60,
    reload_time: 1.88,

    recoil_vertical: 0.012,
    recoil_horizontal: 0.003,
    recoil_recovery: 6.0,

    hip_spread: 0.008,
    ads_spread: 0.002,
    spread_per_shot: 0.012,
    spread_recovery: 3.0,
    move_spread_mult: 1.1,
    sprint_spread_mult: 1.3,
    crouch_spread_mult: 0.8,

    ads_zoom_fov: 65.0,
    ads_time: 0.15,

    // Suppressed — invisible tracers, no muzzle flash
    bullet_speed: 180.0,
    bullet_range: 35.0,
    tracer_length: 0.0,
    tracer_color: [40, 40, 40, 0],
    tracer_thickness: 0.0,
    tracer_lifetime: 0.0,

    crosshair_kick_per_shot: 1.0,
    crosshair_kick_decay: 9.0,
    crosshair_kick_max: 1.2,

    fsa_time: Some(0.2),
    fsa_suppresses_kick: false,

    fire_sound_asset_key: "pistol_shot",
    icon_asset_key: "pistol_icon",

    viewmodel_model: Some("suppressed_pistol_model"),
    viewmodel_asset_key: None,

    falloff_start: 12.0,
    falloff_end: 30.0,
    falloff_min_mult: 0.45,

    recoil_pattern: &[
        (0.012, 0.0),
        (0.011, 0.03),
        (0.010, -0.03),
        (0.010, 0.04),
        (0.009, -0.04),
    ],
    pattern_randomness: 0.1,
    randomness_growth: 0.05,
    pattern_reset_time: 0.35,

    ..M1911
};

// TODO: Taurus Raging Judge — hybrid revolver shotgun, alt-fire shotgun blast
pub const TAURUS_RAGING: WeaponStats = WeaponStats {
    name: "Taurus Raging",
    viewmodel_model: Some("taurus_raging_model"),
    viewmodel_asset_key: None,
    ..M1911
};
