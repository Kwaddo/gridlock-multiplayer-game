use crate::weapons::stats::{CrosshairStyle, FireMode, WeaponStats};

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
    viewmodel_ads_scale: 0.0019,
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

pub const TAURUS_RAGING_SHOTGUN_PELLET_SPREAD: &[(f32, f32)] = &[
    (0.0, 0.0),    // center
    (0.6283, 0.7), // ring of 4
    (1.8850, 0.7),
    (3.1416, 0.7),
    (4.7124, 0.7),
];

/// Taurus Raging Judge — LeMat-style hybrid sidearm.
/// Default profile is normal revolver-like rounds, with a toggle to shotgun shell mode.
pub const TAURUS_RAGING: WeaponStats = WeaponStats {
    name: "Taurus Raging",
    fire_mode: FireMode::SemiAuto,

    damage: 34.0,
    headshot_mult: 2.0,
    pellet_count: 1,
    pellet_spread: None,

    fire_rate: 2.2,
    magazine_size: 5,
    max_reserve: 30,
    reload_time: 2.2,

    recoil_vertical: 0.035,
    recoil_horizontal: 0.012,
    recoil_recovery: 3.5,

    hip_spread: 0.028,
    ads_spread: 0.006,
    spread_per_shot: 0.02,
    spread_recovery: 1.8,
    move_spread_mult: 1.3,
    sprint_spread_mult: 1.6,
    crouch_spread_mult: 0.8,

    ads_zoom_fov: 65.0,
    ads_time: 0.2,
    ads_sensitivity: None,

    bullet_speed: 210.0,
    bullet_range: 45.0,
    tracer_length: 2.2,
    tracer_color: [255, 190, 70, 255],
    tracer_thickness: 0.012,
    tracer_lifetime: 1.3,
    bullet_hole_size: 0.22,
    bullet_hole_lifetime: 5.0,
    bullet_hole_color: [200, 200, 200, 255],

    crosshair_kick_per_shot: 1.7,
    crosshair_kick_decay: 6.0,
    crosshair_kick_max: 2.3,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: Some(0.4),
    fsa_suppresses_kick: false,

    fire_sound_asset_key: "revolver_shot",
    icon_asset_key: "pistol_icon",
    icon_width: 85.0,
    icon_height: 50.0,
    icon_flip_x: false,

    viewmodel_model: Some("taurus_raging_model"),
    viewmodel_asset_key: None,
    viewmodel_scale: 0.0019,
    viewmodel_ads_scale: 0.0019,
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
    falloff_end: 45.0,
    falloff_min_mult: 0.55,

    recoil_pattern: &[
        (0.035, 0.0),
        (0.030, 0.1),
        (0.028, -0.1),
        (0.025, 0.15),
        (0.022, -0.15),
    ],
    pattern_randomness: 0.2,
    randomness_growth: 0.1,
    pattern_reset_time: 0.8,
};
