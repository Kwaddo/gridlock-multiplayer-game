use crate::weapons::crosshair::CrosshairConfig;
use crate::weapons::crosshair_presets::CrosshairPresetManager;
use crate::weapons::state::WeaponInventory;
use crate::weapons::stats::WeaponKind;
use macroquad::prelude::*;

use super::color::hsv_to_rgb;
use super::state::CrosshairSettingsPreview;
use super::types::{crosshair_type_name, CROSSHAIR_TYPES};
use super::widgets::{draw_dropdown, draw_slider, draw_toggle};

pub fn update_crosshair_settings(
    state: &mut CrosshairSettingsPreview,
    weapons: &mut WeaponInventory,
    presets: &mut CrosshairPresetManager,
    x: f32,
    y: f32,
    w: f32,
    _h: f32,
    mouse: (f32, f32),
    clicked: bool,
    held: bool,
) {
    // Weapon selector buttons
    let btn_y = y + 60.0;
    let btn_w = 100.0;
    let btn_h = 35.0;
    let btn_gap = 10.0;

    for (i, kind) in WeaponKind::ALL.iter().enumerate() {
        let btn_x = x + (btn_w + btn_gap) * (i as f32);
        if clicked
            && mouse.0 >= btn_x
            && mouse.0 <= btn_x + btn_w
            && mouse.1 >= btn_y
            && mouse.1 <= btn_y + btn_h
        {
            state.preview_weapon = *kind;
            state.color_synced = false;
        }
    }

    if state.preview_weapon == WeaponKind::Sniper {
        return;
    }

    // sync HSV from current crosshair color on first frame
    let weapon = &weapons.weapons[state.preview_weapon.slot() - 1];
    if !state.color_synced {
        state.sync_from_color(weapon.crosshair.color);
    }

    let left_w = (w * 0.55) - 20.0;

    // Preset row: dropdown + delete only
    let preset_y = btn_y + btn_h + 15.0;
    let preset_dropdown_w = left_w - 100.0;
    let preset_btn_w = 75.0;
    let preset_btn_gap = 5.0;

    // Click preset dropdown to cycle
    if clicked
        && mouse.0 >= x
        && mouse.0 <= x + preset_dropdown_w
        && mouse.1 >= preset_y
        && mouse.1 <= preset_y + 30.0
        && !presets.presets.is_empty()
    {
        state.selected_preset = (state.selected_preset + 1) % presets.presets.len();
        let preset = presets.presets[state.selected_preset].clone();
        let weapon = &mut weapons.weapons[state.preview_weapon.slot() - 1];
        preset.apply_to(&mut weapon.crosshair);
        state.color_synced = false;
    }

    // "Delete" button (only for custom presets)
    let delete_x = x + preset_dropdown_w + preset_btn_gap;
    if state.selected_preset < presets.presets.len()
        && !presets.presets[state.selected_preset].is_default
    {
        if clicked
            && mouse.0 >= delete_x
            && mouse.0 <= delete_x + preset_btn_w
            && mouse.1 >= preset_y
            && mouse.1 <= preset_y + 30.0
        {
            presets.delete(state.selected_preset);
            if state.selected_preset >= presets.presets.len() && !presets.presets.is_empty() {
                state.selected_preset = presets.presets.len() - 1;
            }
        }
    }

    // Sliders
    let slider_x = x;
    let mut slider_y = preset_y + 45.0;
    let slider_w = left_w;

    let weapon = &mut weapons.weapons[state.preview_weapon.slot() - 1];
    let ch = &mut weapon.crosshair;

    slider_y += draw_toggle(
        "Enable dot",
        &mut ch.dot_enabled,
        slider_x,
        slider_y,
        mouse,
        clicked,
    );
    slider_y += draw_slider(
        "Gap",
        &mut ch.center_gap,
        0.0,
        10.0,
        0.1,
        slider_x,
        slider_y,
        slider_w,
        mouse,
        clicked,
        held,
        None,
    );
    slider_y += draw_slider(
        "Length",
        &mut ch.line_length,
        0.0,
        10.0,
        0.1,
        slider_x,
        slider_y,
        slider_w,
        mouse,
        clicked,
        held,
        None,
    );
    slider_y += draw_slider(
        "Thickness",
        &mut ch.line_thickness,
        0.0,
        2.0,
        0.05,
        slider_x,
        slider_y,
        slider_w,
        mouse,
        clicked,
        held,
        None,
    );
    slider_y += draw_slider(
        "Dot size",
        &mut ch.dot_size,
        0.0,
        5.0,
        0.1,
        slider_x,
        slider_y,
        slider_w,
        mouse,
        clicked,
        held,
        None,
    );
    slider_y += 5.0;
    slider_y += draw_toggle(
        "Outline",
        &mut ch.outline_enabled,
        slider_x,
        slider_y,
        mouse,
        clicked,
    );
    slider_y += draw_slider(
        "Thickness",
        &mut ch.outline_thickness,
        0.0,
        2.0,
        0.05,
        slider_x,
        slider_y,
        slider_w,
        mouse,
        clicked,
        held,
        None,
    );
    slider_y += 5.0;

    // Crosshair Type dropdown
    let type_names: Vec<&str> = CROSSHAIR_TYPES
        .iter()
        .map(|t| crosshair_type_name(*t))
        .collect();
    let mut selected_idx = CROSSHAIR_TYPES
        .iter()
        .position(|t| *t == ch.crosshair_type)
        .unwrap_or(0);
    let prev_idx = selected_idx;

    slider_y += draw_dropdown(
        "Crosshair Type",
        &type_names,
        &mut selected_idx,
        slider_x,
        slider_y,
        slider_w,
        mouse,
        clicked,
    );

    if selected_idx < CROSSHAIR_TYPES.len() {
        ch.crosshair_type = CROSSHAIR_TYPES[selected_idx];
    }

    if state.preview_weapon == WeaponKind::Shotgun
        && selected_idx != prev_idx
        && !state.shotgun_warning_shown_this_visit
    {
        state.shotgun_type_warning = true;
    }

    slider_y += 10.0;

    // Color picker (SV square + hue bar + alpha slider)
    let picker_size = 140.0;
    let hue_bar_w = 20.0;
    let picker_x = slider_x;
    let picker_y = slider_y + 20.0;

    // SV square interaction
    let sv_x = picker_x;
    let sv_y = picker_y;
    if held
        && mouse.0 >= sv_x
        && mouse.0 <= sv_x + picker_size
        && mouse.1 >= sv_y
        && mouse.1 <= sv_y + picker_size
    {
        state.sat = ((mouse.0 - sv_x) / picker_size).clamp(0.0, 1.0);
        state.val = 1.0 - ((mouse.1 - sv_y) / picker_size).clamp(0.0, 1.0);
    }

    // Hue bar interaction
    let hue_x = sv_x + picker_size + 10.0;
    let hue_y = sv_y;
    if held
        && mouse.0 >= hue_x
        && mouse.0 <= hue_x + hue_bar_w
        && mouse.1 >= hue_y
        && mouse.1 <= hue_y + picker_size
    {
        state.hue = (((mouse.1 - hue_y) / picker_size) * 360.0).clamp(0.0, 359.99);
    }

    // Alpha slider
    slider_y = picker_y + picker_size + 10.0;
    draw_slider(
        "Alpha",
        &mut state.alpha_val,
        0.0,
        255.0,
        1.0,
        slider_x,
        slider_y,
        picker_size + hue_bar_w + 10.0,
        mouse,
        clicked,
        held,
        Some(Color::from_rgba(200, 200, 200, 255)),
    );

    // Apply HSV + alpha back to crosshair
    let (r, g, b) = hsv_to_rgb(state.hue, state.sat, state.val);
    let weapon = &mut weapons.weapons[state.preview_weapon.slot() - 1];
    weapon.crosshair.color = Color::new(r, g, b, state.alpha_val / 255.0);

    // Preview area buttons
    let preview_x = x + w * 0.55 + 20.0;
    let preview_h = 300.0;
    let btn_row_y = y + 60.0 + 35.0 + 15.0 + preview_h + 10.0;
    let small_btn_w = 90.0;
    let small_btn_h = 30.0;
    let small_gap = 8.0;

    // ADS toggle
    if clicked
        && mouse.0 >= preview_x
        && mouse.0 <= preview_x + small_btn_w
        && mouse.1 >= btn_row_y
        && mouse.1 <= btn_row_y + small_btn_h
    {
        state.preview_ads = !state.preview_ads;
    }

    // Light/Dark background toggle
    let bg_btn_x = preview_x + small_btn_w + small_gap;
    if clicked
        && mouse.0 >= bg_btn_x
        && mouse.0 <= bg_btn_x + small_btn_w
        && mouse.1 >= btn_row_y
        && mouse.1 <= btn_row_y + small_btn_h
    {
        state.preview_light_bg = !state.preview_light_bg;
    }

    // Reset to defaults
    let reset_btn_x = bg_btn_x + small_btn_w + small_gap;
    let weapon = &mut weapons.weapons[state.preview_weapon.slot() - 1];
    if clicked
        && mouse.0 >= reset_btn_x
        && mouse.0 <= reset_btn_x + small_btn_w
        && mouse.1 >= btn_row_y
        && mouse.1 <= btn_row_y + small_btn_h
    {
        weapon.crosshair = CrosshairConfig::for_weapon(weapon.kind);
        state.color_synced = false;
    }

    // Shotgun warning dismiss
    if state.shotgun_type_warning {
        let warn_x = preview_x;
        let warn_y = btn_row_y;
        if super::draw::draw_shotgun_type_warning(warn_x, warn_y, 0.0, 0.0, mouse, clicked) {
            state.shotgun_type_warning = false;
            state.shotgun_warning_shown_this_visit = true;
        }
    }
}
