use super::{ CrosshairStyle, FireMode, WeaponStats };

/// SPAS-12 — tactical shotgun, mid-range consistency
pub const SPAS: WeaponStats = WeaponStats {
    name: "SPAS-12",
    fire_mode: FireMode::SemiAuto,

    damage: 12.0,
    headshot_mult: 1.5,
    pellet_count: 9,
    pellet_spread: Some(
        &[
            (0.0, 0.0),
            (0.4363, 0.9),
            (1.1345, 0.9),
            (2.0071, 0.9),
            (2.7053, 0.9),
            (3.5779, 0.9),
            (4.2761, 0.9),
            (5.1487, 0.9),
            (5.8469, 0.9),
        ]
    ),

    fire_rate: 1.0,
    magazine_size: 7,
    max_reserve: 28,
    reload_time: 1.85,

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
    bullet_hole_size: 0.3,
    bullet_hole_lifetime: 5.0,
    bullet_hole_color: [200, 200, 200, 255],

    crosshair_kick_per_shot: 2.5,
    crosshair_kick_decay: 8.0,
    crosshair_kick_max: 4.0,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: None,
    fsa_suppresses_kick: false,

    fire_sound_asset_key: "shotgun_shot",
    icon_asset_key: "shotgun_icon",
    icon_width: 100.0,
    icon_height: 50.0,
    icon_flip_x: true,

    viewmodel_model: Some("spas_model"),
    viewmodel_asset_key: Some("weapon_fp_shotgun"),

    viewmodel_scale: 0.002,
    viewmodel_ads_scale: 0.002,
    viewmodel_position: (0.22, 0.13, -0.44),
    viewmodel_rotation: (-0.1, -1.45, 0.0),
    viewmodel_muzzle_offset: (-1.1, 0.02, -0.25),
    viewmodel_hands_scale: 1.93,
    viewmodel_hands_offset: (0.18, -0.12, -0.466),
    viewmodel_hands_rotation: (-0.05, 0.9, 0.1),
    viewmodel_pair_offset_pos: (0.044, 0.0, 0.0),
    viewmodel_pair_offset_rot: (0.1, 0.0, 0.0),
    viewmodel_ads_pos_offset: (0.126, 0.025, -0.06),
    viewmodel_ads_rot_offset: (0.275, -0.12, 0.125),
    viewmodel_ads_hands_pos_offset: (0.134, 0.026, -0.537),
    viewmodel_ads_hands_rot_offset: (0.1, 0.0, 0.085),

    falloff_start: 5.0,
    falloff_end: 15.0,
    falloff_min_mult: 0.2,

    recoil_pattern: &[],
    pattern_randomness: 0.0,
    randomness_growth: 0.0,
    pattern_reset_time: 0.0,
};

/// Benelli — fast semi-auto shotgun, quick follow-up shots, weaker per-pellet
pub const BENELLI: WeaponStats = WeaponStats {
    name: "Benelli",
    damage: 10.0,
    headshot_mult: 1.5,
    pellet_count: 8,
    pellet_spread: Some(
        &[
            (0.0, 0.0), // center
            (0.0, 0.85), // ring
            (0.8976, 0.85),
            (1.7952, 0.85),
            (2.6928, 0.85),
            (3.5904, 0.85),
            (4.488, 0.85),
            (5.3856, 0.85),
        ]
    ),

    fire_rate: 2.5,
    magazine_size: 6,
    max_reserve: 30,
    reload_time: 1.85,

    recoil_vertical: 0.05,
    recoil_horizontal: 0.02,
    recoil_recovery: 3.0,

    hip_spread: 0.055,
    ads_spread: 0.03,
    spread_per_shot: 0.035,
    spread_recovery: 1.0,
    move_spread_mult: 1.3,
    sprint_spread_mult: 1.1,
    crouch_spread_mult: 0.85,

    ads_zoom_fov: 75.0,
    ads_time: 0.22,

    bullet_speed: 160.0,
    bullet_range: 18.0,
    tracer_length: 1.2,
    tracer_color: [255, 140, 60, 255],
    tracer_thickness: 0.008,
    tracer_lifetime: 0.6,

    crosshair_kick_per_shot: 2.0,
    crosshair_kick_decay: 9.0,
    crosshair_kick_max: 3.5,

    fire_sound_asset_key: "shotgun_shot",
    icon_asset_key: "shotgun_icon",

    viewmodel_model: Some("benelli_model"),
    viewmodel_asset_key: None,

    
    viewmodel_scale: 0.002,
    viewmodel_ads_scale: 0.002,
    viewmodel_position: (0.22, 0.13, -0.44),
    viewmodel_rotation: (-0.1, -1.45, 0.0),
    viewmodel_muzzle_offset: (-1.1, 0.02, -0.25),
    viewmodel_hands_scale: 1.93,
    viewmodel_hands_offset: (0.18, -0.12, -0.466),
    viewmodel_hands_rotation: (-0.05, 0.9, 0.1),
    viewmodel_pair_offset_pos: (0.044, 0.0, 0.0),
    viewmodel_pair_offset_rot: (0.1, 0.0, 0.0),
    viewmodel_ads_pos_offset: (0.126, 0.025, -0.06),
    viewmodel_ads_rot_offset: (0.275, -0.12, 0.125),
    viewmodel_ads_hands_pos_offset: (0.134, 0.026, -0.537),
    viewmodel_ads_hands_rot_offset: (0.1, 0.0, 0.085),

    falloff_start: 4.0,
    falloff_end: 12.0,
    falloff_min_mult: 0.15,

    ..SPAS
};

/// Sawnoff — double-barrel close-range shotgun, massive damage up close, slow fire rate.
/// Uses a unique hand animation model (fp_hands_sawnoff) that includes the gun,
/// similar to the knife.
pub const SAWNOFF: WeaponStats = WeaponStats {
    name: "Sawnoff",
    fire_mode: FireMode::SemiAuto,

    damage: 10.0,
    headshot_mult: 1.8,
    pellet_count: 10,
    pellet_spread: Some(
        &[
            (0.0, 0.0), // center
            (0.3927, 1.0), // wide ring — sawnoff has big spread
            (1.1781, 1.0),
            (1.9635, 1.0),
            (2.7489, 1.0),
            (3.5343, 1.0),
            (4.3197, 1.0),
            (5.1051, 1.0),
            (5.8905, 1.0),
            (0.7854, 0.5), // inner ring for density
        ]
    ),

    fire_rate: 5.0, // instant
    magazine_size: 2,
    max_reserve: 20,
    reload_time: 3.1,

    recoil_vertical: 0.12,
    recoil_horizontal: 0.05,
    recoil_recovery: 2.0,

    hip_spread: 0.08,
    ads_spread: 0.05,
    spread_per_shot: 0.06,
    spread_recovery: 0.6,
    move_spread_mult: 1.2,
    sprint_spread_mult: 1.1,
    crouch_spread_mult: 0.9,

    ads_zoom_fov: 80.0,
    ads_time: 0.2,
    ads_sensitivity: None,

    bullet_speed: 140.0,
    bullet_range: 12.0,
    tracer_length: 1.2,
    tracer_color: [255, 160, 50, 255],
    tracer_thickness: 0.009,
    tracer_lifetime: 0.6,
    bullet_hole_size: 0.35,
    bullet_hole_lifetime: 5.0,
    bullet_hole_color: [200, 200, 200, 255],

    crosshair_kick_per_shot: 3.0,
    crosshair_kick_decay: 6.0,
    crosshair_kick_max: 5.0,

    crosshair_style: CrosshairStyle::Standard,
    fsa_time: None,
    fsa_suppresses_kick: false,

    fire_sound_asset_key: "shotgun_shot",
    icon_asset_key: "shotgun_icon",
    icon_width: 90.0,
    icon_height: 45.0,
    icon_flip_x: false,

    // Sawnoff model includes hands — rendered as a single animated model
    // like the knife (fp_hands_knife). No separate hands drawn.
    viewmodel_model: Some("fp_hands_sawnoff"),
    viewmodel_asset_key: None,

    // Sawnoff viewmodel
    viewmodel_scale: 0.0025,
    viewmodel_ads_scale: 0.0025,
    viewmodel_position: (0.211, 0.108, -0.37),
    viewmodel_rotation: (-0.165, 0.09, 0.0),
    viewmodel_muzzle_offset: (0.0, 0.0, 0.0),
    viewmodel_hands_scale: 1.0,
    viewmodel_hands_offset: (0.0, 0.0, 0.0),
    viewmodel_hands_rotation: (0.0, 0.0, 0.0),
    viewmodel_pair_offset_pos: (0.0, 0.0, 0.0),
    viewmodel_pair_offset_rot: (0.0, 0.0, 0.0),
    viewmodel_ads_pos_offset: (0.14, 0.02, -0.1),
    viewmodel_ads_rot_offset: (0.01, -0.095, 0.0),
    viewmodel_ads_hands_pos_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_rot_offset: (0.0, 0.005, 0.0),

    // fall off: 
    falloff_start: 3.0,
    falloff_end: 10.0,
    falloff_min_mult: 0.15,

    recoil_pattern: &[],
    pattern_randomness: 0.0,
    randomness_growth: 0.0,
    pattern_reset_time: 0.0,
};
