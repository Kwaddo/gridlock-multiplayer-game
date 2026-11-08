use crate::physics::movement::MovementConfig;
use macroquad::prelude::*;

use super::super::widgets::{draw_section_header, draw_slider};

pub(in crate::screens::dev_mode) fn draw_movement_tab(
    mov: &mut MovementConfig,
    x: f32,
    mut y: f32,
    w: f32,
    mouse: (f32, f32),
    clicked: bool,
    held: bool,
) {
    y += draw_section_header("Speed", x, y);
    y += draw_slider(
        "walk_speed",
        &mut mov.walk_speed,
        1.0,
        15.0,
        0.5,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "sprint_speed",
        &mut mov.sprint_speed,
        2.0,
        25.0,
        0.5,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "crouch_speed",
        &mut mov.crouch_speed,
        0.5,
        8.0,
        0.5,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );

    y += draw_section_header("Physics", x, y);
    y += draw_slider(
        "acceleration",
        &mut mov.acceleration,
        1.0,
        30.0,
        1.0,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "friction",
        &mut mov.friction,
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
        "gravity",
        &mut mov.gravity,
        5.0,
        40.0,
        1.0,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider(
        "jump_strength",
        &mut mov.jump_strength,
        1.0,
        15.0,
        0.5,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );

    y += draw_section_header("Stamina", x, y);
    y += draw_slider(
        "stamina_drain",
        &mut mov.stamina_drain,
        5.0,
        100.0,
        5.0,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    draw_slider(
        "stamina_recover",
        &mut mov.stamina_recover,
        5.0,
        50.0,
        5.0,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
}
