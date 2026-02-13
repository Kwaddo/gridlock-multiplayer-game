/*
    Crosshair settings tab — customize crosshairs per weapon with live preview.
    Features: HSV color picker, preset load/save/overwrite/delete, per-weapon config.
*/

use macroquad::prelude::*;
use crate::weapons::WeaponState;
use crate::weapons::{ WeaponInventory, crosshair::{ CrosshairConfig, CrosshairType } };
use crate::weapons::stats::WeaponKind;
use crate::weapons::crosshair_presets::CrosshairPresetManager;
use std::collections::HashMap;
use crate::ui::fonts::{text_params, measure_game_text};
use crate::ui::draw_arc;

const CROSSHAIR_TYPES: &[CrosshairType] = &[
    CrosshairType::Cross,
    CrosshairType::FourCurves,
    CrosshairType::Chevron,
];

fn crosshair_type_name(ct: CrosshairType) -> &'static str {
    match ct {
        CrosshairType::Cross => "Cross",
        CrosshairType::FourCurves => "4-Curves",
        CrosshairType::Chevron => "Chevron",
    }
}

//  HSV to RGB helpers for color picker

fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let h = if delta < 0.0001 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };

    let s = if max < 0.0001 { 0.0 } else { delta / max };
    let v = max;
    (h, s, v)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let hp = h / 60.0;
    let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());
    let (r1, g1, b1) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    (r1 + m, g1 + m, b1 + m)
}

/*
    STATE
*/

pub struct CrosshairSettingsPreview {
    pub preview_weapon: WeaponKind,
    pub preview_ads: bool,
    pub preview_light_bg: bool,
    pub shotgun_type_warning: bool,
    pub shotgun_warning_shown_this_visit: bool,
    // color picker HSV state
    pub hue: f32,        // 0..360
    pub sat: f32,        // 0..1
    pub val: f32,        // 0..1
    pub alpha_val: f32,  // 0..255
    pub color_synced: bool, // false = need to sync HSV from crosshair color
    // preset state
    pub selected_preset: usize,
    pub naming_mode: bool,
    pub name_input: String,
    // save prompt state
    pub show_save_prompt: bool,
    pub save_prompt_is_leave: bool,
    pub name_conflict_index: Option<usize>,
    pub name_error: String,
    pub pending_tab: Option<u8>, // 0=General, 1=Keybinds — stored as u8 to avoid circular dependency on SettingsTab
}

impl CrosshairSettingsPreview {
    pub fn new() -> Self {
        Self {
            preview_weapon: WeaponKind::Pistol,
            preview_ads: false,
            preview_light_bg: false,
            shotgun_type_warning: false,
            shotgun_warning_shown_this_visit: false,
            hue: 0.0,
            sat: 0.0,
            val: 1.0,
            alpha_val: 255.0,
            color_synced: false,
            selected_preset: 0,
            naming_mode: false,
            name_input: String::new(),
            show_save_prompt: false,
            save_prompt_is_leave: false,
            name_conflict_index: None,
            name_error: String::new(),
            pending_tab: None,
        }
    }

    pub fn reset_visit(&mut self) {
        self.shotgun_warning_shown_this_visit = false;
        self.color_synced = false;
        self.show_save_prompt = false;
        self.naming_mode = false;
        self.name_input.clear();
        self.name_conflict_index = None;
        self.name_error.clear();
        self.pending_tab = None;
    }

    fn sync_from_color(&mut self, color: Color) {
        let (h, s, v) = rgb_to_hsv(color.r, color.g, color.b);
        self.hue = h;
        self.sat = s;
        self.val = v;
        self.alpha_val = color.a * 255.0;
        self.color_synced = true;
    }
}

/*
    UPDATE & DRAW
*/

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
        if clicked && mouse.0 >= btn_x && mouse.0 <= btn_x + btn_w &&
           mouse.1 >= btn_y && mouse.1 <= btn_y + btn_h
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
    if clicked && mouse.0 >= x && mouse.0 <= x + preset_dropdown_w &&
       mouse.1 >= preset_y && mouse.1 <= preset_y + 30.0 && !presets.presets.is_empty()
    {
        state.selected_preset = (state.selected_preset + 1) % presets.presets.len();
        let preset = presets.presets[state.selected_preset].clone();
        let weapon = &mut weapons.weapons[state.preview_weapon.slot() - 1];
        preset.apply_to(&mut weapon.crosshair);
        state.color_synced = false;
    }

    // "Delete" button (only for custom presets)
    let delete_x = x + preset_dropdown_w + preset_btn_gap;
    if state.selected_preset < presets.presets.len() && !presets.presets[state.selected_preset].is_default {
        if clicked && mouse.0 >= delete_x && mouse.0 <= delete_x + preset_btn_w &&
           mouse.1 >= preset_y && mouse.1 <= preset_y + 30.0
        {
            presets.delete(state.selected_preset);
            if state.selected_preset >= presets.presets.len() && !presets.presets.is_empty() {
                state.selected_preset = presets.presets.len() - 1;
            }
        }
    }
/*
    SLIDERS
*/
    let slider_x = x;
    let mut slider_y = preset_y + 45.0;
    let slider_w = left_w;

    let weapon = &mut weapons.weapons[state.preview_weapon.slot() - 1];
    let ch = &mut weapon.crosshair;

    slider_y += draw_toggle("Enable dot", &mut ch.dot_enabled, slider_x, slider_y, mouse, clicked);
    slider_y += draw_slider("Gap", &mut ch.center_gap, 0.0, 10.0, 0.1, slider_x, slider_y, slider_w, mouse, clicked, held, None);
    slider_y += draw_slider("Length", &mut ch.line_length, 0.0, 10.0, 0.1, slider_x, slider_y, slider_w, mouse, clicked, held, None);
    slider_y += draw_slider("Thickness", &mut ch.line_thickness, 0.0, 2.0, 0.05, slider_x, slider_y, slider_w, mouse, clicked, held, None);
    slider_y += draw_slider("Dot size", &mut ch.dot_size, 0.0, 5.0, 0.1, slider_x, slider_y, slider_w, mouse, clicked, held, None);
    slider_y += 5.0;
    slider_y += draw_toggle("Outline", &mut ch.outline_enabled, slider_x, slider_y, mouse, clicked);
    slider_y += draw_slider("Thickness", &mut ch.outline_thickness, 0.0, 2.0, 0.05, slider_x, slider_y, slider_w, mouse, clicked, held, None);
    slider_y += 5.0;

    // crosshair Type dropdown
    let type_names: Vec<&str> = CROSSHAIR_TYPES.iter().map(|t| crosshair_type_name(*t)).collect();
    let mut selected_idx = CROSSHAIR_TYPES.iter().position(|t| *t == ch.crosshair_type).unwrap_or(0);
    let prev_idx = selected_idx;

    slider_y += draw_dropdown("Crosshair Type", &type_names, &mut selected_idx, slider_x, slider_y, slider_w, mouse, clicked);

    if selected_idx < CROSSHAIR_TYPES.len() {
        ch.crosshair_type = CROSSHAIR_TYPES[selected_idx];
    }

    if state.preview_weapon == WeaponKind::Shotgun && selected_idx != prev_idx && !state.shotgun_warning_shown_this_visit {
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
    if held && mouse.0 >= sv_x && mouse.0 <= sv_x + picker_size &&
       mouse.1 >= sv_y && mouse.1 <= sv_y + picker_size
    {
        state.sat = ((mouse.0 - sv_x) / picker_size).clamp(0.0, 1.0);
        state.val = 1.0 - ((mouse.1 - sv_y) / picker_size).clamp(0.0, 1.0);
    }

    // Hue bar interaction
    let hue_x = sv_x + picker_size + 10.0;
    let hue_y = sv_y;
    if held && mouse.0 >= hue_x && mouse.0 <= hue_x + hue_bar_w &&
       mouse.1 >= hue_y && mouse.1 <= hue_y + picker_size
    {
        state.hue = (((mouse.1 - hue_y) / picker_size) * 360.0).clamp(0.0, 359.99);
    }

    // Alpha slider
    slider_y = picker_y + picker_size + 10.0;
    draw_slider("Alpha", &mut state.alpha_val, 0.0, 255.0, 1.0, slider_x, slider_y, picker_size + hue_bar_w + 10.0, mouse, clicked, held, Some(Color::from_rgba(200, 200, 200, 255)));

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
    if clicked && mouse.0 >= preview_x && mouse.0 <= preview_x + small_btn_w &&
       mouse.1 >= btn_row_y && mouse.1 <= btn_row_y + small_btn_h
    {
        state.preview_ads = !state.preview_ads;
    }

    // Light/Dark background toggle
    let bg_btn_x = preview_x + small_btn_w + small_gap;
    if clicked && mouse.0 >= bg_btn_x && mouse.0 <= bg_btn_x + small_btn_w &&
       mouse.1 >= btn_row_y && mouse.1 <= btn_row_y + small_btn_h
    {
        state.preview_light_bg = !state.preview_light_bg;
    }

    // Reset to defaults
    let reset_btn_x = bg_btn_x + small_btn_w + small_gap;
    let weapon = &mut weapons.weapons[state.preview_weapon.slot() - 1];
    if clicked && mouse.0 >= reset_btn_x && mouse.0 <= reset_btn_x + small_btn_w &&
       mouse.1 >= btn_row_y && mouse.1 <= btn_row_y + small_btn_h
    {
        weapon.crosshair = CrosshairConfig::for_weapon(weapon.kind);
        state.color_synced = false;
    }

    // Shotgun warning dismiss
    if state.shotgun_type_warning {
        let warn_x = preview_x;
        let warn_y = btn_row_y;
        if draw_shotgun_type_warning(warn_x, warn_y, 0.0, 0.0, mouse, clicked) {
            state.shotgun_type_warning = false;
            state.shotgun_warning_shown_this_visit = true;
        }
    }
}

/*
    SAVE PROMPT MODAL
    Fullscreen modal for saving crosshair presets.
    Called from settings/mod.rs when show_save_prompt is true.
*/

/// Returns None = still open, Some(true) = done + should leave, Some(false) = done + stay
pub fn update_crosshair_save_prompt(
    state: &mut CrosshairSettingsPreview,
    weapons: &WeaponInventory,
    presets: &mut CrosshairPresetManager,
) -> Option<bool> {
    let mouse = mouse_position();
    let clicked = is_mouse_button_pressed(MouseButton::Left);

    let pw = 400.0;
    let ph = 200.0;
    let px = (screen_width() - pw) / 2.0;
    let py = (screen_height() - ph) / 2.0;

    // --- Name conflict sub-prompt ---
    if let Some(conflict_idx) = state.name_conflict_index {
        // "A preset with this name already exists"
        // Buttons: Overwrite / Rename / Cancel
        let btn_w = 110.0;
        let btn_h = 35.0;
        let btn_y = py + ph - btn_h - 20.0;

        // Overwrite
        let ow_x = px + 20.0;
        if clicked && mouse.0 >= ow_x && mouse.0 <= ow_x + btn_w &&
           mouse.1 >= btn_y && mouse.1 <= btn_y + btn_h
        {
            let weapon = &weapons.weapons[state.preview_weapon.slot() - 1];
            presets.overwrite(conflict_idx, &weapon.crosshair);
            state.selected_preset = conflict_idx;
            state.name_conflict_index = None;
            state.naming_mode = false;
            state.name_input.clear();
            state.name_error.clear();
            state.show_save_prompt = false;
            return if state.save_prompt_is_leave { Some(true) } else { Some(false) };
        }

        // Rename (go back to naming)
        let rename_x = px + (pw - btn_w) / 2.0;
        if clicked && mouse.0 >= rename_x && mouse.0 <= rename_x + btn_w &&
           mouse.1 >= btn_y && mouse.1 <= btn_y + btn_h
        {
            state.name_conflict_index = None;
            // stay in naming_mode with current name_input
            return None;
        }

        // Cancel
        let cancel_x = px + pw - btn_w - 20.0;
        if clicked && mouse.0 >= cancel_x && mouse.0 <= cancel_x + btn_w &&
           mouse.1 >= btn_y && mouse.1 <= btn_y + btn_h
        {
            state.name_conflict_index = None;
            state.naming_mode = false;
            state.name_input.clear();
            state.name_error.clear();
            // back to main save prompt
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

    // --- Naming mode (text input for preset name) ---
    if state.naming_mode {
        while let Some(ch) = get_char_pressed() {
            if ch.is_ascii_graphic() || ch == ' ' {
                if state.name_input.len() < 24 {
                    state.name_input.push(ch);
                    state.name_error.clear();
                }
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            state.name_input.pop();
            state.name_error.clear();
        }
        if is_key_pressed(KeyCode::Enter) && !state.name_input.is_empty() {
            // Check for name collisions
            if let Some((idx, is_default)) = presets.find_by_name(&state.name_input) {
                if is_default {
                    state.name_error = "Name reserved, choose a different name".to_string();
                } else {
                    state.name_conflict_index = Some(idx);
                }
            } else {
                // No collision — save as new
                let weapon = &weapons.weapons[state.preview_weapon.slot() - 1];
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
            // go back to main save prompt, don't close
            return None;
        }
        return None;
    }

    // --- Main save prompt ---
    let is_custom = state.selected_preset < presets.presets.len()
        && !presets.presets[state.selected_preset].is_default;

    let btn_w = 120.0;
    let btn_h = 35.0;
    let btn_y = py + ph - btn_h - 20.0;

    if is_custom {
        // 3 buttons: Save As New / Overwrite / Don't Save
        let gap = 15.0;
        let total = btn_w * 3.0 + gap * 2.0;
        let start_x = px + (pw - total) / 2.0;

        // Save As New
        if clicked && mouse.0 >= start_x && mouse.0 <= start_x + btn_w &&
           mouse.1 >= btn_y && mouse.1 <= btn_y + btn_h
        {
            state.naming_mode = true;
            state.name_input = presets.suggest_name();
            state.name_error.clear();
            return None;
        }

        // Overwrite
        let ow_x = start_x + btn_w + gap;
        if clicked && mouse.0 >= ow_x && mouse.0 <= ow_x + btn_w &&
           mouse.1 >= btn_y && mouse.1 <= btn_y + btn_h
        {
            let weapon = &weapons.weapons[state.preview_weapon.slot() - 1];
            presets.overwrite(state.selected_preset, &weapon.crosshair);
            state.show_save_prompt = false;
            return if state.save_prompt_is_leave { Some(true) } else { Some(false) };
        }

        // Don't Save
        let ds_x = ow_x + btn_w + gap;
        if clicked && mouse.0 >= ds_x && mouse.0 <= ds_x + btn_w &&
           mouse.1 >= btn_y && mouse.1 <= btn_y + btn_h
        {
            state.show_save_prompt = false;
            return if state.save_prompt_is_leave { Some(true) } else { Some(false) };
        }
    } else {
        // 2 buttons: Save As New / Don't Save
        let gap = 20.0;
        let total = btn_w * 2.0 + gap;
        let start_x = px + (pw - total) / 2.0;

        // Save As New
        if clicked && mouse.0 >= start_x && mouse.0 <= start_x + btn_w &&
           mouse.1 >= btn_y && mouse.1 <= btn_y + btn_h
        {
            state.naming_mode = true;
            state.name_input = presets.suggest_name();
            state.name_error.clear();
            return None;
        }

        // Don't Save
        let ds_x = start_x + btn_w + gap;
        if clicked && mouse.0 >= ds_x && mouse.0 <= ds_x + btn_w &&
           mouse.1 >= btn_y && mouse.1 <= btn_y + btn_h
        {
            state.show_save_prompt = false;
            return if state.save_prompt_is_leave { Some(true) } else { Some(false) };
        }
    }

    // ESC = Don't Save
    if is_key_pressed(KeyCode::Escape) {
        state.show_save_prompt = false;
        return if state.save_prompt_is_leave { Some(true) } else { Some(false) };
    }

    None
}

/// Draw the save prompt modal on top of everything
pub fn draw_crosshair_save_prompt(
    state: &CrosshairSettingsPreview,
    presets: &CrosshairPresetManager,
) {
    // Fullscreen dark overlay
    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::from_rgba(0, 0, 0, 150));

    let pw = 400.0;
    let ph = 200.0;
    let px = (screen_width() - pw) / 2.0;
    let py = (screen_height() - ph) / 2.0;

    draw_rectangle(px, py, pw, ph, Color::from_rgba(40, 40, 40, 255));
    draw_rectangle_lines(px, py, pw, ph, 2.0, WHITE);

    let btn_w = 120.0;
    let btn_h = 35.0;
    let btn_y = py + ph - btn_h - 20.0;

    // --- Name conflict sub-prompt ---
    if state.name_conflict_index.is_some() {
        let title = "Name Already Exists";
        let title_w = measure_game_text(title, 20);
        draw_text_ex(title, px + (pw - title_w) / 2.0, py + 35.0, text_params(20, YELLOW));

        let msg = "A preset with this name already exists.";
        let msg_w = measure_game_text(msg, 14);
        draw_text_ex(msg, px + (pw - msg_w) / 2.0, py + 65.0, text_params(14, WHITE));

        let conflict_btn_w = 110.0;

        // Overwrite
        let ow_x = px + 20.0;
        draw_rectangle(ow_x, btn_y, conflict_btn_w, btn_h, Color::from_rgba(80, 80, 30, 255));
        draw_rectangle_lines(ow_x, btn_y, conflict_btn_w, btn_h, 1.0, WHITE);
        let ow_tw = measure_game_text("Overwrite", 16);
        draw_text_ex("Overwrite", ow_x + (conflict_btn_w - ow_tw) / 2.0, btn_y + 24.0, text_params(16, WHITE));

        // Rename
        let rename_x = px + (pw - conflict_btn_w) / 2.0;
        draw_rectangle(rename_x, btn_y, conflict_btn_w, btn_h, Color::from_rgba(30, 80, 30, 255));
        draw_rectangle_lines(rename_x, btn_y, conflict_btn_w, btn_h, 1.0, WHITE);
        let rn_tw = measure_game_text("Rename", 16);
        draw_text_ex("Rename", rename_x + (conflict_btn_w - rn_tw) / 2.0, btn_y + 24.0, text_params(16, WHITE));

        // Cancel
        let cancel_x = px + pw - conflict_btn_w - 20.0;
        draw_rectangle(cancel_x, btn_y, conflict_btn_w, btn_h, Color::from_rgba(80, 30, 30, 255));
        draw_rectangle_lines(cancel_x, btn_y, conflict_btn_w, btn_h, 1.0, WHITE);
        let cn_tw = measure_game_text("Cancel", 16);
        draw_text_ex("Cancel", cancel_x + (conflict_btn_w - cn_tw) / 2.0, btn_y + 24.0, text_params(16, WHITE));

        return;
    }

    // --- Naming mode ---
    if state.naming_mode {
        let title = "Preset Name";
        let title_w = measure_game_text(title, 20);
        draw_text_ex(title, px + (pw - title_w) / 2.0, py + 35.0, text_params(20, Color::from_rgba(100, 180, 255, 255)));

        // Text input box
        let input_x = px + 30.0;
        let input_y = py + 60.0;
        let input_w = pw - 60.0;
        draw_rectangle(input_x, input_y, input_w, 32.0, Color::from_rgba(20, 20, 20, 255));
        draw_rectangle_lines(input_x, input_y, input_w, 32.0, 1.0, WHITE);

        let cursor = if (get_time() * 2.0) as u32 % 2 == 0 { "|" } else { "" };
        let display = format!("{}{}", state.name_input, cursor);
        draw_text_ex(&display, input_x + 8.0, input_y + 22.0, text_params(16, WHITE));

        // Error text
        if !state.name_error.is_empty() {
            draw_text_ex(&state.name_error, input_x, input_y + 50.0, text_params(14, Color::from_rgba(255, 80, 80, 255)));
        }

        let hint = "Enter to save, Esc to go back";
        let hint_w = measure_game_text(hint, 12);
        draw_text_ex(hint, px + (pw - hint_w) / 2.0, py + ph - 15.0, text_params(12, GRAY));

        return;
    }

    // --- Main save prompt ---
    let title = "Save Crosshair Preset";
    let title_w = measure_game_text(title, 20);
    draw_text_ex(title, px + (pw - title_w) / 2.0, py + 35.0, text_params(20, YELLOW));

    let preset_name = if state.selected_preset < presets.presets.len() {
        &presets.presets[state.selected_preset].name
    } else {
        "None"
    };
    let msg = format!("Current: {}", preset_name);
    let msg_w = measure_game_text(&msg, 14);
    draw_text_ex(&msg, px + (pw - msg_w) / 2.0, py + 65.0, text_params(14, GRAY));

    let is_custom = state.selected_preset < presets.presets.len()
        && !presets.presets[state.selected_preset].is_default;

    if is_custom {
        // 3 buttons
        let gap = 15.0;
        let total = btn_w * 3.0 + gap * 2.0;
        let start_x = px + (pw - total) / 2.0;

        draw_rectangle(start_x, btn_y, btn_w, btn_h, Color::from_rgba(30, 80, 30, 255));
        draw_rectangle_lines(start_x, btn_y, btn_w, btn_h, 1.0, WHITE);
        let sa_tw = measure_game_text("Save As New", 14);
        draw_text_ex("Save As New", start_x + (btn_w - sa_tw) / 2.0, btn_y + 24.0, text_params(14, WHITE));

        let ow_x = start_x + btn_w + gap;
        draw_rectangle(ow_x, btn_y, btn_w, btn_h, Color::from_rgba(80, 80, 30, 255));
        draw_rectangle_lines(ow_x, btn_y, btn_w, btn_h, 1.0, WHITE);
        let ow_tw = measure_game_text("Overwrite", 14);
        draw_text_ex("Overwrite", ow_x + (btn_w - ow_tw) / 2.0, btn_y + 24.0, text_params(14, WHITE));

        let ds_x = ow_x + btn_w + gap;
        draw_rectangle(ds_x, btn_y, btn_w, btn_h, Color::from_rgba(80, 30, 30, 255));
        draw_rectangle_lines(ds_x, btn_y, btn_w, btn_h, 1.0, WHITE);
        let ds_tw = measure_game_text("Don't Save", 14);
        draw_text_ex("Don't Save", ds_x + (btn_w - ds_tw) / 2.0, btn_y + 24.0, text_params(14, WHITE));
    } else {
        // 2 buttons
        let gap = 20.0;
        let total = btn_w * 2.0 + gap;
        let start_x = px + (pw - total) / 2.0;

        draw_rectangle(start_x, btn_y, btn_w, btn_h, Color::from_rgba(30, 80, 30, 255));
        draw_rectangle_lines(start_x, btn_y, btn_w, btn_h, 1.0, WHITE);
        let sa_tw = measure_game_text("Save As New", 14);
        draw_text_ex("Save As New", start_x + (btn_w - sa_tw) / 2.0, btn_y + 24.0, text_params(14, WHITE));

        let ds_x = start_x + btn_w + gap;
        draw_rectangle(ds_x, btn_y, btn_w, btn_h, Color::from_rgba(80, 30, 30, 255));
        draw_rectangle_lines(ds_x, btn_y, btn_w, btn_h, 1.0, WHITE);
        let ds_tw = measure_game_text("Don't Save", 14);
        draw_text_ex("Don't Save", ds_x + (btn_w - ds_tw) / 2.0, btn_y + 24.0, text_params(14, WHITE));
    }
}

/*
    DRAW
*/
pub fn draw_crosshair_settings(
    state: &CrosshairSettingsPreview,
    weapons: &WeaponInventory,
    presets: &CrosshairPresetManager,
    x: f32,
    y: f32,
    w: f32,
    _h: f32,
    _assets: &HashMap<String, Texture2D>,
    _mouse: (f32, f32)
) {
    draw_section_header("Crosshair Settings", x + 20.0, y + 20.0);

    // Weapon selector buttons 
    let btn_y = y + 60.0;
    let btn_w = 100.0;
    let btn_h = 35.0;
    let btn_gap = 10.0;

    for (i, kind) in WeaponKind::ALL.iter().enumerate() {
        let btn_x = x + (btn_w + btn_gap) * (i as f32);
        let btn_color = if kind == &state.preview_weapon {
            Color::from_rgba(80, 150, 220, 255)
        } else {
            Color::from_rgba(40, 40, 40, 255)
        };
        draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_color);
        draw_rectangle_lines(btn_x, btn_y, btn_w, btn_h, 1.0, GRAY);
        let name = kind.stats().name;
        let text_w = measure_game_text(name, 14);
        draw_text_ex(name, btn_x + (btn_w - text_w) / 2.0, btn_y + 22.0, text_params(14, WHITE));
    }

    if state.preview_weapon == WeaponKind::Sniper {
        let sniper_y = btn_y + btn_h + 60.0;
        draw_text_ex("Sniper scope customization coming soon", x + 20.0, sniper_y, text_params(16, YELLOW));
        return;
    }

    let left_w = (w * 0.55) - 20.0;

    // Preset row: dropdown + delete
    let preset_y = btn_y + btn_h + 15.0;
    let preset_dropdown_w = left_w - 100.0;
    let preset_btn_w = 75.0;
    let preset_btn_gap = 5.0;

    // Preset dropdown
    let preset_name = if state.selected_preset < presets.presets.len() {
        &presets.presets[state.selected_preset].name
    } else {
        "None"
    };
    let is_custom = state.selected_preset < presets.presets.len() && !presets.presets[state.selected_preset].is_default;

    draw_rectangle(x, preset_y, preset_dropdown_w, 30.0, Color::from_rgba(35, 35, 35, 255));
    draw_rectangle_lines(x, preset_y, preset_dropdown_w, 30.0, 1.0, Color::from_rgba(80, 80, 80, 255));
    let preset_tw = measure_game_text(preset_name, 14);
    draw_text_ex(preset_name, x + (preset_dropdown_w - preset_tw) / 2.0, preset_y + 20.0, text_params(14, WHITE));
    draw_text_ex("<", x + 6.0, preset_y + 20.0, text_params(14, GRAY));
    let arrow_r = measure_game_text(">", 14);
    draw_text_ex(">", x + preset_dropdown_w - arrow_r - 6.0, preset_y + 20.0, text_params(14, GRAY));

    // "Delete" button
    let delete_x = x + preset_dropdown_w + preset_btn_gap;
    let del_color = if is_custom { Color::from_rgba(80, 30, 30, 255) } else { Color::from_rgba(30, 30, 30, 255) };
    draw_rectangle(delete_x, preset_y, preset_btn_w, 30.0, del_color);
    draw_rectangle_lines(delete_x, preset_y, preset_btn_w, 30.0, 1.0, GRAY);
    let del_tw = measure_game_text("Delete", 12);
    let del_text_color = if is_custom { WHITE } else { Color::from_rgba(80, 80, 80, 255) };
    draw_text_ex("Delete", delete_x + (preset_btn_w - del_tw) / 2.0, preset_y + 19.0, text_params(12, del_text_color));

    // Sliders 
    let slider_x = x;
    let mut slider_y = preset_y + 45.0;
    let slider_w = left_w;

    let weapon = &weapons.weapons[state.preview_weapon.slot() - 1];
    let ch = &weapon.crosshair;

    // draw toggles & sliders (read-only draw; update fn handles mutation)
    draw_toggle_display("Enable dot", ch.dot_enabled, slider_x, slider_y);
    slider_y += 28.0;
    draw_slider_display("Gap", ch.center_gap, 0.0, 10.0, slider_x, slider_y, slider_w, None);
    slider_y += 28.0;
    draw_slider_display("Length", ch.line_length, 0.0, 10.0, slider_x, slider_y, slider_w, None);
    slider_y += 28.0;
    draw_slider_display("Thickness", ch.line_thickness, 0.0, 2.0, slider_x, slider_y, slider_w, None);
    slider_y += 28.0;
    draw_slider_display("Dot size", ch.dot_size, 0.0, 5.0, slider_x, slider_y, slider_w, None);
    slider_y += 33.0;
    draw_toggle_display("Outline", ch.outline_enabled, slider_x, slider_y);
    slider_y += 28.0;
    draw_slider_display("Thickness", ch.outline_thickness, 0.0, 2.0, slider_x, slider_y, slider_w, None);
    slider_y += 33.0;

    // Crosshair type display
    let type_name = crosshair_type_name(ch.crosshair_type);
    draw_dropdown_display("Crosshair Type", type_name, slider_x, slider_y, slider_w);
    slider_y += 40.0;

    //  Color picker
    draw_text_ex("COLOR", slider_x, slider_y + 14.0, text_params(14, Color::from_rgba(100, 180, 255, 255)));
    slider_y += 20.0;

    let picker_size = 140.0;
    let hue_bar_w = 20.0;
    let sv_x = slider_x;
    let sv_y = slider_y;

    // SV square: draw with gradient approximation
    let steps = 20u32;
    let cell_w = picker_size / steps as f32;
    let cell_h = picker_size / steps as f32;
    for sy in 0..steps {
        for sx in 0..steps {
            let s = (sx as f32 + 0.5) / steps as f32;
            let v = 1.0 - (sy as f32 + 0.5) / steps as f32;
            let (r, g, b) = hsv_to_rgb(state.hue, s, v);
            draw_rectangle(
                sv_x + sx as f32 * cell_w,
                sv_y + sy as f32 * cell_h,
                cell_w + 1.0,
                cell_h + 1.0,
                Color::new(r, g, b, 1.0),
            );
        }
    }
    draw_rectangle_lines(sv_x, sv_y, picker_size, picker_size, 1.0, GRAY);

    // SV cursor
    let cursor_x = sv_x + state.sat * picker_size;
    let cursor_y = sv_y + (1.0 - state.val) * picker_size;
    draw_circle_lines(cursor_x, cursor_y, 5.0, 2.0, WHITE);
    draw_circle_lines(cursor_x, cursor_y, 4.0, 1.0, BLACK);

    // Hue bar
    let hue_x = sv_x + picker_size + 10.0;
    let hue_steps = 36u32;
    let hue_cell_h = picker_size / hue_steps as f32;
    for i in 0..hue_steps {
        let h = (i as f32 / hue_steps as f32) * 360.0;
        let (r, g, b) = hsv_to_rgb(h, 1.0, 1.0);
        draw_rectangle(hue_x, sv_y + i as f32 * hue_cell_h, hue_bar_w, hue_cell_h + 1.0, Color::new(r, g, b, 1.0));
    }
    draw_rectangle_lines(hue_x, sv_y, hue_bar_w, picker_size, 1.0, GRAY);

    // Hue cursor
    let hue_cursor_y = sv_y + (state.hue / 360.0) * picker_size;
    draw_rectangle(hue_x - 2.0, hue_cursor_y - 2.0, hue_bar_w + 4.0, 4.0, WHITE);
    draw_rectangle_lines(hue_x - 2.0, hue_cursor_y - 2.0, hue_bar_w + 4.0, 4.0, 1.0, BLACK);

    // Color preview swatch
    let swatch_x = hue_x + hue_bar_w + 15.0;
    let swatch_size = 40.0;
    // checkerboard behind swatch (to show alpha)
    let check = 10.0;
    for cy_i in 0..4 {
        for cx_i in 0..4 {
            let col = if (cx_i + cy_i) % 2 == 0 { Color::from_rgba(180, 180, 180, 255) } else { Color::from_rgba(120, 120, 120, 255) };
            draw_rectangle(swatch_x + cx_i as f32 * check, sv_y + cy_i as f32 * check, check, check, col);
        }
    }
    let (pr, pg, pb) = hsv_to_rgb(state.hue, state.sat, state.val);
    draw_rectangle(swatch_x, sv_y, swatch_size, swatch_size, Color::new(pr, pg, pb, state.alpha_val / 255.0));
    draw_rectangle_lines(swatch_x, sv_y, swatch_size, swatch_size, 1.0, WHITE);

    // Alpha slider below picker
    let alpha_y = sv_y + picker_size + 10.0;
    draw_slider_display("Alpha", state.alpha_val, 0.0, 255.0, slider_x, alpha_y, picker_size + hue_bar_w + 10.0, Some(Color::from_rgba(200, 200, 200, 255)));

    //  Preview box 
    let preview_x = x + w * 0.55 + 20.0;
    let preview_y = btn_y + btn_h + 15.0;
    let preview_w = w * 0.45 - 40.0;
    let preview_h = 300.0;

    draw_crosshair_preview(weapon, state, preview_x, preview_y, preview_w, preview_h, _assets);

    //  Preview buttons 
    let btn_row_y = preview_y + preview_h + 10.0;
    let small_btn_w = 90.0;
    let small_btn_h = 30.0;
    let small_gap = 8.0;

    // ADS toggle
    let ads_color = if state.preview_ads { Color::from_rgba(80, 150, 220, 255) } else { Color::from_rgba(40, 40, 40, 255) };
    draw_rectangle(preview_x, btn_row_y, small_btn_w, small_btn_h, ads_color);
    draw_rectangle_lines(preview_x, btn_row_y, small_btn_w, small_btn_h, 1.0, GRAY);
    let ads_text = if state.preview_ads { "ADS: ON" } else { "ADS: OFF" };
    let ads_tw = measure_game_text(ads_text, 12);
    draw_text_ex(ads_text, preview_x + (small_btn_w - ads_tw) / 2.0, btn_row_y + 20.0, text_params(12, WHITE));

    // Background toggle
    let bg_btn_x = preview_x + small_btn_w + small_gap;
    let bg_color = if state.preview_light_bg { Color::from_rgba(180, 180, 180, 255) } else { Color::from_rgba(40, 40, 40, 255) };
    draw_rectangle(bg_btn_x, btn_row_y, small_btn_w, small_btn_h, bg_color);
    draw_rectangle_lines(bg_btn_x, btn_row_y, small_btn_w, small_btn_h, 1.0, GRAY);
    let bg_text = if state.preview_light_bg { "Light" } else { "Dark" };
    let bg_tw = measure_game_text(bg_text, 12);
    let bg_text_color = if state.preview_light_bg { BLACK } else { WHITE };
    draw_text_ex(bg_text, bg_btn_x + (small_btn_w - bg_tw) / 2.0, btn_row_y + 20.0, text_params(12, bg_text_color));

    // Reset button
    let reset_btn_x = bg_btn_x + small_btn_w + small_gap;
    draw_rectangle(reset_btn_x, btn_row_y, small_btn_w, small_btn_h, Color::from_rgba(60, 40, 40, 255));
    draw_rectangle_lines(reset_btn_x, btn_row_y, small_btn_w, small_btn_h, 1.0, GRAY);
    let reset_tw = measure_game_text("Reset", 12);
    draw_text_ex("Reset", reset_btn_x + (small_btn_w - reset_tw) / 2.0, btn_row_y + 20.0, text_params(12, WHITE));

    // Shotgun warning
    if state.preview_weapon == WeaponKind::Shotgun && state.shotgun_type_warning {
        draw_shotgun_type_warning(preview_x, btn_row_y, 0.0, 0.0, _mouse, false);
    }
}

/*
    PREVIEW BOX
*/

fn draw_crosshair_preview(
    weapon: &WeaponState,
    state: &CrosshairSettingsPreview,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    assets: &HashMap<String, Texture2D>
) {
    const PREVIEW_BG_TEXTURE_KEY: &str = "crosshair_preview_bg";

    if let Some(bg_texture) = assets.get(PREVIEW_BG_TEXTURE_KEY) {
        draw_texture_ex(
            bg_texture, x, y, WHITE,
            DrawTextureParams { dest_size: Some(vec2(w, h)), ..Default::default() }
        );
    } else {
        let bg_color = if state.preview_light_bg {
            Color::from_rgba(200, 200, 200, 255)
        } else {
            Color::from_rgba(20, 20, 20, 255)
        };
        draw_rectangle(x, y, w, h, bg_color);
    }
    draw_rectangle_lines(x, y, w, h, 2.0, GRAY);

    let cx = x + w / 2.0;
    let cy = y + h / 2.0;

    if weapon.kind == WeaponKind::Sniper {
        draw_text_ex("Scope preview coming soon", cx - 80.0, cy, text_params(14, YELLOW));
        return;
    }

    let xhair = &weapon.crosshair;
    let sh = screen_height();
    let ads_progress = if state.preview_ads { 1.0 } else { 0.0 };
    let color = xhair.color;

    match xhair.crosshair_type {
        CrosshairType::FourCurves => {
            let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - ads_progress);
            let ads_rest_gap = ((xhair.center_gap * sh) / 100.0) * 0.4 * ads_progress;
            let radius = hip_gap + ads_rest_gap;
            let thick = (xhair.line_thickness * sh) / 100.0;
            let gap_angle: f32 = 20.0_f32.to_radians();
            let quarter = std::f32::consts::FRAC_PI_2;
            let segs = 32;

            if xhair.outline_enabled {
                let ot = thick + (xhair.outline_thickness * sh) / 100.0 * 2.0;
                let oc = Color::from_rgba(0, 0, 0, 180);
                for q in 0..4 {
                    let base_angle = q as f32 * quarter;
                    draw_arc(cx, cy, radius, base_angle + gap_angle, base_angle + quarter - gap_angle, ot, segs, oc);
                }
            }
            for q in 0..4 {
                let base_angle = q as f32 * quarter;
                draw_arc(cx, cy, radius, base_angle + gap_angle, base_angle + quarter - gap_angle, thick, segs, color);
            }
            if xhair.dot_enabled {
                let dot_alpha = ((1.0 - ads_progress) * 255.0) as u8;
                let dot_color = Color::from_rgba((color.r * 255.0) as u8, (color.g * 255.0) as u8, (color.b * 255.0) as u8, dot_alpha);
                draw_circle(cx, cy, (xhair.dot_size * sh) / 100.0, dot_color);
            }
        }
        CrosshairType::Chevron => {
            let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - ads_progress);
            let ads_rest_gap = ((xhair.center_gap * sh) / 100.0) * 0.4 * ads_progress;
            let radius = hip_gap + ads_rest_gap;
            let thick = (xhair.line_thickness * sh) / 100.0;
            let arm = ((xhair.line_length / 2.0) * sh) / 100.0;

            let corners: [(f32, f32); 4] = [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)];

            for &(sx, sy) in &corners {
                let corner_x = cx + sx * radius;
                let corner_y = cy + sy * radius;
                let h_end_x = corner_x - sx * arm;
                let v_end_y = corner_y - sy * arm;
                let chamfer = arm * 0.35;
                let h_near = corner_x - sx * chamfer;
                let v_near = corner_y - sy * chamfer;

                let p0x = h_near; let p0y = corner_y;
                let p1x = corner_x; let p1y = corner_y;
                let p2x = corner_x; let p2y = v_near;
                let bez = |t: f32| -> (f32, f32) {
                    let u = 1.0 - t;
                    (u*u*p0x + 2.0*u*t*p1x + t*t*p2x, u*u*p0y + 2.0*u*t*p1y + t*t*p2y)
                };

                let t_a = 0.10_f32;
                let t_b = 0.90_f32;
                let (mid_ax, mid_ay) = bez(t_a);
                let (mid_bx, mid_by) = bez(t_b);
                let chamfer_segs = 4;

                let draw_corner_shape = |thickness: f32, col: Color| {
                    for i in 0..chamfer_segs {
                        let s0 = t_a * (i as f32 / chamfer_segs as f32);
                        let s1 = t_a * ((i + 1) as f32 / chamfer_segs as f32);
                        let (x0, y0) = bez(s0);
                        let (x1, y1) = bez(s1);
                        draw_line(x0, y0, x1, y1, thickness, col);
                    }
                    draw_line(mid_ax, mid_ay, mid_bx, mid_by, thickness, col);
                    for i in 0..chamfer_segs {
                        let s0 = t_b + (1.0 - t_b) * (i as f32 / chamfer_segs as f32);
                        let s1 = t_b + (1.0 - t_b) * ((i + 1) as f32 / chamfer_segs as f32);
                        let (x0, y0) = bez(s0);
                        let (x1, y1) = bez(s1);
                        draw_line(x0, y0, x1, y1, thickness, col);
                    }
                };

                if xhair.outline_enabled {
                    let ot = thick + (xhair.outline_thickness * sh) / 100.0 * 2.0;
                    let oc = Color::from_rgba(0, 0, 0, 180);
                    draw_line(h_near, corner_y, h_end_x, corner_y, ot, oc);
                    draw_line(corner_x, v_near, corner_x, v_end_y, ot, oc);
                    draw_corner_shape(ot, oc);
                }
                draw_line(h_near, corner_y, h_end_x, corner_y, thick, color);
                draw_line(corner_x, v_near, corner_x, v_end_y, thick, color);
                draw_corner_shape(thick, color);
            }

            if xhair.dot_enabled {
                let dot_alpha = ((1.0 - ads_progress) * 255.0) as u8;
                let dot_color = Color::from_rgba((color.r * 255.0) as u8, (color.g * 255.0) as u8, (color.b * 255.0) as u8, dot_alpha);
                draw_circle(cx, cy, (xhair.dot_size * sh) / 100.0, dot_color);
            }
        }
        CrosshairType::Cross => {
            let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - ads_progress);
            let ads_rest_gap = ((xhair.center_gap * sh) / 100.0) * 0.4 * ads_progress;
            let gap = hip_gap + ads_rest_gap;
            let len = (xhair.line_length * sh) / 100.0;
            let thick = (xhair.line_thickness * sh) / 100.0;

            if xhair.outline_enabled {
                let ot = thick + (xhair.outline_thickness * sh) / 100.0 * 2.0;
                let oc = Color::from_rgba(0, 0, 0, 180);
                draw_rectangle(cx - ot / 2.0, cy - gap - len, ot, len, oc);
                draw_rectangle(cx - ot / 2.0, cy + gap, ot, len, oc);
                draw_rectangle(cx - gap - len, cy - ot / 2.0, len, ot, oc);
                draw_rectangle(cx + gap, cy - ot / 2.0, len, ot, oc);
            }
            draw_rectangle(cx - thick / 2.0, cy - gap - len, thick, len, color);
            draw_rectangle(cx - thick / 2.0, cy + gap, thick, len, color);
            draw_rectangle(cx - gap - len, cy - thick / 2.0, len, thick, color);
            draw_rectangle(cx + gap, cy - thick / 2.0, len, thick, color);

            if xhair.dot_enabled {
                let dot_alpha = ((1.0 - ads_progress) * 255.0) as u8;
                let dot_color = Color::from_rgba((color.r * 255.0) as u8, (color.g * 255.0) as u8, (color.b * 255.0) as u8, dot_alpha);
                draw_circle(cx, cy, (xhair.dot_size * sh) / 100.0, dot_color);
            }
        }
    }

    // Weapon name
    let name_color = if state.preview_light_bg { BLACK } else { WHITE };
    draw_text_ex(weapon.kind.stats().name, x + 10.0, y + 20.0, text_params(16, name_color));
}

//  Shotgun warning

fn draw_shotgun_type_warning(
    x: f32, y: f32, _btn_w: f32, _btn_h: f32, mouse: (f32, f32), clicked: bool,
) -> bool {
    let box_w = 340.0;
    let box_h = 80.0;
    let box_x = x;
    let box_y = y - box_h - 10.0;

    draw_rectangle(box_x, box_y, box_w, box_h, Color::from_rgba(60, 30, 10, 240));
    draw_rectangle_lines(box_x, box_y, box_w, box_h, 2.0, Color::from_rgba(255, 160, 0, 255));
    draw_text_ex("Switching to a non-standard crosshair", box_x + 10.0, box_y + 22.0, text_params(14, Color::from_rgba(255, 200, 80, 255)));
    draw_text_ex("for shotgun may affect gameplay.", box_x + 10.0, box_y + 42.0, text_params(14, Color::from_rgba(255, 200, 80, 255)));

    let ok_w = 50.0;
    let ok_h = 25.0;
    let ok_x = box_x + box_w - ok_w - 10.0;
    let ok_y = box_y + box_h - ok_h - 8.0;

    draw_rectangle(ok_x, ok_y, ok_w, ok_h, Color::from_rgba(80, 150, 220, 255));
    draw_rectangle_lines(ok_x, ok_y, ok_w, ok_h, 1.0, GRAY);
    let ok_tw = measure_game_text("OK", 14);
    draw_text_ex("OK", ok_x + (ok_w - ok_tw) / 2.0, ok_y + 17.0, text_params(14, WHITE));

    clicked && mouse.0 >= ok_x && mouse.0 <= ok_x + ok_w && mouse.1 >= ok_y && mouse.1 <= ok_y + ok_h
}

//  HELPER WIDGETS
fn draw_section_header(label: &str, x: f32, y: f32) -> f32 {
    draw_text_ex(label, x, y + 18.0, text_params(18, Color::from_rgba(100, 180, 255, 255)));
    draw_line(x, y + 25.0, x + 300.0, y + 25.0, 1.0, GRAY);
    35.0
}

fn draw_slider(
    label: &str, value: &mut f32, min: f32, max: f32, step: f32,
    x: f32, y: f32, width: f32,
    mouse: (f32, f32), _clicked: bool, held: bool,
    fill_color: Option<Color>,
) -> f32 {
    let label_w = 100.0;
    let slider_x = x + label_w;
    let slider_w = width - label_w - 50.0;
    let slider_h = 14.0;
    let row_h = 28.0;

    draw_text_ex(label, x, y + 14.0, text_params(14, WHITE));

    draw_rectangle(slider_x, y + 1.0, slider_w, slider_h, Color::from_rgba(35, 35, 35, 255));
    let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
    let fc = fill_color.unwrap_or(Color::from_rgba(80, 150, 220, 255));
    draw_rectangle(slider_x, y + 1.0, slider_w * t, slider_h, fc);
    draw_rectangle_lines(slider_x, y + 1.0, slider_w, slider_h, 1.0, Color::from_rgba(60, 60, 60, 255));

    if held && mouse.1 >= y && mouse.1 <= y + row_h &&
       mouse.0 >= slider_x && mouse.0 <= slider_x + slider_w
    {
        let raw = min + ((mouse.0 - slider_x) / slider_w) * (max - min);
        *value = (raw / step).round() * step;
        *value = value.clamp(min, max);
    }

    let precision = if step >= 1.0 { 0 } else if step >= 0.1 { 1 } else { 2 };
    let txt = format!("{:.*}", precision, *value);
    draw_text_ex(&txt, slider_x + slider_w + 6.0, y + 12.0, text_params(12, GRAY));

    row_h
}

// draw-only version of slider (doesn't handle input, just displays the value)
fn draw_slider_display(
    label: &str, value: f32, min: f32, max: f32,
    x: f32, y: f32, width: f32,
    fill_color: Option<Color>,
) {
    let label_w = 100.0;
    let slider_x = x + label_w;
    let slider_w = width - label_w - 50.0;
    let slider_h = 14.0;

    draw_text_ex(label, x, y + 14.0, text_params(14, WHITE));

    draw_rectangle(slider_x, y + 1.0, slider_w, slider_h, Color::from_rgba(35, 35, 35, 255));
    let t = ((value - min) / (max - min)).clamp(0.0, 1.0);
    let fc = fill_color.unwrap_or(Color::from_rgba(80, 150, 220, 255));
    draw_rectangle(slider_x, y + 1.0, slider_w * t, slider_h, fc);
    draw_rectangle_lines(slider_x, y + 1.0, slider_w, slider_h, 1.0, Color::from_rgba(60, 60, 60, 255));

    let step = if max > 100.0 { 1.0 } else if max > 5.0 { 0.1 } else { 0.05 };
    let precision = if step >= 1.0 { 0 } else if step >= 0.1 { 1 } else { 2 };
    let txt = format!("{:.*}", precision, value);
    draw_text_ex(&txt, slider_x + slider_w + 6.0, y + 12.0, text_params(12, GRAY));
}

fn draw_toggle(
    label: &str, value: &mut bool, x: f32, y: f32, mouse: (f32, f32), clicked: bool,
) -> f32 {
    let box_size = 18.0;
    let row_h = 28.0;
    let checkbox_color = if *value { Color::from_rgba(80, 150, 220, 255) } else { Color::from_rgba(35, 35, 35, 255) };
    draw_rectangle(x, y, box_size, box_size, checkbox_color);
    draw_rectangle_lines(x, y, box_size, box_size, 1.0, Color::from_rgba(60, 60, 60, 255));
    if clicked && mouse.0 >= x && mouse.0 <= x + box_size && mouse.1 >= y && mouse.1 <= y + box_size {
        *value = !*value;
    }
    draw_text_ex(label, x + box_size + 8.0, y + 14.0, text_params(14, WHITE));
    row_h
}

fn draw_toggle_display(label: &str, value: bool, x: f32, y: f32) {
    let box_size = 18.0;
    let checkbox_color = if value { Color::from_rgba(80, 150, 220, 255) } else { Color::from_rgba(35, 35, 35, 255) };
    draw_rectangle(x, y, box_size, box_size, checkbox_color);
    draw_rectangle_lines(x, y, box_size, box_size, 1.0, Color::from_rgba(60, 60, 60, 255));
    draw_text_ex(label, x + box_size + 8.0, y + 14.0, text_params(14, WHITE));
}

fn draw_dropdown(
    label: &str, options: &[&str], selected: &mut usize,
    x: f32, y: f32, width: f32,
    mouse: (f32, f32), clicked: bool,
) -> f32 {
    let label_w = 100.0;
    let dropdown_x = x + label_w;
    let dropdown_w = width - label_w - 10.0;
    let dropdown_h = 28.0;

    draw_text_ex(label, x, y + 18.0, text_params(14, WHITE));
    draw_rectangle(dropdown_x, y, dropdown_w, dropdown_h, Color::from_rgba(35, 35, 35, 255));
    draw_rectangle_lines(dropdown_x, y, dropdown_w, dropdown_h, 1.0, Color::from_rgba(60, 60, 60, 255));

    let current_text = options[*selected];
    let text_w = measure_game_text(current_text, 14);
    draw_text_ex(current_text, dropdown_x + (dropdown_w - text_w) / 2.0, y + 18.0, text_params(14, WHITE));

    if clicked && mouse.0 >= dropdown_x && mouse.0 <= dropdown_x + dropdown_w &&
       mouse.1 >= y && mouse.1 <= y + dropdown_h
    {
        *selected = (*selected + 1) % options.len();
    }
    32.0
}

fn draw_dropdown_display(label: &str, value: &str, x: f32, y: f32, width: f32) {
    let label_w = 100.0;
    let dropdown_x = x + label_w;
    let dropdown_w = width - label_w - 10.0;
    let dropdown_h = 28.0;

    draw_text_ex(label, x, y + 18.0, text_params(14, WHITE));
    draw_rectangle(dropdown_x, y, dropdown_w, dropdown_h, Color::from_rgba(35, 35, 35, 255));
    draw_rectangle_lines(dropdown_x, y, dropdown_w, dropdown_h, 1.0, Color::from_rgba(60, 60, 60, 255));

    let text_w = measure_game_text(value, 14);
    draw_text_ex(value, dropdown_x + (dropdown_w - text_w) / 2.0, y + 18.0, text_params(14, WHITE));
}
