use crate::weapons::stats::{ CrosshairStyle, FireMode, WeaponStats };

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
    reload_time: 1.85,

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
    viewmodel_ads_scale: 0.003,
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
    reload_time: 1.85,

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

    viewmodel_scale: 0.003,
    viewmodel_ads_scale: 0.003,
    viewmodel_position: (0.22, 0.08, -0.31),
    viewmodel_rotation: (0.05, -1.75, 0.0),
    viewmodel_muzzle_offset: (-1.0, 0.02, -0.25),
    viewmodel_hands_scale: 1.93,
    viewmodel_hands_offset: (0.16, -0.16, -0.28),
    viewmodel_hands_rotation: (-0.15, 0.45, -0.05),
    viewmodel_pair_offset_pos: (0.0, 0.0, 0.0),
    viewmodel_pair_offset_rot: (0.0, 0.0, 0.0),
    viewmodel_ads_pos_offset: (0.125, 0.024, -0.151),
    viewmodel_ads_rot_offset: (0.005, 0.175, -0.02),
    viewmodel_ads_hands_pos_offset: (0.0, 0.0, -0.704),
    viewmodel_ads_hands_rot_offset: (0.0, 0.0, 0.0),

    falloff_start: 30.0,
    falloff_end: 75.0,
    falloff_min_mult: 0.55,

    // Clean, easy-to-learn pattern — slight rightward drift then centers
    recoil_pattern: &[
        (0.008, 0.0),
        (0.008, 0.0),
        (0.009, 0.05),
        (0.009, 0.1),
        (0.01, 0.15),
        (0.01, 0.1),
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
    reload_time: 1.85,

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

    viewmodel_scale: 0.003,
    viewmodel_ads_scale: 0.0032,
    viewmodel_position: (0.22, 0.063, -0.31),
    viewmodel_rotation: (0.05, -1.75, 0.0),
    viewmodel_muzzle_offset: (-1.0, 0.02, -0.25),
    viewmodel_hands_scale: 1.93,
    viewmodel_hands_offset: (0.16, -0.16, -0.28),
    viewmodel_hands_rotation: (-0.15, 0.45, -0.05),
    viewmodel_pair_offset_pos: (0.0, 0.0, 0.0),
    viewmodel_pair_offset_rot: (0.0, 0.0, 0.0),
    viewmodel_ads_pos_offset: (0.125, 0.034, -0.251),
    viewmodel_ads_rot_offset: (0.02, 0.185, -0.02),
    viewmodel_ads_hands_pos_offset: (0.0, 0.0, -0.704),
    viewmodel_ads_hands_rot_offset: (0.0, 0.0, 0.0),

    falloff_start: 25.0,
    falloff_end: 70.0,
    falloff_min_mult: 0.6,

    // Wild pattern — heavy vertical climb with unpredictable horizontal sway
    recoil_pattern: &[
        (0.02, 0.0),
        (0.022, 0.0),
        (0.025, 0.15),
        (0.025, 0.3),
        (0.024, 0.2),
        (0.022, -0.1),
        (0.02, -0.35),
        (0.018, -0.5),
        (0.016, -0.3),
        (0.015, 0.1),
    ],
    pattern_randomness: 0.2,
    randomness_growth: 0.08,
    pattern_reset_time: 0.6,

    ..M16_STATS
};
