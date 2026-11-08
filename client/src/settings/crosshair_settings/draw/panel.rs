use crate::ui::fonts::{measure_game_text, text_params};
use crate::weapons::crosshair_presets::CrosshairPresetManager;
use crate::weapons::stats::WeaponCategory;
use crate::weapons::WeaponInventory;
use macroquad::prelude::*;
use std::collections::HashMap;

use super::super::shared::{crosshair_type_name, hsv_to_rgb, CrosshairSettingsPreview};
use super::preview::draw_crosshair_preview;
use super::warning::draw_shotgun_type_warning;
use super::widgets::{
    draw_dropdown_display, draw_section_header, draw_slider_display, draw_toggle_display,
};

pub fn draw_crosshair_settings(
    state: &CrosshairSettingsPreview,
    weapons: &WeaponInventory,
    presets: &CrosshairPresetManager,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    _assets: &HashMap<String, Texture2D>,
    _mouse: (f32, f32),
    s: f32,
) {
    let pad = 15.0 * s;
    let x = x + pad;
    let w = w - pad * 2.0;

    // Apply scroll offset
    let y = y - state.crosshair_scroll;

    draw_section_header("Crosshair Settings", x, y + 15.0 * s, s);

    let btn_y = y + 50.0 * s;
    let btn_w = 90.0 * s;
    let btn_h = 35.0 * s;
    let btn_gap = 10.0 * s;
    let btn_fs = (14.0 * s) as u16;

    for (i, kind) in WeaponCategory::ALL.iter().enumerate() {
        let btn_x = x + (btn_w + btn_gap) * (i as f32);
        let btn_color = if kind == &state.preview_weapon {
            Color::from_rgba(80, 150, 220, 255)
        } else {
            Color::from_rgba(40, 40, 40, 255)
        };
        draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_color);
        draw_rectangle_lines(btn_x, btn_y, btn_w, btn_h, 1.0, GRAY);
        let name: &str = kind.display_name();
        let text_w = measure_game_text(name, btn_fs);
        draw_text_ex(
            name,
            btn_x + (btn_w - text_w) / 2.0,
            btn_y + btn_h * 0.63,
            text_params(btn_fs, WHITE),
        );
    }

    if state.preview_weapon == WeaponCategory::Sniper {
        let sniper_y = btn_y + btn_h + 60.0 * s;
        let sniper_fs = (16.0 * s) as u16;
        draw_text_ex(
            "Sniper scope customization coming soon",
            x + 20.0 * s,
            sniper_y,
            text_params(sniper_fs, YELLOW),
        );
        return;
    }

    let left_w = (w * 0.55) - 20.0 * s;

    let preset_y = btn_y + btn_h + 15.0 * s;
    let preset_dropdown_w = left_w - 100.0 * s;
    let preset_btn_w = 75.0 * s;
    let preset_btn_gap = 5.0 * s;
    let small_fs = (14.0 * s) as u16;
    let tiny_fs = (12.0 * s) as u16;

    let preset_name = if state.selected_preset < presets.presets.len() {
        &presets.presets[state.selected_preset].name
    } else {
        "None"
    };
    let is_custom = state.selected_preset < presets.presets.len()
        && !presets.presets[state.selected_preset].is_default;

    let dropdown_h = 30.0 * s;
    draw_rectangle(
        x,
        preset_y,
        preset_dropdown_w,
        dropdown_h,
        Color::from_rgba(35, 35, 35, 255),
    );
    draw_rectangle_lines(
        x,
        preset_y,
        preset_dropdown_w,
        dropdown_h,
        1.0,
        Color::from_rgba(80, 80, 80, 255),
    );
    let preset_tw = measure_game_text(preset_name, small_fs);
    draw_text_ex(
        preset_name,
        x + (preset_dropdown_w - preset_tw) / 2.0,
        preset_y + dropdown_h * 0.67,
        text_params(small_fs, WHITE),
    );
    draw_text_ex("<", x + 6.0 * s, preset_y + dropdown_h * 0.67, text_params(small_fs, GRAY));
    let arrow_r = measure_game_text(">", small_fs);
    draw_text_ex(
        ">",
        x + preset_dropdown_w - arrow_r - 6.0 * s,
        preset_y + dropdown_h * 0.67,
        text_params(small_fs, GRAY),
    );

    let delete_x = x + preset_dropdown_w + preset_btn_gap;
    let del_color = if is_custom {
        Color::from_rgba(80, 30, 30, 255)
    } else {
        Color::from_rgba(30, 30, 30, 255)
    };
    draw_rectangle(delete_x, preset_y, preset_btn_w, dropdown_h, del_color);
    draw_rectangle_lines(delete_x, preset_y, preset_btn_w, dropdown_h, 1.0, GRAY);
    let del_tw = measure_game_text("Delete", tiny_fs);
    let del_text_color = if is_custom {
        WHITE
    } else {
        Color::from_rgba(80, 80, 80, 255)
    };
    draw_text_ex(
        "Delete",
        delete_x + (preset_btn_w - del_tw) / 2.0,
        preset_y + dropdown_h * 0.63,
        text_params(tiny_fs, del_text_color),
    );

    let slider_x = x;
    let mut slider_y = preset_y + 45.0 * s;
    let slider_w = left_w;
    let row_step = 28.0 * s;
    let section_step = 33.0 * s;

    let _cat_idx = weapons.index_for_category(state.preview_weapon);
    let weapon = &weapons.weapons[_cat_idx];
    let ch = &weapon.crosshair;

    draw_toggle_display("Enable dot", ch.dot_enabled, slider_x, slider_y, s);
    slider_y += row_step;
    draw_slider_display("Gap", ch.center_gap, 0.0, 10.0, slider_x, slider_y, slider_w, None, s);
    slider_y += row_step;
    draw_slider_display("Length", ch.line_length, 0.0, 10.0, slider_x, slider_y, slider_w, None, s);
    slider_y += row_step;
    draw_slider_display("Thickness", ch.line_thickness, 0.0, 2.0, slider_x, slider_y, slider_w, None, s);
    slider_y += row_step;
    draw_slider_display("Dot size", ch.dot_size, 0.0, 5.0, slider_x, slider_y, slider_w, None, s);
    slider_y += section_step;
    draw_toggle_display("Outline", ch.outline_enabled, slider_x, slider_y, s);
    slider_y += row_step;
    draw_slider_display(
        "Thickness",
        ch.outline_thickness,
        0.0,
        2.0,
        slider_x,
        slider_y,
        slider_w,
        None,
        s,
    );
    slider_y += section_step;

    let type_name = crosshair_type_name(ch.crosshair_type);
    draw_dropdown_display("Crosshair Type", type_name, slider_x, slider_y, slider_w, s);
    slider_y += 40.0 * s;

    let color_fs = (14.0 * s) as u16;
    draw_text_ex(
        "COLOR",
        slider_x,
        slider_y + 14.0 * s,
        text_params(color_fs, Color::from_rgba(100, 180, 255, 255)),
    );
    slider_y += 20.0 * s;

    let picker_size = 120.0 * s;
    let hue_bar_w = 20.0 * s;
    let sv_x = slider_x;
    let sv_y = slider_y;

    let steps = 20u32;
    let cell_w = picker_size / steps as f32;
    let cell_h = picker_size / steps as f32;
    for sy in 0..steps {
        for sx in 0..steps {
            let sv_s = (sx as f32 + 0.5) / steps as f32;
            let v = 1.0 - (sy as f32 + 0.5) / steps as f32;
            let (r, g, b) = hsv_to_rgb(state.hue, sv_s, v);
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

    let cursor_x = sv_x + state.sat * picker_size;
    let cursor_y = sv_y + (1.0 - state.val) * picker_size;
    draw_circle_lines(cursor_x, cursor_y, 5.0 * s, 2.0, WHITE);
    draw_circle_lines(cursor_x, cursor_y, 4.0 * s, 1.0, BLACK);

    let hue_x = sv_x + picker_size + 10.0 * s;
    let hue_steps = 36u32;
    let hue_cell_h = picker_size / hue_steps as f32;
    for i in 0..hue_steps {
        let h = (i as f32 / hue_steps as f32) * 360.0;
        let (r, g, b) = hsv_to_rgb(h, 1.0, 1.0);
        draw_rectangle(
            hue_x,
            sv_y + i as f32 * hue_cell_h,
            hue_bar_w,
            hue_cell_h + 1.0,
            Color::new(r, g, b, 1.0),
        );
    }
    draw_rectangle_lines(hue_x, sv_y, hue_bar_w, picker_size, 1.0, GRAY);

    let hue_cursor_y = sv_y + (state.hue / 360.0) * picker_size;
    draw_rectangle(hue_x - 2.0, hue_cursor_y - 2.0, hue_bar_w + 4.0, 4.0, WHITE);
    draw_rectangle_lines(
        hue_x - 2.0,
        hue_cursor_y - 2.0,
        hue_bar_w + 4.0,
        4.0,
        1.0,
        BLACK,
    );

    let swatch_x = hue_x + hue_bar_w + 15.0 * s;
    let swatch_size = 40.0 * s;
    let check = 10.0 * s;
    for cy_i in 0..4 {
        for cx_i in 0..4 {
            let col = if (cx_i + cy_i) % 2 == 0 {
                Color::from_rgba(180, 180, 180, 255)
            } else {
                Color::from_rgba(120, 120, 120, 255)
            };
            draw_rectangle(
                swatch_x + cx_i as f32 * check,
                sv_y + cy_i as f32 * check,
                check,
                check,
                col,
            );
        }
    }
    let (pr, pg, pb) = hsv_to_rgb(state.hue, state.sat, state.val);
    draw_rectangle(
        swatch_x,
        sv_y,
        swatch_size,
        swatch_size,
        Color::new(pr, pg, pb, state.alpha_val / 255.0),
    );
    draw_rectangle_lines(swatch_x, sv_y, swatch_size, swatch_size, 1.0, WHITE);

    let alpha_y = sv_y + picker_size + 10.0 * s;
    draw_slider_display(
        "Alpha",
        state.alpha_val,
        0.0,
        255.0,
        slider_x,
        alpha_y,
        picker_size + hue_bar_w + 10.0 * s,
        Some(Color::from_rgba(200, 200, 200, 255)),
        s,
    );

    let preview_x = x + w * 0.55 + 20.0 * s;
    let preview_y = btn_y + btn_h + 15.0 * s;
    let preview_w = w * 0.45 - 40.0 * s;
    let preview_h = (h - 150.0 * s).max(100.0 * s);

    draw_crosshair_preview(weapon, state, preview_x, preview_y, preview_w, preview_h, _assets, s);

    let btn_row_y = preview_y + preview_h + 10.0 * s;
    let small_btn_w = 90.0 * s;
    let small_btn_h = 30.0 * s;
    let small_gap = 8.0 * s;

    let ads_color = if state.preview_ads {
        Color::from_rgba(80, 150, 220, 255)
    } else {
        Color::from_rgba(40, 40, 40, 255)
    };
    draw_rectangle(preview_x, btn_row_y, small_btn_w, small_btn_h, ads_color);
    draw_rectangle_lines(preview_x, btn_row_y, small_btn_w, small_btn_h, 1.0, GRAY);
    let ads_text = if state.preview_ads { "ADS: ON" } else { "ADS: OFF" };
    let ads_tw = measure_game_text(ads_text, tiny_fs);
    draw_text_ex(
        ads_text,
        preview_x + (small_btn_w - ads_tw) / 2.0,
        btn_row_y + small_btn_h * 0.67,
        text_params(tiny_fs, WHITE),
    );

    let bg_btn_x = preview_x + small_btn_w + small_gap;
    let bg_color = if state.preview_light_bg {
        Color::from_rgba(180, 180, 180, 255)
    } else {
        Color::from_rgba(40, 40, 40, 255)
    };
    draw_rectangle(bg_btn_x, btn_row_y, small_btn_w, small_btn_h, bg_color);
    draw_rectangle_lines(bg_btn_x, btn_row_y, small_btn_w, small_btn_h, 1.0, GRAY);
    let bg_text = if state.preview_light_bg { "Light" } else { "Dark" };
    let bg_tw = measure_game_text(bg_text, tiny_fs);
    let bg_text_color = if state.preview_light_bg { BLACK } else { WHITE };
    draw_text_ex(
        bg_text,
        bg_btn_x + (small_btn_w - bg_tw) / 2.0,
        btn_row_y + small_btn_h * 0.67,
        text_params(tiny_fs, bg_text_color),
    );

    let reset_btn_x = bg_btn_x + small_btn_w + small_gap;
    draw_rectangle(
        reset_btn_x,
        btn_row_y,
        small_btn_w,
        small_btn_h,
        Color::from_rgba(60, 40, 40, 255),
    );
    draw_rectangle_lines(reset_btn_x, btn_row_y, small_btn_w, small_btn_h, 1.0, GRAY);
    let reset_tw = measure_game_text("Reset", tiny_fs);
    draw_text_ex(
        "Reset",
        reset_btn_x + (small_btn_w - reset_tw) / 2.0,
        btn_row_y + small_btn_h * 0.67,
        text_params(tiny_fs, WHITE),
    );

    if state.preview_weapon == WeaponCategory::Shotgun && state.shotgun_type_warning {
        draw_shotgun_type_warning(preview_x, btn_row_y, 0.0, 0.0, _mouse, false, s);
    }
}
