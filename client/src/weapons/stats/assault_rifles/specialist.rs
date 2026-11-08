use crate::weapons::stats::{ FireMode, WeaponStats };

use super::classic::M16_STATS;

/// FAMAS — 3-round burst, rewards precise timing and aim
pub const FAMAS: WeaponStats = WeaponStats {
    name: "FAMAS",
    fire_mode: FireMode::Burst(3),
    damage: 24.0,
    headshot_mult: 1.8,
    fire_rate: 8.0, // speed within burst — extra pause after last shot added in fire logic
    magazine_size: 30,
    max_reserve: 90,
    reload_time: 1.85,

    recoil_vertical: 0.012,
    recoil_horizontal: 0.006,
    recoil_recovery: 10.0, // fast reset between bursts

    hip_spread: 0.032,
    ads_spread: 0.006,
    spread_per_shot: 0.005, // low bloom within burst
    spread_recovery: 8.0, // resets fast between bursts
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
    viewmodel_ads_pos_offset: (0.13, 0.025, -0.213),
    viewmodel_ads_rot_offset: (-0.015, 0.185, -0.02),
    viewmodel_ads_hands_pos_offset: (0.0, 0.0, -0.704),
    viewmodel_ads_hands_rot_offset: (0.0, 0.0, 0.0),

    falloff_start: 25.0,
    falloff_end: 65.0,
    falloff_min_mult: 0.55,

    // Tight 3-shot climb — predictable within each burst
    recoil_pattern: &[
        (0.01, 0.0),
        (0.012, 0.05),
        (0.014, 0.1),
        (0.01, -0.05),
        (0.012, -0.1),
        (0.014, 0.0),
        (0.01, 0.1),
        (0.012, -0.05),
        (0.014, -0.1),
        (0.01, 0.0),
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
    reload_time: 1.85,

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

    viewmodel_scale: 0.003,
    viewmodel_ads_scale: 0.003,
    viewmodel_position: (0.22, 0.08, -0.31),
    viewmodel_rotation: (0.08, -1.75, 0.0),
    viewmodel_muzzle_offset: (-1.0, 0.02, -0.25),
    viewmodel_hands_scale: 1.93,
    viewmodel_hands_offset: (0.16, -0.16, -0.28),
    viewmodel_hands_rotation: (-0.15, 0.45, -0.05),
    viewmodel_pair_offset_pos: (0.0, 0.0, 0.0),
    viewmodel_pair_offset_rot: (0.0, 0.0, 0.0),
    viewmodel_ads_pos_offset: (0.134, 0.026, -0.157),
    viewmodel_ads_rot_offset: (0.015, 0.18, -0.02),
    viewmodel_ads_hands_pos_offset: (0.0, 0.0, -0.704),
    viewmodel_ads_hands_rot_offset: (0.0, 0.0, 0.0),

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
    reload_time: 1.85,

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

    falloff_start: 15.0,
    falloff_end: 40.0,
    falloff_min_mult: 0.4,

    // Tight, controllable pattern — rewards sustained fire at close range
    recoil_pattern: &[
        (0.009, 0.0),
        (0.009, 0.0),
        (0.01, 0.05),
        (0.01, 0.1),
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
