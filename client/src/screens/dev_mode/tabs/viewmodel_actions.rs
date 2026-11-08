use crate::weapons::WeaponState;
use macroquad::prelude::*;

use super::super::state::{DevModeState, HandsAnimRequest};
use super::super::widgets::{draw_action_button, draw_section_header};

pub(super) fn draw_viewmodel_actions(
    weapon: &mut WeaponState,
    dev: &mut DevModeState,
    x: f32,
    mut y: f32,
    mouse: (f32, f32),
    clicked: bool,
) -> f32 {
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
    let (h, copy_pressed) = draw_action_button("Copy to Code", x, y, 140.0, mouse, clicked);
    y += h;
    if copy_pressed {
        let s = &weapon.stats_data;
        println!(
            "// {} viewmodel \n\
             viewmodel_scale: {:.4},\n\
             viewmodel_ads_scale: {:.4},\n\
             viewmodel_position: ({:.3}, {:.3}, {:.3}),\n\
             viewmodel_rotation: ({:.3}, {:.3}, {:.3}),\n\
             viewmodel_muzzle_offset: ({:.3}, {:.3}, {:.3}),\n\
             viewmodel_hands_scale: {:.3},\n\
             viewmodel_hands_offset: ({:.3}, {:.3}, {:.3}),\n\
             viewmodel_hands_rotation: ({:.3}, {:.3}, {:.3}),\n\
             viewmodel_pair_offset_pos: ({:.3}, {:.3}, {:.3}),\n\
             viewmodel_pair_offset_rot: ({:.3}, {:.3}, {:.3}),\n\
             viewmodel_ads_pos_offset: ({:.3}, {:.3}, {:.3}),\n\
             viewmodel_ads_rot_offset: ({:.3}, {:.3}, {:.3}),\n\
             viewmodel_ads_hands_pos_offset: ({:.3}, {:.3}, {:.3}),\n\
             viewmodel_ads_hands_rot_offset: ({:.3}, {:.3}, {:.3}),",
            s.name,
            s.viewmodel_scale,
            s.viewmodel_ads_scale,
            s.viewmodel_position.0,
            s.viewmodel_position.1,
            s.viewmodel_position.2,
            s.viewmodel_rotation.0,
            s.viewmodel_rotation.1,
            s.viewmodel_rotation.2,
            s.viewmodel_muzzle_offset.0,
            s.viewmodel_muzzle_offset.1,
            s.viewmodel_muzzle_offset.2,
            s.viewmodel_hands_scale,
            s.viewmodel_hands_offset.0,
            s.viewmodel_hands_offset.1,
            s.viewmodel_hands_offset.2,
            s.viewmodel_hands_rotation.0,
            s.viewmodel_hands_rotation.1,
            s.viewmodel_hands_rotation.2,
            weapon.pair_offset_pos.0,
            weapon.pair_offset_pos.1,
            weapon.pair_offset_pos.2,
            weapon.pair_offset_rot.0,
            weapon.pair_offset_rot.1,
            weapon.pair_offset_rot.2,
            weapon.ads_pos_offset.0,
            weapon.ads_pos_offset.1,
            weapon.ads_pos_offset.2,
            weapon.ads_rot_offset.0,
            weapon.ads_rot_offset.1,
            weapon.ads_rot_offset.2,
            weapon.ads_hands_pos_offset.0,
            weapon.ads_hands_pos_offset.1,
            weapon.ads_hands_pos_offset.2,
            weapon.ads_hands_rot_offset.0,
            weapon.ads_hands_rot_offset.1,
            weapon.ads_hands_rot_offset.2,
        );
    }

    let (h, pressed) =
        draw_action_button("Reset Viewmodel to Defaults", x, y, 260.0, mouse, clicked);
    y += h;
    if pressed {
        let defaults = weapon.id.stats();
        weapon.stats_data.viewmodel_scale = defaults.viewmodel_scale;
        weapon.stats_data.viewmodel_ads_scale = defaults.viewmodel_ads_scale;
        weapon.stats_data.viewmodel_position = defaults.viewmodel_position;
        weapon.stats_data.viewmodel_rotation = defaults.viewmodel_rotation;
        weapon.stats_data.viewmodel_muzzle_offset = defaults.viewmodel_muzzle_offset;
        weapon.stats_data.viewmodel_hands_scale = defaults.viewmodel_hands_scale;
        weapon.stats_data.viewmodel_hands_offset = defaults.viewmodel_hands_offset;
        weapon.stats_data.viewmodel_hands_rotation = defaults.viewmodel_hands_rotation;
        weapon.stats_data.viewmodel_pair_offset_pos = defaults.viewmodel_pair_offset_pos;
        weapon.stats_data.viewmodel_pair_offset_rot = defaults.viewmodel_pair_offset_rot;
        weapon.pair_offset_pos = defaults.viewmodel_pair_offset_pos;
        weapon.pair_offset_rot = defaults.viewmodel_pair_offset_rot;
        weapon.ads_pos_offset = defaults.viewmodel_ads_pos_offset;
        weapon.ads_rot_offset = defaults.viewmodel_ads_rot_offset;
        weapon.ads_hands_pos_offset = defaults.viewmodel_ads_hands_pos_offset;
        weapon.ads_hands_rot_offset = defaults.viewmodel_ads_hands_rot_offset;
        dev.hands_anim_request = Some(HandsAnimRequest::IdleLoop);
    }

    y
}
