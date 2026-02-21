use crate::weapons::crosshair::CrosshairConfig;
use crate::weapons::crosshair_presets::CrosshairPresetManager;
use crate::weapons::stats::WeaponCategory;
use crate::weapons::WeaponInventory;
use macroquad::prelude::*;

use super::draw::{draw_dropdown, draw_shotgun_type_warning, draw_slider, draw_toggle};
use super::shared::{crosshair_type_name, hsv_to_rgb, CrosshairSettingsPreview, CROSSHAIR_TYPES};

pub fn update_crosshair_settings(
    state: &mut CrosshairSettingsPreview,
    weapons: &mut WeaponInventory,
    presets: &mut CrosshairPresetManager,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    mouse: (f32, f32),
    clicked: bool,
    held: bool,
    s: f32,
) {
    let pad = 15.0 * s;
    let x = x + pad;
    let w = w - pad * 2.0;

    // Mouse wheel scroll within the panel area
    let in_panel = mouse.0 >= x && mouse.0 <= x + w && mouse.1 >= y && mouse.1 <= y + h;
    if in_panel {
        let wheel = mouse_wheel().1;
        state.crosshair_scroll -= wheel * 20.0 * s;
    }

    // Compute total content height for clamping scroll
    let picker_size = 120.0 * s;
    let total_h = 50.0 * s      // header → weapon buttons
        + 35.0 * s              // weapon buttons height
        + 15.0 * s              // gap to preset
        + 30.0 * s              // preset row
        + 15.0 * s              // gap to sliders
        + 24.0 * s * 5.0        // 5 slider rows
        + 5.0 * s               // section gap
        + 24.0 * s * 2.0        // outline toggle + thickness
        + 5.0 * s               // section gap
        + 30.0 * s              // dropdown
        + 10.0 * s              // gap
        + 20.0 * s              // "COLOR" label
        + picker_size            // color picker
        + 10.0 * s              // gap
        + 24.0 * s              // alpha slider
        + 10.0 * s;             // bottom padding
    let max_scroll = (total_h - h).max(0.0);
    state.crosshair_scroll = state.crosshair_scroll.clamp(0.0, max_scroll);

    // Apply scroll offset
    let y = y - state.crosshair_scroll;

    let btn_y = y + 50.0 * s;
    let btn_w = 90.0 * s;
    let btn_h = 35.0 * s;
    let btn_gap = 10.0 * s;

    for (i, kind) in WeaponCategory::ALL.iter().enumerate() {
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

    if state.preview_weapon == WeaponCategory::Sniper {
        return;
    }

    let _cat_idx = weapons.index_for_category(state.preview_weapon);
    let weapon = &weapons.weapons[_cat_idx];
    if !state.color_synced {
        state.sync_from_color(weapon.crosshair.color);
    }

    let left_w = (w * 0.55) - 20.0 * s;
    let preset_y = btn_y + btn_h + 15.0 * s;
    let preset_dropdown_w = left_w - 100.0 * s;
    let preset_btn_w = 75.0 * s;
    let preset_btn_gap = 5.0 * s;
    let dropdown_h = 30.0 * s;

    if clicked
        && mouse.0 >= x
        && mouse.0 <= x + preset_dropdown_w
        && mouse.1 >= preset_y
        && mouse.1 <= preset_y + dropdown_h
        && !presets.presets.is_empty()
    {
        state.selected_preset = (state.selected_preset + 1) % presets.presets.len();
        let preset = presets.presets[state.selected_preset].clone();
        let weapon = &mut weapons.weapons[_cat_idx];
        preset.apply_to(&mut weapon.crosshair);
        state.color_synced = false;
    }

    let delete_x = x + preset_dropdown_w + preset_btn_gap;
    if state.selected_preset < presets.presets.len()
        && !presets.presets[state.selected_preset].is_default
    {
        if clicked
            && mouse.0 >= delete_x
            && mouse.0 <= delete_x + preset_btn_w
            && mouse.1 >= preset_y
            && mouse.1 <= preset_y + dropdown_h
        {
            presets.delete(state.selected_preset);
            if state.selected_preset >= presets.presets.len() && !presets.presets.is_empty() {
                state.selected_preset = presets.presets.len() - 1;
            }
        }
    }

    let slider_x = x;
    let mut slider_y = preset_y + 45.0 * s;
    let slider_w = left_w;

    let weapon = &mut weapons.weapons[_cat_idx];
    let ch = &mut weapon.crosshair;

    slider_y += draw_toggle("Enable dot", &mut ch.dot_enabled, slider_x, slider_y, mouse, clicked, s);
    slider_y += draw_slider("Gap", &mut ch.center_gap, 0.0, 10.0, 0.1, slider_x, slider_y, slider_w, mouse, clicked, held, None, s);
    slider_y += draw_slider("Length", &mut ch.line_length, 0.0, 10.0, 0.1, slider_x, slider_y, slider_w, mouse, clicked, held, None, s);
    slider_y += draw_slider("Thickness", &mut ch.line_thickness, 0.0, 2.0, 0.05, slider_x, slider_y, slider_w, mouse, clicked, held, None, s);
    slider_y += draw_slider("Dot size", &mut ch.dot_size, 0.0, 5.0, 0.1, slider_x, slider_y, slider_w, mouse, clicked, held, None, s);
    slider_y += 5.0 * s;
    slider_y += draw_toggle("Outline", &mut ch.outline_enabled, slider_x, slider_y, mouse, clicked, s);
    slider_y += draw_slider("Thickness", &mut ch.outline_thickness, 0.0, 2.0, 0.05, slider_x, slider_y, slider_w, mouse, clicked, held, None, s);
    slider_y += 5.0 * s;

    let type_names: Vec<&str> = CROSSHAIR_TYPES.iter().map(|t| crosshair_type_name(*t)).collect();
    let mut selected_idx = CROSSHAIR_TYPES
        .iter()
        .position(|t| *t == ch.crosshair_type)
        .unwrap_or(0);
    let prev_idx = selected_idx;

    slider_y += draw_dropdown("Crosshair Type", &type_names, &mut selected_idx, slider_x, slider_y, slider_w, mouse, clicked, s);

    if selected_idx < CROSSHAIR_TYPES.len() {
        ch.crosshair_type = CROSSHAIR_TYPES[selected_idx];
    }

    if state.preview_weapon == WeaponCategory::Shotgun
        && selected_idx != prev_idx
        && !state.shotgun_warning_shown_this_visit
    {
        state.shotgun_type_warning = true;
    }

    slider_y += 10.0 * s;

    let hue_bar_w = 20.0 * s;
    let picker_x = slider_x;
    let picker_y = slider_y + 20.0 * s;

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

    let hue_x = sv_x + picker_size + 10.0 * s;
    let hue_y = sv_y;
    if held
        && mouse.0 >= hue_x
        && mouse.0 <= hue_x + hue_bar_w
        && mouse.1 >= hue_y
        && mouse.1 <= hue_y + picker_size
    {
        state.hue = (((mouse.1 - hue_y) / picker_size) * 360.0).clamp(0.0, 359.99);
    }

    slider_y = picker_y + picker_size + 10.0 * s;
    draw_slider(
        "Alpha",
        &mut state.alpha_val,
        0.0,
        255.0,
        1.0,
        slider_x,
        slider_y,
        picker_size + hue_bar_w + 10.0 * s,
        mouse,
        clicked,
        held,
        Some(Color::from_rgba(200, 200, 200, 255)),
        s,
    );

    let (r, g, b) = hsv_to_rgb(state.hue, state.sat, state.val);
    let weapon = &mut weapons.weapons[_cat_idx];
    weapon.crosshair.color = Color::new(r, g, b, state.alpha_val / 255.0);

    // Right column buttons (preview area)
    let preview_x = x + w * 0.55 + 20.0 * s;
    let preview_h = (h - 150.0 * s).max(100.0 * s);
    let btn_row_y = btn_y + btn_h + 15.0 * s + preview_h + 10.0 * s;
    let small_btn_w = 90.0 * s;
    let small_btn_h = 30.0 * s;
    let small_gap = 8.0 * s;

    if clicked
        && mouse.0 >= preview_x
        && mouse.0 <= preview_x + small_btn_w
        && mouse.1 >= btn_row_y
        && mouse.1 <= btn_row_y + small_btn_h
    {
        state.preview_ads = !state.preview_ads;
    }

    let bg_btn_x = preview_x + small_btn_w + small_gap;
    if clicked
        && mouse.0 >= bg_btn_x
        && mouse.0 <= bg_btn_x + small_btn_w
        && mouse.1 >= btn_row_y
        && mouse.1 <= btn_row_y + small_btn_h
    {
        state.preview_light_bg = !state.preview_light_bg;
    }

    let reset_btn_x = bg_btn_x + small_btn_w + small_gap;
    let weapon = &mut weapons.weapons[_cat_idx];
    if clicked
        && mouse.0 >= reset_btn_x
        && mouse.0 <= reset_btn_x + small_btn_w
        && mouse.1 >= btn_row_y
        && mouse.1 <= btn_row_y + small_btn_h
    {
        weapon.crosshair = CrosshairConfig::for_category(weapon.id.category());
        state.color_synced = false;
    }

    if state.shotgun_type_warning {
        let warn_x = preview_x;
        let warn_y = btn_row_y;
        if draw_shotgun_type_warning(warn_x, warn_y, 0.0, 0.0, mouse, clicked, s) {
            state.shotgun_type_warning = false;
            state.shotgun_warning_shown_this_visit = true;
        }
    }
}

pub fn update_crosshair_save_prompt(
    state: &mut CrosshairSettingsPreview,
    weapons: &WeaponInventory,
    presets: &mut CrosshairPresetManager,
) -> Option<bool> {
    let mouse = mouse_position();
    let clicked = is_mouse_button_pressed(MouseButton::Left);
    let cat_idx = weapons.index_for_category(state.preview_weapon);

    let pw = 400.0;
    let ph = 200.0;
    let px = (screen_width() - pw) / 2.0;
    let py = (screen_height() - ph) / 2.0;

    if let Some(conflict_idx) = state.name_conflict_index {
        let btn_w = 110.0;
        let btn_h = 35.0;
        let btn_y = py + ph - btn_h - 20.0;

        let ow_x = px + 20.0;
        if clicked
            && mouse.0 >= ow_x
            && mouse.0 <= ow_x + btn_w
            && mouse.1 >= btn_y
            && mouse.1 <= btn_y + btn_h
        {
            let weapon = &weapons.weapons[cat_idx];
            presets.overwrite(conflict_idx, &weapon.crosshair);
            state.selected_preset = conflict_idx;
            state.name_conflict_index = None;
            state.naming_mode = false;
            state.name_input.clear();
            state.name_error.clear();
            state.show_save_prompt = false;
            return if state.save_prompt_is_leave { Some(true) } else { Some(false) };
        }

        let rename_x = px + (pw - btn_w) / 2.0;
        if clicked
            && mouse.0 >= rename_x
            && mouse.0 <= rename_x + btn_w
            && mouse.1 >= btn_y
            && mouse.1 <= btn_y + btn_h
        {
            state.name_conflict_index = None;
            return None;
        }

        let cancel_x = px + pw - btn_w - 20.0;
        if clicked
            && mouse.0 >= cancel_x
            && mouse.0 <= cancel_x + btn_w
            && mouse.1 >= btn_y
            && mouse.1 <= btn_y + btn_h
        {
            state.name_conflict_index = None;
            state.naming_mode = false;
            state.name_input.clear();
            state.name_error.clear();
            return None;
        }

        if is_key_pressed(KeyCode::Escape) {
            state.name_conflict_index = None;
            state.naming_mode = false;
            state.name_input.clear();
            state.name_error.clear();
            return None;
        }

        return None;
    }

    if state.naming_mode {
        while let Some(ch) = get_char_pressed() {
            if (ch.is_ascii_graphic() || ch == ' ') && state.name_input.len() < 24 {
                state.name_input.push(ch);
                state.name_error.clear();
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            state.name_input.pop();
            state.name_error.clear();
        }
        if is_key_pressed(KeyCode::Enter) && !state.name_input.is_empty() {
            if let Some((idx, is_default)) = presets.find_by_name(&state.name_input) {
                if is_default {
                    state.name_error = "Name reserved, choose a different name".to_string();
                } else {
                    state.name_conflict_index = Some(idx);
                }
            } else {
                let weapon = &weapons.weapons[cat_idx];
                presets.save_as_new(&state.name_input, &weapon.crosshair);
                state.selected_preset = presets.presets.len() - 1;
                state.naming_mode = false;
                state.name_input.clear();
                state.name_error.clear();
                state.show_save_prompt = false;
                return if state.save_prompt_is_leave { Some(true) } else { Some(false) };
            }
        }
        if is_key_pressed(KeyCode::Escape) {
            state.naming_mode = false;
            state.name_input.clear();
            state.name_error.clear();
            return None;
        }
        return None;
    }

    let is_custom =
        state.selected_preset < presets.presets.len() && !presets.presets[state.selected_preset].is_default;

    let btn_w = 120.0;
    let btn_h = 35.0;
    let btn_y = py + ph - btn_h - 20.0;

    if is_custom {
        let gap = 15.0;
        let total = btn_w * 3.0 + gap * 2.0;
        let start_x = px + (pw - total) / 2.0;

        if clicked
            && mouse.0 >= start_x
            && mouse.0 <= start_x + btn_w
            && mouse.1 >= btn_y
            && mouse.1 <= btn_y + btn_h
        {
            state.naming_mode = true;
            state.name_input = presets.suggest_name();
            state.name_error.clear();
            return None;
        }

        let ow_x = start_x + btn_w + gap;
        if clicked
            && mouse.0 >= ow_x
            && mouse.0 <= ow_x + btn_w
            && mouse.1 >= btn_y
            && mouse.1 <= btn_y + btn_h
        {
            let weapon = &weapons.weapons[cat_idx];
            presets.overwrite(state.selected_preset, &weapon.crosshair);
            state.show_save_prompt = false;
            return if state.save_prompt_is_leave { Some(true) } else { Some(false) };
        }

        let ds_x = ow_x + btn_w + gap;
        if clicked
            && mouse.0 >= ds_x
            && mouse.0 <= ds_x + btn_w
            && mouse.1 >= btn_y
            && mouse.1 <= btn_y + btn_h
        {
            state.show_save_prompt = false;
            return if state.save_prompt_is_leave { Some(true) } else { Some(false) };
        }
    } else {
        let gap = 20.0;
        let total = btn_w * 2.0 + gap;
        let start_x = px + (pw - total) / 2.0;

        if clicked
            && mouse.0 >= start_x
            && mouse.0 <= start_x + btn_w
            && mouse.1 >= btn_y
            && mouse.1 <= btn_y + btn_h
        {
            state.naming_mode = true;
            state.name_input = presets.suggest_name();
            state.name_error.clear();
            return None;
        }

        let ds_x = start_x + btn_w + gap;
        if clicked
            && mouse.0 >= ds_x
            && mouse.0 <= ds_x + btn_w
            && mouse.1 >= btn_y
            && mouse.1 <= btn_y + btn_h
        {
            state.show_save_prompt = false;
            return if state.save_prompt_is_leave { Some(true) } else { Some(false) };
        }
    }

    if is_key_pressed(KeyCode::Escape) {
        state.show_save_prompt = false;
        return if state.save_prompt_is_leave { Some(true) } else { Some(false) };
    }

    None
}
