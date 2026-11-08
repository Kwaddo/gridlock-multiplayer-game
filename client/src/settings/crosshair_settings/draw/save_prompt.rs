use crate::ui::fonts::{measure_game_text, text_params};
use crate::weapons::crosshair_presets::CrosshairPresetManager;
use macroquad::prelude::*;

use super::super::shared::CrosshairSettingsPreview;

pub fn draw_crosshair_save_prompt(
    state: &CrosshairSettingsPreview,
    presets: &CrosshairPresetManager,
) {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::from_rgba(0, 0, 0, 150),
    );

    let pw = 400.0;
    let ph = 200.0;
    let px = (screen_width() - pw) / 2.0;
    let py = (screen_height() - ph) / 2.0;

    draw_rectangle(px, py, pw, ph, Color::from_rgba(40, 40, 40, 255));
    draw_rectangle_lines(px, py, pw, ph, 2.0, WHITE);

    let btn_w = 120.0;
    let btn_h = 35.0;
    let btn_y = py + ph - btn_h - 20.0;

    if state.name_conflict_index.is_some() {
        let title = "Name Already Exists";
        let title_w = measure_game_text(title, 20);
        draw_text_ex(
            title,
            px + (pw - title_w) / 2.0,
            py + 35.0,
            text_params(20, YELLOW),
        );

        let msg = "A preset with this name already exists.";
        let msg_w = measure_game_text(msg, 14);
        draw_text_ex(
            msg,
            px + (pw - msg_w) / 2.0,
            py + 65.0,
            text_params(14, WHITE),
        );

        let conflict_btn_w = 110.0;

        let ow_x = px + 20.0;
        draw_rectangle(
            ow_x,
            btn_y,
            conflict_btn_w,
            btn_h,
            Color::from_rgba(80, 80, 30, 255),
        );
        draw_rectangle_lines(ow_x, btn_y, conflict_btn_w, btn_h, 1.0, WHITE);
        let ow_tw = measure_game_text("Overwrite", 16);
        draw_text_ex(
            "Overwrite",
            ow_x + (conflict_btn_w - ow_tw) / 2.0,
            btn_y + 24.0,
            text_params(16, WHITE),
        );

        let rename_x = px + (pw - conflict_btn_w) / 2.0;
        draw_rectangle(
            rename_x,
            btn_y,
            conflict_btn_w,
            btn_h,
            Color::from_rgba(30, 80, 30, 255),
        );
        draw_rectangle_lines(rename_x, btn_y, conflict_btn_w, btn_h, 1.0, WHITE);
        let rn_tw = measure_game_text("Rename", 16);
        draw_text_ex(
            "Rename",
            rename_x + (conflict_btn_w - rn_tw) / 2.0,
            btn_y + 24.0,
            text_params(16, WHITE),
        );

        let cancel_x = px + pw - conflict_btn_w - 20.0;
        draw_rectangle(
            cancel_x,
            btn_y,
            conflict_btn_w,
            btn_h,
            Color::from_rgba(80, 30, 30, 255),
        );
        draw_rectangle_lines(cancel_x, btn_y, conflict_btn_w, btn_h, 1.0, WHITE);
        let cn_tw = measure_game_text("Cancel", 16);
        draw_text_ex(
            "Cancel",
            cancel_x + (conflict_btn_w - cn_tw) / 2.0,
            btn_y + 24.0,
            text_params(16, WHITE),
        );

        return;
    }

    if state.naming_mode {
        let title = "Preset Name";
        let title_w = measure_game_text(title, 20);
        draw_text_ex(
            title,
            px + (pw - title_w) / 2.0,
            py + 35.0,
            text_params(20, Color::from_rgba(100, 180, 255, 255)),
        );

        let input_x = px + 30.0;
        let input_y = py + 60.0;
        let input_w = pw - 60.0;
        draw_rectangle(
            input_x,
            input_y,
            input_w,
            32.0,
            Color::from_rgba(20, 20, 20, 255),
        );
        draw_rectangle_lines(input_x, input_y, input_w, 32.0, 1.0, WHITE);

        let cursor = if (get_time() * 2.0) as u32 % 2 == 0 {
            "|"
        } else {
            ""
        };
        let display = format!("{}{}", state.name_input, cursor);
        draw_text_ex(
            &display,
            input_x + 8.0,
            input_y + 22.0,
            text_params(16, WHITE),
        );

        if !state.name_error.is_empty() {
            draw_text_ex(
                &state.name_error,
                input_x,
                input_y + 50.0,
                text_params(14, Color::from_rgba(255, 80, 80, 255)),
            );
        }

        let hint = "Enter to save, Esc to go back";
        let hint_w = measure_game_text(hint, 12);
        draw_text_ex(
            hint,
            px + (pw - hint_w) / 2.0,
            py + ph - 15.0,
            text_params(12, GRAY),
        );

        return;
    }

    let title = "Save Crosshair Preset";
    let title_w = measure_game_text(title, 20);
    draw_text_ex(
        title,
        px + (pw - title_w) / 2.0,
        py + 35.0,
        text_params(20, YELLOW),
    );

    let preset_name = if state.selected_preset < presets.presets.len() {
        &presets.presets[state.selected_preset].name
    } else {
        "None"
    };
    let msg = format!("Current: {}", preset_name);
    let msg_w = measure_game_text(&msg, 14);
    draw_text_ex(
        &msg,
        px + (pw - msg_w) / 2.0,
        py + 65.0,
        text_params(14, GRAY),
    );

    let is_custom = state.selected_preset < presets.presets.len()
        && !presets.presets[state.selected_preset].is_default;

    if is_custom {
        let gap = 15.0;
        let total = btn_w * 3.0 + gap * 2.0;
        let start_x = px + (pw - total) / 2.0;

        draw_rectangle(
            start_x,
            btn_y,
            btn_w,
            btn_h,
            Color::from_rgba(30, 80, 30, 255),
        );
        draw_rectangle_lines(start_x, btn_y, btn_w, btn_h, 1.0, WHITE);
        let sa_tw = measure_game_text("Save As New", 14);
        draw_text_ex(
            "Save As New",
            start_x + (btn_w - sa_tw) / 2.0,
            btn_y + 24.0,
            text_params(14, WHITE),
        );

        let ow_x = start_x + btn_w + gap;
        draw_rectangle(ow_x, btn_y, btn_w, btn_h, Color::from_rgba(80, 80, 30, 255));
        draw_rectangle_lines(ow_x, btn_y, btn_w, btn_h, 1.0, WHITE);
        let ow_tw = measure_game_text("Overwrite", 14);
        draw_text_ex(
            "Overwrite",
            ow_x + (btn_w - ow_tw) / 2.0,
            btn_y + 24.0,
            text_params(14, WHITE),
        );

        let ds_x = ow_x + btn_w + gap;
        draw_rectangle(ds_x, btn_y, btn_w, btn_h, Color::from_rgba(80, 30, 30, 255));
        draw_rectangle_lines(ds_x, btn_y, btn_w, btn_h, 1.0, WHITE);
        let ds_tw = measure_game_text("Don't Save", 14);
        draw_text_ex(
            "Don't Save",
            ds_x + (btn_w - ds_tw) / 2.0,
            btn_y + 24.0,
            text_params(14, WHITE),
        );
    } else {
        let gap = 20.0;
        let total = btn_w * 2.0 + gap;
        let start_x = px + (pw - total) / 2.0;

        draw_rectangle(
            start_x,
            btn_y,
            btn_w,
            btn_h,
            Color::from_rgba(30, 80, 30, 255),
        );
        draw_rectangle_lines(start_x, btn_y, btn_w, btn_h, 1.0, WHITE);
        let sa_tw = measure_game_text("Save As New", 14);
        draw_text_ex(
            "Save As New",
            start_x + (btn_w - sa_tw) / 2.0,
            btn_y + 24.0,
            text_params(14, WHITE),
        );

        let ds_x = start_x + btn_w + gap;
        draw_rectangle(ds_x, btn_y, btn_w, btn_h, Color::from_rgba(80, 30, 30, 255));
        draw_rectangle_lines(ds_x, btn_y, btn_w, btn_h, 1.0, WHITE);
        let ds_tw = measure_game_text("Don't Save", 14);
        draw_text_ex(
            "Don't Save",
            ds_x + (btn_w - ds_tw) / 2.0,
            btn_y + 24.0,
            text_params(14, WHITE),
        );
    }
}
