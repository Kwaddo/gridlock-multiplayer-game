use crate::weapons::stats::WeaponStats;

use super::revolvers::TAURUS_RAGING_SHOTGUN_PELLET_SPREAD;

pub fn apply_taurus_raging_pistol_mode(stats: &mut WeaponStats) {
    stats.damage = 34.0;
    stats.headshot_mult = 2.0;
    stats.pellet_count = 1;
    stats.pellet_spread = None;
    stats.magazine_size = 5;
    stats.fire_rate = 2.2;
    stats.recoil_vertical = 0.035;
    stats.recoil_horizontal = 0.012;
    stats.recoil_recovery = 3.5;
    stats.hip_spread = 0.028;
    stats.ads_spread = 0.006;
    stats.spread_per_shot = 0.02;
    stats.spread_recovery = 1.8;
    stats.ads_zoom_fov = 65.0;
    stats.bullet_speed = 210.0;
    stats.bullet_range = 45.0;
    stats.tracer_length = 2.2;
    stats.tracer_color = [255, 190, 70, 255];
    stats.tracer_thickness = 0.012;
    stats.tracer_lifetime = 1.3;
    stats.bullet_hole_size = 0.22;
    stats.crosshair_kick_per_shot = 1.7;
    stats.crosshair_kick_decay = 6.0;
    stats.crosshair_kick_max = 2.3;
    stats.fire_sound_asset_key = "revolver_shot";
    stats.falloff_start = 20.0;
    stats.falloff_end = 45.0;
    stats.falloff_min_mult = 0.55;
    stats.recoil_pattern = &[
        (0.035, 0.0),
        (0.030, 0.1),
        (0.028, -0.1),
        (0.025, 0.15),
        (0.022, -0.15),
    ];
    stats.pattern_randomness = 0.2;
    stats.randomness_growth = 0.1;
    stats.pattern_reset_time = 0.8;
}

pub fn apply_taurus_raging_shotgun_mode(stats: &mut WeaponStats) {
    stats.damage = 14.0;
    stats.headshot_mult = 1.8;
    stats.magazine_size = 1;
    stats.pellet_count = 5;
    stats.pellet_spread = Some(TAURUS_RAGING_SHOTGUN_PELLET_SPREAD);
    stats.fire_rate = 1.4;
    stats.recoil_vertical = 0.06;
    stats.recoil_horizontal = 0.03;
    stats.recoil_recovery = 2.5;
    stats.hip_spread = 0.05;
    stats.ads_spread = 0.025;
    stats.spread_per_shot = 0.04;
    stats.spread_recovery = 1.0;
    stats.ads_zoom_fov = 70.0;
    stats.bullet_speed = 150.0;
    stats.bullet_range = 15.0;
    stats.tracer_length = 1.5;
    stats.tracer_color = [255, 160, 50, 255];
    stats.tracer_thickness = 0.009;
    stats.tracer_lifetime = 0.7;
    stats.bullet_hole_size = 0.25;
    stats.crosshair_kick_per_shot = 2.5;
    stats.crosshair_kick_decay = 6.0;
    stats.crosshair_kick_max = 4.0;
    stats.fire_sound_asset_key = "shotgun_shot";
    stats.falloff_start = 4.0;
    stats.falloff_end = 12.0;
    stats.falloff_min_mult = 0.2;
    stats.recoil_pattern = &[
        (0.06, 0.0),
        (0.05, 0.1),
        (0.045, -0.1),
        (0.04, 0.08),
        (0.035, -0.08),
    ];
    stats.pattern_randomness = 0.2;
    stats.randomness_growth = 0.1;
    stats.pattern_reset_time = 0.8;
}
