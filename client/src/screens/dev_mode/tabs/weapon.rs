use crate::ui::fonts::text_params;
use crate::weapons::state::WeaponInventory;
use macroquad::prelude::*;

use super::super::state::DevModeState;
use super::super::widgets::{
    draw_action_button, draw_section_header, draw_slider, draw_slider_u32,
};

pub(in crate::screens::dev_mode) fn draw_weapon_tab(
    inv: &mut WeaponInventory,
    dev: &mut DevModeState,
    x: f32,
    mut y: f32,
    w: f32,
    mouse: (f32, f32),
    clicked: bool,
    held: bool,
) {
    let weapon = inv.active_mut();
    let name = weapon.stats_data.name;
    draw_text_ex(
        &format!("Active: {}", name),
        x,
        y + 14.0,
        text_params(16, Color::from_rgba(255, 200, 80, 255)),
    );
    y += 24.0;

    // world weapon adjustments
    y += draw_section_header("World Weapon Position", x, y);
    y += draw_slider(
        "pos_x",
        &mut dev.weapon_pos_offset.0,
        -1.0,
        1.0,
        0.01,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "pos_y",
        &mut dev.weapon_pos_offset.1,
        -1.0,
        1.0,
        0.01,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "pos_z",
        &mut dev.weapon_pos_offset.2,
        -1.0,
        1.0,
        0.01,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_section_header("World Weapon Rotation", x, y);
    y += draw_slider(
        "rot_pitch",
        &mut dev.weapon_rot_offset.0,
        -3.15,
        3.15,
        0.05,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "rot_yaw",
        &mut dev.weapon_rot_offset.1,
        -3.15,
        3.15,
        0.05,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    // adjust yaw during movement (running/walking)
    y += draw_slider(
        "yaw_move",
        &mut dev.weapon_yaw_adjust_move,
        -3.15,
        3.15,
        0.01,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    // adjust yaw when aiming down sights
    y += draw_slider(
        "yaw_ads",
        &mut dev.weapon_yaw_adjust_ads,
        -3.15,
        3.15,
        0.01,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "rot_roll",
        &mut dev.weapon_rot_offset.2,
        -3.15,
        3.15,
        0.05,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );

    y += draw_section_header("Damage", x, y);
    y += draw_slider(
        "damage",
        &mut weapon.stats_data.damage,
        1.0,
        200.0,
        1.0,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "headshot_mult",
        &mut weapon.stats_data.headshot_mult,
        1.0,
        5.0,
        0.1,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );

    y += draw_section_header("Fire", x, y);
    y += draw_slider(
        "fire_rate",
        &mut weapon.stats_data.fire_rate,
        0.5,
        20.0,
        0.5,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider_u32(
        "pellet_count",
        &mut weapon.stats_data.pellet_count,
        1,
        20,
        1,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );

    y += draw_section_header("Ammo", x, y);
    y += draw_slider_u32(
        "magazine_size",
        &mut weapon.stats_data.magazine_size,
        1,
        100,
        1,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider_u32(
        "max_reserve",
        &mut weapon.stats_data.max_reserve,
        0,
        500,
        10,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "reload_time",
        &mut weapon.stats_data.reload_time,
        0.1,
        5.0,
        0.1,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );

    y += draw_section_header("Recoil", x, y);
    y += draw_slider(
        "recoil_vertical",
        &mut weapon.stats_data.recoil_vertical,
        0.0,
        0.2,
        0.005,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "recoil_horizontal",
        &mut weapon.stats_data.recoil_horizontal,
        0.0,
        0.1,
        0.002,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "recoil_recovery",
        &mut weapon.stats_data.recoil_recovery,
        0.5,
        10.0,
        0.5,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );

    y += draw_section_header("Spread", x, y);
    y += draw_slider(
        "hip_spread",
        &mut weapon.stats_data.hip_spread,
        0.0,
        0.2,
        0.005,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "ads_spread",
        &mut weapon.stats_data.ads_spread,
        0.0,
        0.1,
        0.001,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "spread_per_shot",
        &mut weapon.stats_data.spread_per_shot,
        0.0,
        0.1,
        0.005,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "spread_recovery",
        &mut weapon.stats_data.spread_recovery,
        0.1,
        5.0,
        0.1,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "move_spread_mult",
        &mut weapon.stats_data.move_spread_mult,
        1.0,
        3.0,
        0.1,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "sprint_spread_mult",
        &mut weapon.stats_data.sprint_spread_mult,
        1.0,
        5.0,
        0.1,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "crouch_spread_mult",
        &mut weapon.stats_data.crouch_spread_mult,
        0.1,
        1.0,
        0.05,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );

    y += draw_section_header("ADS", x, y);
    y += draw_slider(
        "ads_zoom_fov",
        &mut weapon.stats_data.ads_zoom_fov,
        10.0,
        90.0,
        1.0,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "ads_time",
        &mut weapon.stats_data.ads_time,
        0.01,
        1.0,
        0.01,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );

    y += draw_section_header("Bullet", x, y);
    y += draw_slider(
        "bullet_range",
        &mut weapon.stats_data.bullet_range,
        5.0,
        200.0,
        5.0,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "tracer_length",
        &mut weapon.stats_data.tracer_length,
        0.5,
        15.0,
        0.5,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );

    y += draw_section_header("Falloff", x, y);
    y += draw_slider(
        "falloff_start",
        &mut weapon.stats_data.falloff_start,
        1.0,
        100.0,
        1.0,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "falloff_end",
        &mut weapon.stats_data.falloff_end,
        5.0,
        200.0,
        5.0,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "falloff_min_mult",
        &mut weapon.stats_data.falloff_min_mult,
        0.0,
        1.0,
        0.05,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );

    y += 8.0;
    let (_, pressed) = draw_action_button("Reset Weapon to Defaults", x, y, 240.0, mouse, clicked);
    if pressed {
        inv.active_mut().reset_stats();
    }
}
