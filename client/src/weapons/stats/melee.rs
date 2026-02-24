use super::{ CrosshairStyle, FireMode, WeaponStats };

/// Knife — fast melee weapon, no ammo, no reload.
/// Left click = quick slash (cycles 3 swings), right click = heavy stab (one-shot).
/// All gun-specific fields (bullet, tracer, spread, etc.) are zeroed out since
/// melee doesn't use the hitscan/projectile pipeline.
pub const KNIFE: WeaponStats = WeaponStats {
    name: "Knife",
    fire_mode: FireMode::SemiAuto,

    // --- Damage (light slash — heavy stab is handled separately in simulation)
    damage: 40.0,
    headshot_mult: 1.0, // melee doesn't headshot
    pellet_count: 1,
    pellet_spread: None,

    // --- Fire rate & ammo (no ammo system)
    fire_rate: 3.0, // ~0.33s between light slashes
    magazine_size: u32::MAX, // infinite
    max_reserve: 0,
    reload_time: 0.0, // no reload

    // --- Recoil (none)
    recoil_vertical: 0.0,
    recoil_horizontal: 0.0,
    recoil_recovery: 0.0,

    // --- Spread / bloom (none — melee is a short-range box check)
    hip_spread: 0.0,
    ads_spread: 0.0,
    spread_per_shot: 0.0,
    spread_recovery: 0.0,
    move_spread_mult: 1.0,
    sprint_spread_mult: 1.0,
    crouch_spread_mult: 1.0,

    // --- ADS (knife doesn't aim)
    ads_zoom_fov: 90.0, // no zoom
    ads_time: 0.1, // non-zero to avoid division by zero in ADS lerp
    ads_sensitivity: None,

    // --- Bullet visual (none — melee has no tracers)
    bullet_speed: 0.0,
    bullet_range: 2.5, // melee reach distance (units)
    tracer_length: 0.0,
    tracer_color: [0, 0, 0, 0],
    tracer_thickness: 0.0,
    tracer_lifetime: 0.0,
    bullet_hole_size: 0.0,
    bullet_hole_lifetime: 0.0,
    bullet_hole_color: [0, 0, 0, 0],

    // --- Crosshair kick (none)
    crosshair_kick_per_shot: 0.0,
    crosshair_kick_decay: 0.0,
    crosshair_kick_max: 0.0,

    // --- Crosshair
    crosshair_style: CrosshairStyle::Standard, // small dot/cross for melee aiming
    fsa_time: None,
    fsa_suppresses_kick: false,

    // --- HUD icon
    fire_sound_asset_key: "knife_slash", // TODO: add knife sound to manifest
    icon_asset_key: "knife_icon", // TODO: add knife icon to manifest
    icon_width: 80.0,
    icon_height: 40.0,
    icon_flip_x: false,

    // --- Viewmodel: knife replaces hands entirely (drawn as fp_hands_knife)
    // viewmodel_model is None because the knife isn't drawn as a separate weapon
    // on top of hands — it IS the hands model.
    viewmodel_model: Some("fp_hands_knife"),
    viewmodel_asset_key: None,

    // draw_3d_viewmodel multiplies by 28.0; this keeps knife close to previous size.
    viewmodel_scale: 0.0035,
    viewmodel_ads_scale: 0.0035,
    viewmodel_position: (0.345, 0.1, -0.37),
    viewmodel_rotation: (-0.15, 0.0, 0.0),
    viewmodel_muzzle_offset: (0.0, 0.0, 0.0),
    viewmodel_hands_scale: 1.0,
    viewmodel_hands_offset: (-0.03, 0.0, 0.0),
    viewmodel_hands_rotation: (0.0, 0.0, 0.0),
    viewmodel_pair_offset_pos: (0.0, 0.0, 0.0),
    viewmodel_pair_offset_rot: (0.0, 0.0, 0.0),
    viewmodel_ads_pos_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_rot_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_pos_offset: (0.0, 0.0, 0.0),
    viewmodel_ads_hands_rot_offset: (0.0, 0.0, 0.0),

    // --- Damage falloff (melee has fixed range, no falloff)
    falloff_start: 100.0,
    falloff_end: 100.0,
    falloff_min_mult: 1.0,

    // --- Spray pattern (none)
    recoil_pattern: &[(0.0, 0.0)],
    pattern_randomness: 0.0,
    randomness_growth: 0.0,
    pattern_reset_time: 0.0,
};
