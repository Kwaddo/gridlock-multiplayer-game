use super::{CrosshairStyle, FireMode, WeaponStats};

/// Bolt Action — classic sniper, one shot one kill
pub const BOLT_ACTION: WeaponStats = WeaponStats {
    name: "Bolt Action",
    fire_mode: FireMode::BoltAction,

    damage: 105.0,
    headshot_mult: 2.5,
    pellet_count: 1,
    pellet_spread: None,
    fire_rate: 0.8,
    magazine_size: 1,
    max_reserve: 8,
    reload_time: 1.60,

    recoil_vertical: 0.08,
    recoil_horizontal: 0.01,
    recoil_recovery: 2.0,

    hip_spread: 0.1,
    ads_spread: 0.001,
    spread_per_shot: 0.06,
    spread_recovery: 0.8,
    move_spread_mult: 2.0,
    sprint_spread_mult: 3.0,
    crouch_spread_mult: 0.5,

    ads_zoom_fov: 15.0,
    ads_time: 0.05,
    ads_sensitivity: Some(0.2),

    bullet_speed: 350.0,
    bullet_range: 120.0,
    tracer_length: 8.0,
    tracer_color: [180, 200, 255, 255],
    tracer_thickness: 0.03,
    tracer_lifetime: 3.5,
    bullet_hole_size: 0.5,
    bullet_hole_lifetime: 5.0,
    bullet_hole_color: [200, 200, 200, 255],

    crosshair_kick_per_shot: 0.0,
    crosshair_kick_decay: 0.0,
    crosshair_kick_max: 0.0,

    crosshair_style: CrosshairStyle::Scope,
    fsa_time: None,
    fsa_suppresses_kick: false,

    fire_sound_asset_key: "sniper_shot",
    icon_asset_key: "sniper_icon",
    icon_width: 110.0,
    icon_height: 40.0,
    icon_flip_x: false,

    viewmodel_model: Some("bolt_action_model"),
    viewmodel_asset_key: None,
    viewmodel_scale: 0.0030,
    viewmodel_ads_scale: 0.0030,
    viewmodel_position: (0.220, 0.082, -0.300),
    viewmodel_rotation: (0.050, -1.680, 0.000),
    viewmodel_muzzle_offset: (-1.000, 0.020, -0.250),
    viewmodel_hands_scale: 0.130,
    viewmodel_hands_offset: (-0.078, -0.085, -0.290),
    viewmodel_hands_rotation: (-0.400, 0.695, -0.100),
    viewmodel_pair_offset_pos: (0.000, 0.000, 0.050),
    viewmodel_pair_offset_rot: (0.000, 0.100, 0.000),
    viewmodel_ads_pos_offset: (0.132, 0.025, -0.151),
    viewmodel_ads_rot_offset: (0.015, 0.195, -0.020),
    viewmodel_ads_hands_pos_offset: (0.000, 0.000, -0.704),
    viewmodel_ads_hands_rot_offset: (0.000, 0.000, 0.000),

    falloff_start: 1000.0,
    falloff_end: 10000.0,
    falloff_min_mult: 1.0,

    recoil_pattern: &[],
    pattern_randomness: 0.0,
    randomness_growth: 0.0,
    pattern_reset_time: 0.0,
};

/// Hunting Rifle — Fortnite-style no-scope sniper, standard crosshair, high body-shot damage
pub const HUNTING_RIFLE: WeaponStats = WeaponStats {
    name: "Hunting Rifle",
    fire_mode: FireMode::BoltAction,
    damage: 75.0,
    headshot_mult: 1.8, 
    fire_rate: 1.2,
    magazine_size: 1,
    max_reserve: 10,
    reload_time: 1.60,

    recoil_vertical: 0.06,
    recoil_horizontal: 0.008,
    recoil_recovery: 2.5,

    hip_spread: 0.04,
    ads_spread: 0.003,
    spread_per_shot: 0.04,
    spread_recovery: 1.0,
    move_spread_mult: 1.6,
    sprint_spread_mult: 2.5,
    crouch_spread_mult: 0.6,

    ads_zoom_fov: 45.0,
    ads_time: 0.2,
    ads_sensitivity: None, // no scope, normal sens

    bullet_speed: 320.0,
    bullet_range: 100.0,
    tracer_length: 6.0,
    tracer_color: [200, 210, 255, 255],
    tracer_thickness: 0.025,
    tracer_lifetime: 3.0,

    crosshair_kick_per_shot: 0.0,
    crosshair_kick_decay: 0.0,
    crosshair_kick_max: 0.0,

    crosshair_style: CrosshairStyle::Standard, // no scope — standard crosshair
    fsa_time: None,

    fire_sound_asset_key: "sniper_shot",
    icon_asset_key: "sniper_icon",

    viewmodel_model: Some("hunting_rifle_model"),
    viewmodel_asset_key: None,

    falloff_start: 1000.0,
    falloff_end: 10000.0,
    falloff_min_mult: 1.0, // no falloff

    ..BOLT_ACTION // means for any fields we didnt explicitly set, take from bolt action
};

/// Semi-Auto Sniper — Fortnite-style scoped, lower damage but fast follow-ups
pub const SEMI_AUTO_SNIPER: WeaponStats = WeaponStats {
    name: "Semi-Auto Sniper",
    fire_mode: FireMode::SemiAuto,
    damage: 58.0,
    headshot_mult: 2.0,
    fire_rate: 2.0,
    magazine_size: 10,
    max_reserve: 30,
    reload_time: 1.60,

    recoil_vertical: 0.04,
    recoil_horizontal: 0.006,
    recoil_recovery: 3.0,

    hip_spread: 0.08,
    ads_spread: 0.002,
    spread_per_shot: 0.03,
    spread_recovery: 1.2,
    move_spread_mult: 1.8,
    sprint_spread_mult: 2.5,
    crouch_spread_mult: 0.5,

    ads_zoom_fov: 20.0,
    ads_time: 0.3,
    ads_sensitivity: Some(0.25),

    bullet_speed: 330.0,
    bullet_range: 110.0,
    tracer_length: 7.0,
    tracer_color: [180, 200, 255, 255],
    tracer_thickness: 0.025,
    tracer_lifetime: 3.0,

    crosshair_kick_per_shot: 0.0,
    crosshair_kick_decay: 0.0,
    crosshair_kick_max: 0.0,

    crosshair_style: CrosshairStyle::Scope,
    fsa_time: None,

    fire_sound_asset_key: "sniper_shot",
    icon_asset_key: "sniper_icon",

    viewmodel_model: Some("semi_auto_sniper_model"),
    viewmodel_asset_key: None,

    falloff_start: 1000.0,
    falloff_end: 10000.0,
    falloff_min_mult: 1.0, // no falloff

    recoil_pattern: &[
        (0.040, 0.0),
        (0.035, 0.05),
        (0.030, -0.05),
        (0.028, 0.08),
        (0.025, -0.08),
    ],
    pattern_randomness: 0.15,
    randomness_growth: 0.08,
    pattern_reset_time: 0.6,

    ..BOLT_ACTION
};
