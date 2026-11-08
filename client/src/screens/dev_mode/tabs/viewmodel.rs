use crate::ui::fonts::text_params;
use crate::weapons::state::WeaponInventory;
use macroquad::prelude::*;

use super::super::state::DevModeState;
use super::super::widgets::{
    draw_action_button, draw_section_header, draw_slider_nudge, draw_toggle,
};
use super::viewmodel_actions::draw_viewmodel_actions;
use super::viewmodel_ads::draw_viewmodel_ads_section;
use super::viewmodel_pair::draw_viewmodel_pair_section;

pub(in crate::screens::dev_mode) fn draw_viewmodel_tab(
    inv: &mut WeaponInventory,
    dev: &mut DevModeState,
    x: f32,
    mut y: f32,
    w: f32,
    mouse: (f32, f32),
    clicked: bool,
    held: bool,
) -> f32 {
    // step multiplier: precision mode shrinks all steps by 10x for micro-adjustments
    let sm = if dev.precision_mode { 0.1_f32 } else { 1.0_f32 };

    let weapon = inv.active_mut();
    let name = weapon.stats_data.name;
    draw_text_ex(
        &format!("Viewmodel: {}", name),
        x,
        y + 14.0,
        text_params(16, Color::from_rgba(255, 200, 80, 255)),
    );
    y += 24.0;

    // Precision mode toggle
    let prec_label = if dev.precision_mode {
        "Precision Mode  [ON]"
    } else {
        "Precision Mode [OFF]"
    };
    y += draw_toggle(prec_label, &mut dev.precision_mode, x, y, mouse, clicked);

    // ADS preview toggle — simulates ADS held without physically holding right-click
    let ads_label = if dev.force_ads {
        "ADS Preview  [ON]"
    } else {
        "ADS Preview [OFF]"
    };
    y += draw_toggle(ads_label, &mut dev.force_ads, x, y, mouse, clicked);

    // Viewport drag mode — drag directly on viewport to adjust position / rotation
    let drag_label = if dev.viewport_drag_active {
        "Drag Mode  [ON]"
    } else {
        "Drag Mode [OFF]"
    };
    y += draw_toggle(
        drag_label,
        &mut dev.viewport_drag_active,
        x,
        y,
        mouse,
        clicked,
    );
    y += 4.0;

    y += draw_section_header("Scale", x, y);
    y += draw_slider_nudge(
        "viewmodel_scale",
        &mut weapon.stats_data.viewmodel_scale,
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

    y += draw_section_header("Position", x, y);
    y += draw_slider_nudge(
        "pos_x (right)",
        &mut weapon.stats_data.viewmodel_position.0,
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
        "pos_y (up)",
        &mut weapon.stats_data.viewmodel_position.1,
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
        "pos_z (forward)",
        &mut weapon.stats_data.viewmodel_position.2,
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

    y += draw_section_header("Rotation (radians)", x, y);
    y += draw_slider_nudge(
        "pitch (up/down)",
        &mut weapon.stats_data.viewmodel_rotation.0,
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
        "yaw (left/right)",
        &mut weapon.stats_data.viewmodel_rotation.1,
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
        "roll (tilt)",
        &mut weapon.stats_data.viewmodel_rotation.2,
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

    y += draw_section_header("Muzzle Offset", x, y);
    y += draw_slider_nudge(
        "muzzle_x",
        &mut weapon.stats_data.viewmodel_muzzle_offset.0,
        -2.0,
        2.0,
        0.02 * sm,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider_nudge(
        "muzzle_y",
        &mut weapon.stats_data.viewmodel_muzzle_offset.1,
        -2.0,
        2.0,
        0.02 * sm,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider_nudge(
        "muzzle_z",
        &mut weapon.stats_data.viewmodel_muzzle_offset.2,
        -2.0,
        2.0,
        0.02 * sm,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );

    y += draw_section_header("FP Hands", x, y);
    y += draw_slider_nudge(
        "hands_scale",
        &mut weapon.stats_data.viewmodel_hands_scale,
        0.01,
        2.0,
        0.01 * sm,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    y += draw_slider_nudge(
        "hands_offset_x",
        &mut weapon.stats_data.viewmodel_hands_offset.0,
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
        "hands_offset_y",
        &mut weapon.stats_data.viewmodel_hands_offset.1,
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
        "hands_offset_z",
        &mut weapon.stats_data.viewmodel_hands_offset.2,
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
        "hands_rot_pitch",
        &mut weapon.stats_data.viewmodel_hands_rotation.0,
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
        "hands_rot_yaw",
        &mut weapon.stats_data.viewmodel_hands_rotation.1,
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
        "hands_rot_roll",
        &mut weapon.stats_data.viewmodel_hands_rotation.2,
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

    y = draw_viewmodel_pair_section(weapon, sm, x, y, w, mouse, clicked, held);

    y = draw_viewmodel_ads_section(weapon, sm, x, y, w, mouse, clicked, held);

    y = draw_viewmodel_actions(weapon, dev, x, y, mouse, clicked);
    // `weapon` borrow ends here — now safe to use `inv` directly for Copy From

    y += draw_section_header("Copy From Weapon", x, y);
    let active_slot = inv.active_slot;
    let mut copy_hip_from_slot: Option<usize> = None;
    let mut copy_ads_from_slot: Option<usize> = None;
    for i in 0..inv.weapons.len() {
        if i == active_slot {
            continue;
        }
        let wname: &'static str = inv.weapons[i].stats_data.name;
        let hip_label = format!("Copy Hip: {}", wname);
        let (h, hip_pressed) = draw_action_button(&hip_label, x, y, 220.0, mouse, clicked);
        y += h;
        if hip_pressed {
            copy_hip_from_slot = Some(i);
        }

        let ads_label = format!("Copy ADS: {}", wname);
        let (h, ads_pressed) = draw_action_button(&ads_label, x, y, 220.0, mouse, clicked);
        y += h;
        if ads_pressed {
            copy_ads_from_slot = Some(i);
        }
    }
    if let Some(src) = copy_hip_from_slot {
        let (scale, pos, rot, muz, hs, ho, hr, pp, pr) = {
            let sw = &inv.weapons[src];
            (
                sw.stats_data.viewmodel_scale,
                sw.stats_data.viewmodel_position,
                sw.stats_data.viewmodel_rotation,
                sw.stats_data.viewmodel_muzzle_offset,
                sw.stats_data.viewmodel_hands_scale,
                sw.stats_data.viewmodel_hands_offset,
                sw.stats_data.viewmodel_hands_rotation,
                sw.pair_offset_pos,
                sw.pair_offset_rot,
            )
        };
        let dst = inv.active_mut();
        dst.stats_data.viewmodel_scale = scale;
        dst.stats_data.viewmodel_position = pos;
        dst.stats_data.viewmodel_rotation = rot;
        dst.stats_data.viewmodel_muzzle_offset = muz;
        dst.stats_data.viewmodel_hands_scale = hs;
        dst.stats_data.viewmodel_hands_offset = ho;
        dst.stats_data.viewmodel_hands_rotation = hr;
        dst.pair_offset_pos = pp;
        dst.pair_offset_rot = pr;
    }
    if let Some(src) = copy_ads_from_slot {
        let (ads_scale, ap, ar, ahp, ahr) = {
            let sw = &inv.weapons[src];
            (
                sw.stats_data.viewmodel_ads_scale,
                sw.ads_pos_offset,
                sw.ads_rot_offset,
                sw.ads_hands_pos_offset,
                sw.ads_hands_rot_offset,
            )
        };
        let dst = inv.active_mut();
        dst.stats_data.viewmodel_ads_scale = ads_scale;
        dst.ads_pos_offset = ap;
        dst.ads_rot_offset = ar;
        dst.ads_hands_pos_offset = ahp;
        dst.ads_hands_rot_offset = ahr;
    }

    y
}
