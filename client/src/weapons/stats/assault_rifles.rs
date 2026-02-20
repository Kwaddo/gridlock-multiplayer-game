use super::{CrosshairStyle, FireMode, WeaponStats};

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

    crosshair_kick_per_shot: 0.8,
    crosshair_kick_decay: 5.0,
    crosshair_kick_max: 2.3,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: Some(0.35),

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
    viewmodel_muzzle_offset: (-1.00, 0.02, -0.25),
    viewmodel_hands_scale: 1.93,
    viewmodel_hands_offset: (0.16, -0.16, -0.28),
    viewmodel_hands_rotation: (-0.15, 0.45, -0.05),
    viewmodel_pair_offset_pos: (0.0, 0.0, 0.0),
    viewmodel_pair_offset_rot: (0.0, 0.0, 0.0),
    viewmodel_ads_pos_offset: (0.08, 0.03, 0.0),
    viewmodel_ads_rot_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_pos_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_rot_offset: (0.0, 0.0, 0.0),

    falloff_start: 25.0,
    falloff_end: 70.0,
    falloff_min_mult: 0.6,

    recoil_pattern: &[
        (0.010, 0.0),
        (0.011, 0.0),
        (0.012, 0.1),
        (0.012, 0.2),
        (0.011, 0.3),
        (0.010, 0.1),
        (0.010, -0.2),
        (0.009, -0.4),
        (0.008, -0.3),
        (0.007, -0.1),
    ],
    pattern_randomness: 0.1,
    randomness_growth: 0.06,
    pattern_reset_time: 0.5,
};

// M4A1 - versatile assault rifle, balanced stats all around
pub const M4A1: WeaponStats = WeaponStats {
    name: "M4A1",
    viewmodel_model: Some("m4a1_model"),
    viewmodel_asset_key: None,
    ..M16_STATS
};

// TODO: AK-47 — high damage / high recoil, strongest per shot AR, hardest to control
pub const AK: WeaponStats = WeaponStats {
    name: "AK-47",
    viewmodel_model: Some("ak_model"),
    viewmodel_asset_key: None,
    ..M16_STATS
};

// TODO: FAMAS — burst rifle, burst accuracy reward weapon
pub const FAMAS: WeaponStats = WeaponStats {
    name: "FAMAS",
    viewmodel_model: Some("famas_model"),
    viewmodel_asset_key: None,
    ..M16_STATS
};

// TODO: SCAR — heavy rifle, slower fire rate but hits harder than M4
pub const SCAR: WeaponStats = WeaponStats {
    name: "SCAR",
    viewmodel_model: Some("scar_model"),
    viewmodel_asset_key: None,
    ..M16_STATS
};

// TODO: AS VAL — suppressed close-range rifle, stealth + fast TTK but short range
pub const AS_VAL: WeaponStats = WeaponStats {
    name: "AS VAL",
    viewmodel_model: Some("as_val_model"),
    viewmodel_asset_key: None,
    ..M16_STATS
};
