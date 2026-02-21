use super::{CrosshairStyle, FireMode, WeaponStats, VM_BASE_POS, VM_BASE_ROT, VM_BASE_SCALE};

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
    reload_time: 3.7,

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
    viewmodel_scale: VM_BASE_SCALE,
    viewmodel_position: VM_BASE_POS,
    viewmodel_rotation: VM_BASE_ROT,
    viewmodel_muzzle_offset: (-1.20, 0.02, -0.28),
    viewmodel_hands_scale: 1.93,
    viewmodel_hands_offset: (0.0, 0.0, 0.0),
    viewmodel_hands_rotation: (-0.15, 0.65, -0.05),
    viewmodel_pair_offset_pos: (0.0, 0.0, 0.0),
    viewmodel_pair_offset_rot: (0.0, 0.0, 0.0),
    viewmodel_ads_pos_offset: (0.08, 0.03, 0.0),
    viewmodel_ads_rot_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_pos_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_rot_offset: (0.0, 0.0, 0.0),

    falloff_start: 1000.0,
    falloff_end: 10000.0,
    falloff_min_mult: 1.0,

    recoil_pattern: &[],
    pattern_randomness: 0.0,
    randomness_growth: 0.0,
    pattern_reset_time: 0.0,
};

// TODO: Hunting Rifle — fill in unique sniper stats
pub const HUNTING_RIFLE: WeaponStats = WeaponStats {
    name: "Hunting Rifle",
    viewmodel_model: Some("hunting_rifle_model"),
    viewmodel_asset_key: None,
    ..BOLT_ACTION
};

// TODO: Semi-Auto Sniper — fill in unique sniper stats
pub const SEMI_AUTO_SNIPER: WeaponStats = WeaponStats {
    name: "Semi-Auto Sniper",
    viewmodel_model: Some("semi_auto_sniper_model"),
    viewmodel_asset_key: None,
    ..BOLT_ACTION
};
