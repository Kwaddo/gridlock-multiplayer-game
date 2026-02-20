use super::{CrosshairStyle, FireMode, WeaponStats};

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

// Benelli is a stub — Spas is the default shotgun
pub const BENELLI: WeaponStats = WeaponStats {
    name: "Benelli",
    viewmodel_model: Some("benelli_model"),
    viewmodel_asset_key: None,
    ..SPAS
};
