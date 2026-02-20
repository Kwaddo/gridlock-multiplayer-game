use crate::physics::movement::MovementConfig;
use crate::ui::fonts::text_params;
use crate::weapons::crosshair::{CrosshairConfig, CrosshairType};
use crate::weapons::state::WeaponInventory;
use macroquad::prelude::*;

use super::state::{
    DevModeState, HandsAnimRequest, HANDS_DEFAULT_OFFSET_X, HANDS_DEFAULT_OFFSET_Y,
    HANDS_DEFAULT_OFFSET_Z, HANDS_DEFAULT_ROT_PITCH, HANDS_DEFAULT_ROT_ROLL,
    HANDS_DEFAULT_ROT_YAW, HANDS_DEFAULT_SCALE,
};
use super::widgets::{
    draw_action_button, draw_section_header, draw_slider, draw_slider_u32, draw_toggle,
};

pub(super) fn draw_weapon_tab(
    inv: &mut WeaponInventory,
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

pub(super) fn draw_crosshair_tab(
    inv: &mut WeaponInventory,
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
        &format!("Crosshair: {}", name),
        x,
        y + 14.0,
        text_params(16, Color::from_rgba(255, 200, 80, 255)),
    );
    y += 24.0;

    let ch = &mut weapon.crosshair;

    y += draw_section_header("Shape", x, y);
    y += draw_slider("center_gap", &mut ch.center_gap, 0.0, 20.0, 0.5, x, y, w, mouse, clicked, held);
    y += draw_slider("line_length", &mut ch.line_length, 0.0, 30.0, 0.5, x, y, w, mouse, clicked, held);
    y += draw_slider("line_thickness", &mut ch.line_thickness, 0.0, 5.0, 0.1, x, y, w, mouse, clicked, held);
    y += draw_toggle("dot_enabled", &mut ch.dot_enabled, x, y, mouse, clicked);
    y += draw_slider("dot_size", &mut ch.dot_size, 0.0, 8.0, 0.05, x, y, w, mouse, clicked, held);

    y += draw_section_header("Kick", x, y);
    y += draw_slider(
        "kick_per_shot",
        &mut weapon.stats_data.crosshair_kick_per_shot,
        0.0,
        10.0,
        0.5,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "kick_decay_rate",
        &mut weapon.stats_data.crosshair_kick_decay,
        1.0,
        20.0,
        1.0,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "kick_max",
        &mut weapon.stats_data.crosshair_kick_max,
        0.0,
        30.0,
        1.0,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );

    y += draw_section_header("Color (RGB)", x, y);
    let mut r = ch.color.r * 255.0;
    let mut g = ch.color.g * 255.0;
    let mut b = ch.color.b * 255.0;
    y += draw_slider("Red", &mut r, 0.0, 255.0, 5.0, x, y, w, mouse, clicked, held);
    y += draw_slider("Green", &mut g, 0.0, 255.0, 5.0, x, y, w, mouse, clicked, held);
    y += draw_slider("Blue", &mut b, 0.0, 255.0, 5.0, x, y, w, mouse, clicked, held);
    ch.color = Color::from_rgba(r as u8, g as u8, b as u8, 255);

    y += draw_section_header("Outline", x, y);
    y += draw_toggle("outline_enabled", &mut ch.outline_enabled, x, y, mouse, clicked);
    y += draw_slider(
        "outline_thickness",
        &mut ch.outline_thickness,
        0.0,
        3.0,
        0.1,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );

    let type_label = match ch.crosshair_type {
        CrosshairType::Cross => "Cross",
        CrosshairType::FourCurves => "4-Curves",
        CrosshairType::Chevron => "Chevron",
        CrosshairType::Circle => "Circle",
        CrosshairType::Square => "Square",
        CrosshairType::Triangle => "Triangle",
        CrosshairType::AngleBrackets => "Angle Brackets",
    };
    let (btn_h, pressed) = draw_action_button(
        &format!("Type: {}", type_label),
        x,
        y,
        200.0,
        mouse,
        clicked,
    );
    if pressed {
        ch.crosshair_type = match ch.crosshair_type {
            CrosshairType::Cross => CrosshairType::FourCurves,
            CrosshairType::FourCurves => CrosshairType::Chevron,
            CrosshairType::Chevron => CrosshairType::Circle,
            CrosshairType::Circle => CrosshairType::Square,
            CrosshairType::Square => CrosshairType::Triangle,
            CrosshairType::Triangle => CrosshairType::AngleBrackets,
            CrosshairType::AngleBrackets => CrosshairType::Cross,
        };
    }
    y += btn_h;

    y += 8.0;
    let cat = weapon.id.category();
    let (_, pressed) = draw_action_button("Reset Crosshair to Defaults", x, y, 260.0, mouse, clicked);
    if pressed {
        inv.active_mut().crosshair = CrosshairConfig::for_category(cat);
    }
}

pub(super) fn draw_movement_tab(
    mov: &mut MovementConfig,
    x: f32,
    mut y: f32,
    w: f32,
    mouse: (f32, f32),
    clicked: bool,
    held: bool,
) {
    y += draw_section_header("Speed", x, y);
    y += draw_slider("walk_speed", &mut mov.walk_speed, 1.0, 15.0, 0.5, x, y, w, mouse, clicked, held);
    y += draw_slider("sprint_speed", &mut mov.sprint_speed, 2.0, 25.0, 0.5, x, y, w, mouse, clicked, held);
    y += draw_slider("crouch_speed", &mut mov.crouch_speed, 0.5, 8.0, 0.5, x, y, w, mouse, clicked, held);

    y += draw_section_header("Physics", x, y);
    y += draw_slider("acceleration", &mut mov.acceleration, 1.0, 30.0, 1.0, x, y, w, mouse, clicked, held);
    y += draw_slider("friction", &mut mov.friction, 1.0, 20.0, 1.0, x, y, w, mouse, clicked, held);
    y += draw_slider("gravity", &mut mov.gravity, 5.0, 40.0, 1.0, x, y, w, mouse, clicked, held);
    y += draw_slider("jump_strength", &mut mov.jump_strength, 1.0, 15.0, 0.5, x, y, w, mouse, clicked, held);

    y += draw_section_header("Stamina", x, y);
    y += draw_slider("stamina_drain", &mut mov.stamina_drain, 5.0, 100.0, 5.0, x, y, w, mouse, clicked, held);
    draw_slider("stamina_recover", &mut mov.stamina_recover, 5.0, 50.0, 5.0, x, y, w, mouse, clicked, held);
}

pub(super) fn draw_viewmodel_tab(
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
        &format!("Viewmodel: {}", name),
        x,
        y + 14.0,
        text_params(16, Color::from_rgba(255, 200, 80, 255)),
    );
    y += 24.0;

    y += draw_section_header("Scale", x, y);
    y += draw_slider("viewmodel_scale", &mut weapon.stats_data.viewmodel_scale, 0.001, 0.1, 0.001, x, y, w, mouse, clicked, held);

    y += draw_section_header("Position", x, y);
    y += draw_slider("pos_x (right)", &mut weapon.stats_data.viewmodel_position.0, -0.5, 0.5, 0.01, x, y, w, mouse, clicked, held);
    y += draw_slider("pos_y (up)", &mut weapon.stats_data.viewmodel_position.1, -0.5, 0.5, 0.01, x, y, w, mouse, clicked, held);
    y += draw_slider("pos_z (forward)", &mut weapon.stats_data.viewmodel_position.2, -0.5, 0.5, 0.01, x, y, w, mouse, clicked, held);

    y += draw_section_header("Rotation (radians)", x, y);
    y += draw_slider("pitch (up/down)", &mut weapon.stats_data.viewmodel_rotation.0, -3.15, 3.15, 0.05, x, y, w, mouse, clicked, held);
    y += draw_slider("yaw (left/right)", &mut weapon.stats_data.viewmodel_rotation.1, -3.15, 3.15, 0.05, x, y, w, mouse, clicked, held);
    y += draw_slider("roll (tilt)", &mut weapon.stats_data.viewmodel_rotation.2, -3.15, 3.15, 0.05, x, y, w, mouse, clicked, held);

    y += draw_section_header("Muzzle Offset", x, y);
    y += draw_slider("muzzle_x", &mut weapon.stats_data.viewmodel_muzzle_offset.0, -2.0, 2.0, 0.02, x, y, w, mouse, clicked, held);
    y += draw_slider("muzzle_y", &mut weapon.stats_data.viewmodel_muzzle_offset.1, -2.0, 2.0, 0.02, x, y, w, mouse, clicked, held);
    y += draw_slider("muzzle_z", &mut weapon.stats_data.viewmodel_muzzle_offset.2, -2.0, 2.0, 0.02, x, y, w, mouse, clicked, held);

    y += draw_section_header("Per-Weapon Hands Offset", x, y);
    y += draw_slider("hands_off_x", &mut weapon.stats_data.viewmodel_hands_offset.0, -0.5, 0.5, 0.01, x, y, w, mouse, clicked, held);
    y += draw_slider("hands_off_y", &mut weapon.stats_data.viewmodel_hands_offset.1, -0.5, 0.5, 0.01, x, y, w, mouse, clicked, held);
    y += draw_slider("hands_off_z", &mut weapon.stats_data.viewmodel_hands_offset.2, -0.5, 0.5, 0.01, x, y, w, mouse, clicked, held);

    y += draw_section_header("FP Hands", x, y);
    y += draw_slider("hands_scale", &mut dev.hands_scale, 0.01, 2.0, 0.01, x, y, w, mouse, clicked, held);
    y += draw_slider("hands_offset_x", &mut dev.hands_offset_x, -1.0, 1.0, 0.01, x, y, w, mouse, clicked, held);
    y += draw_slider("hands_offset_y", &mut dev.hands_offset_y, -1.0, 1.0, 0.01, x, y, w, mouse, clicked, held);
    y += draw_slider("hands_offset_z", &mut dev.hands_offset_z, -1.0, 1.0, 0.01, x, y, w, mouse, clicked, held);
    y += draw_slider("hands_rot_pitch", &mut dev.hands_rot_pitch, -3.15, 3.15, 0.05, x, y, w, mouse, clicked, held);
    y += draw_slider("hands_rot_yaw", &mut dev.hands_rot_yaw, -3.15, 3.15, 0.05, x, y, w, mouse, clicked, held);
    y += draw_slider("hands_rot_roll", &mut dev.hands_rot_roll, -3.15, 3.15, 0.05, x, y, w, mouse, clicked, held);

    y += draw_section_header("Hands Animation", x, y);
    let (h, idle_pressed) = draw_action_button("Play Idle (Loop)", x, y, 170.0, mouse, clicked);
    y += h;
    if idle_pressed {
        dev.hands_anim_request = Some(HandsAnimRequest::IdleLoop);
    }
    let (h, reload_pressed) = draw_action_button("Play Reload (Once)", x, y, 180.0, mouse, clicked);
    y += h;
    if reload_pressed {
        dev.hands_anim_request = Some(HandsAnimRequest::ReloadOnce);
    }
    let (h, stop_pressed) = draw_action_button("Stop Hands", x, y, 120.0, mouse, clicked);
    y += h;
    if stop_pressed {
        dev.hands_anim_request = Some(HandsAnimRequest::Stop);
    }

    y += 8.0;
    let (_, pressed) = draw_action_button("Reset Viewmodel to Defaults", x, y, 260.0, mouse, clicked);
    if pressed {
        let defaults = weapon.id.stats();
        weapon.stats_data.viewmodel_scale = defaults.viewmodel_scale;
        weapon.stats_data.viewmodel_position = defaults.viewmodel_position;
        weapon.stats_data.viewmodel_rotation = defaults.viewmodel_rotation;
        weapon.stats_data.viewmodel_muzzle_offset = defaults.viewmodel_muzzle_offset;
        weapon.stats_data.viewmodel_hands_offset = defaults.viewmodel_hands_offset;
        dev.hands_scale = HANDS_DEFAULT_SCALE;
        dev.hands_offset_x = HANDS_DEFAULT_OFFSET_X;
        dev.hands_offset_y = HANDS_DEFAULT_OFFSET_Y;
        dev.hands_offset_z = HANDS_DEFAULT_OFFSET_Z;
        dev.hands_rot_pitch = HANDS_DEFAULT_ROT_PITCH;
        dev.hands_rot_yaw = HANDS_DEFAULT_ROT_YAW;
        dev.hands_rot_roll = HANDS_DEFAULT_ROT_ROLL;
        dev.hands_anim_request = Some(HandsAnimRequest::IdleLoop);
    }
}

pub(super) fn draw_debug_tab(
    dev: &mut DevModeState,
    inv: &mut WeaponInventory,
    x: f32,
    mut y: f32,
    mouse: (f32, f32),
    clicked: bool,
) {
    y += draw_section_header("Toggles", x, y);
    y += draw_toggle("Infinite Ammo", &mut dev.infinite_ammo, x, y, mouse, clicked);
    y += draw_toggle("No Recoil", &mut dev.no_recoil, x, y, mouse, clicked);
    y += draw_toggle("God Mode", &mut dev.god_mode, x, y, mouse, clicked);
    y += draw_toggle("Show Hitboxes", &mut dev.show_hitboxes, x, y, mouse, clicked);
    y += draw_toggle("Show Health Bars", &mut dev.show_health_bars, x, y, mouse, clicked);

    y += draw_section_header("Actions", x, y);

    let (h, pressed) = draw_action_button("Refill Ammo", x, y, 160.0, mouse, clicked);
    y += h;
    if pressed {
        let w = inv.active_mut();
        w.ammo = w.stats_data.magazine_size;
        w.reserve = w.stats_data.max_reserve;
    }

    let (_, pressed) = draw_action_button("Reset All Weapons", x, y, 180.0, mouse, clicked);
    if pressed {
        for w in &mut inv.weapons {
            w.reset_stats();
            w.crosshair = CrosshairConfig::for_category(w.id.category());
        }
    }
}
