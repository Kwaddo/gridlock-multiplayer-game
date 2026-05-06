use super::{ CrosshairStyle, FireMode, WeaponStats };

/// Punch — dedicated melee action, no ammo, no reload.
/// Bound to a dedicated key (default G). Fires instantly and auto-returns to gun.
/// All gun-specific fields are zeroed since melee doesn't use the hitscan pipeline.

pub const PUNCH_DAMAGE: f32 = 43.75; // 35% of base player HP (125)
pub const KNIFE_MISS_PENALTY_SECS: f32 = 0.25;
pub const KNIFE_MISS_MOVE_SLOW: f32 = 0.6;
pub const KNIFE_MISS_MOVE_SLOW_DURATION: f32 = 0.5;
pub const KNIFE_SPEED_BONUS: f32 = 1.15;
pub const KNIFE_SWITCH_TO_TIME: f32 = 0.15;
pub const KNIFE_SWITCH_FROM_TIME: f32 = 0.40;
pub const KNIFE_LIGHT_ANIM_DURATION: f32 = 0.66;


pub const KNIFE: WeaponStats = WeaponStats {
    name: "Punch",
    fire_mode: FireMode::SemiAuto,

    // --- Damage (35% of base HP — no heavy attack)
    damage: 43.75,
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

    // Punch uses 2D sprite frames (punch1/punch2/punch3 cycled by viewmodel renderer).
    // No 3D model — the frame is selected based on knife_attack_lock_timer in draw_viewmodel.
    viewmodel_model: None,
    viewmodel_asset_key: Some("punch1"),

    viewmodel_scale: 1.0,
    viewmodel_ads_scale: 1.0,
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
