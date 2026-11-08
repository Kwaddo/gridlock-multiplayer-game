use crate::weapons::WeaponState;
use macroquad::prelude::*;

use super::super::widgets::{
    draw_action_button, draw_section_header, draw_slider_nudge, draw_toggle,
};

pub(super) fn draw_viewmodel_ads_section(
    weapon: &mut WeaponState,
    sm: f32,
    x: f32,
    mut y: f32,
    w: f32,
    mouse: (f32, f32),
    clicked: bool,
    held: bool,
) -> f32 {
    y += draw_section_header("ADS Viewmodel", x, y);
    y += draw_toggle(
        "Lock Gun + Hands (ADS)",
        &mut weapon.ads_pair_locked,
        x,
        y,
        mouse,
        clicked,
    );
    y += draw_slider_nudge(
        "viewmodel_ads_scale",
        &mut weapon.stats_data.viewmodel_ads_scale,
        0.001,
        0.1,
        0.001 * sm,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );

    if weapon.ads_pair_locked {
        // Locked: one set of sliders drives both gun and hands together
        let mut ads_pair_pos_x = weapon.ads_pos_offset.0;
        let mut ads_pair_pos_y = weapon.ads_pos_offset.1;
        let mut ads_pair_pos_z = weapon.ads_pos_offset.2;
        let mut ads_pair_rot_p = weapon.ads_rot_offset.0;
        let mut ads_pair_rot_y = weapon.ads_rot_offset.1;
        let mut ads_pair_rot_r = weapon.ads_rot_offset.2;

        y += draw_section_header("ADS Pair Position", x, y);
        y += draw_slider_nudge(
            "ads_pos_x (right)",
            &mut ads_pair_pos_x,
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
            "ads_pos_y (up)",
            &mut ads_pair_pos_y,
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
            "ads_pos_z (forward)",
            &mut ads_pair_pos_z,
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
        y += draw_section_header("ADS Pair Rotation", x, y);
        y += draw_slider_nudge(
            "ads_pitch",
            &mut ads_pair_rot_p,
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
            "ads_yaw",
            &mut ads_pair_rot_y,
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
            "ads_roll",
            &mut ads_pair_rot_r,
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

        // Apply to both gun and hands
        weapon.ads_pos_offset.0 = ads_pair_pos_x;
        weapon.ads_pos_offset.1 = ads_pair_pos_y;
        weapon.ads_pos_offset.2 = ads_pair_pos_z;
        weapon.ads_hands_pos_offset.0 = ads_pair_pos_x;
        weapon.ads_hands_pos_offset.1 = ads_pair_pos_y;
        weapon.ads_hands_pos_offset.2 = ads_pair_pos_z;
        weapon.ads_rot_offset.0 = ads_pair_rot_p;
        weapon.ads_rot_offset.1 = ads_pair_rot_y;
        weapon.ads_rot_offset.2 = ads_pair_rot_r;
        weapon.ads_hands_rot_offset.0 = ads_pair_rot_p;
        weapon.ads_hands_rot_offset.1 = ads_pair_rot_y;
        weapon.ads_hands_rot_offset.2 = ads_pair_rot_r;
    } else {
        // Unlocked: adjust gun and hands independently
        y += draw_slider_nudge(
            "ads_pos_x (right)",
            &mut weapon.ads_pos_offset.0,
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
            "ads_pos_y (up)",
            &mut weapon.ads_pos_offset.1,
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
            "ads_pos_z (forward)",
            &mut weapon.ads_pos_offset.2,
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
            "ads_pitch",
            &mut weapon.ads_rot_offset.0,
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
            "ads_yaw",
            &mut weapon.ads_rot_offset.1,
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
            "ads_roll",
            &mut weapon.ads_rot_offset.2,
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
        y += draw_section_header("ADS Hands", x, y);
        y += draw_slider_nudge(
            "ads_hands_x",
            &mut weapon.ads_hands_pos_offset.0,
            -1.0,
            1.0,
            0.01 * sm,
            x,
            y,
            w,
            mouse,
            clicked,
            held,
        );
        y += draw_slider_nudge(
            "ads_hands_y",
            &mut weapon.ads_hands_pos_offset.1,
            -1.0,
            1.0,
            0.01 * sm,
            x,
            y,
            w,
            mouse,
            clicked,
            held,
        );
        y += draw_slider_nudge(
            "ads_hands_z",
            &mut weapon.ads_hands_pos_offset.2,
            -1.0,
            1.0,
            0.01 * sm,
            x,
            y,
            w,
            mouse,
            clicked,
            held,
        );
        y += draw_slider_nudge(
            "ads_hands_pitch",
            &mut weapon.ads_hands_rot_offset.0,
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
            "ads_hands_yaw",
            &mut weapon.ads_hands_rot_offset.1,
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
            "ads_hands_roll",
            &mut weapon.ads_hands_rot_offset.2,
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
    }

    let (h, ads_reset) = draw_action_button("Reset ADS to Defaults", x, y, 200.0, mouse, clicked);
    y += h;
    if ads_reset {
        let defaults = weapon.id.stats();
        weapon.stats_data.viewmodel_ads_scale = defaults.viewmodel_ads_scale;
        weapon.ads_pos_offset = defaults.viewmodel_ads_pos_offset;
        weapon.ads_rot_offset = defaults.viewmodel_ads_rot_offset;
        weapon.ads_hands_pos_offset = defaults.viewmodel_ads_hands_pos_offset;
        weapon.ads_hands_rot_offset = defaults.viewmodel_ads_hands_rot_offset;
    }

    y
}
