use crate::weapons::WeaponState;
use macroquad::prelude::*;

use super::super::widgets::{
    draw_action_button, draw_section_header, draw_slider_nudge, draw_toggle,
};

pub(super) fn draw_viewmodel_pair_section(
    weapon: &mut WeaponState,
    sm: f32,
    x: f32,
    mut y: f32,
    w: f32,
    mouse: (f32, f32),
    clicked: bool,
    held: bool,
) -> f32 {
    y += draw_section_header("Hand + Gun Pair", x, y);
    y += draw_toggle(
        "Lock Together",
        &mut weapon.pair_locked,
        x,
        y,
        mouse,
        clicked,
    );
    if weapon.pair_locked {
        y += draw_section_header("Pair Position", x, y);
        y += draw_slider_nudge(
            "pair_pos_x (right)",
            &mut weapon.pair_offset_pos.0,
            -0.5,
            0.5,
            0.01 * sm,
            x,
            y,
            w,
            mouse,
            clicked,
            held,
        );
        y += draw_slider_nudge(
            "pair_pos_y (up)",
            &mut weapon.pair_offset_pos.1,
            -0.5,
            0.5,
            0.01 * sm,
            x,
            y,
            w,
            mouse,
            clicked,
            held,
        );
        y += draw_slider_nudge(
            "pair_pos_z (forward)",
            &mut weapon.pair_offset_pos.2,
            -0.5,
            0.5,
            0.01 * sm,
            x,
            y,
            w,
            mouse,
            clicked,
            held,
        );
        y += draw_section_header("Pair Rotation (radians)", x, y);
        y += draw_slider_nudge(
            "pair_pitch",
            &mut weapon.pair_offset_rot.0,
            -3.15,
            3.15,
            0.05 * sm,
            x,
            y,
            w,
            mouse,
            clicked,
            held,
        );
        y += draw_slider_nudge(
            "pair_yaw",
            &mut weapon.pair_offset_rot.1,
            -3.15,
            3.15,
            0.05 * sm,
            x,
            y,
            w,
            mouse,
            clicked,
            held,
        );
        y += draw_slider_nudge(
            "pair_roll",
            &mut weapon.pair_offset_rot.2,
            -3.15,
            3.15,
            0.05 * sm,
            x,
            y,
            w,
            mouse,
            clicked,
            held,
        );
        let (h, pair_reset) = draw_action_button("Reset Pair", x, y, 120.0, mouse, clicked);
        y += h;
        if pair_reset {
            weapon.pair_offset_pos = (0.0, 0.0, 0.0);
            weapon.pair_offset_rot = (0.0, 0.0, 0.0);
        }
    }

    y
}
