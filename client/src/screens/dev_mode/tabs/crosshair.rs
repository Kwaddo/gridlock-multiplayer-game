use crate::ui::fonts::text_params;
use crate::weapons::crosshair::{CrosshairConfig, CrosshairType};
use crate::weapons::state::WeaponInventory;
use macroquad::prelude::*;

use super::super::widgets::{draw_action_button, draw_section_header, draw_slider, draw_toggle};

pub(in crate::screens::dev_mode) fn draw_crosshair_tab(
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
    y += draw_slider(
        "center_gap",
        &mut ch.center_gap,
        0.0,
        20.0,
        0.5,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "line_length",
        &mut ch.line_length,
        0.0,
        30.0,
        0.5,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "line_thickness",
        &mut ch.line_thickness,
        0.0,
        5.0,
        0.1,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_toggle("dot_enabled", &mut ch.dot_enabled, x, y, mouse, clicked);
    y += draw_slider(
        "dot_size",
        &mut ch.dot_size,
        0.0,
        8.0,
        0.05,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );

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
    y += draw_slider(
        "Red", &mut r, 0.0, 255.0, 5.0, x, y, w, mouse, clicked, held,
    );
    y += draw_slider(
        "Green", &mut g, 0.0, 255.0, 5.0, x, y, w, mouse, clicked, held,
    );
    y += draw_slider(
        "Blue", &mut b, 0.0, 255.0, 5.0, x, y, w, mouse, clicked, held,
    );
    ch.color = Color::from_rgba(r as u8, g as u8, b as u8, 255);

    y += draw_section_header("Outline", x, y);
    y += draw_toggle(
        "outline_enabled",
        &mut ch.outline_enabled,
        x,
        y,
        mouse,
        clicked,
    );
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
    let (_, pressed) =
        draw_action_button("Reset Crosshair to Defaults", x, y, 260.0, mouse, clicked);
    if pressed {
        inv.active_mut().crosshair = CrosshairConfig::for_category(cat);
    }
}
