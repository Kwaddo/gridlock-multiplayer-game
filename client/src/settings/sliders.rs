use crate::core::GameConfig;
use crate::settings::fov;
use crate::settings::sensitivity;
use crate::ui::fonts::{measure_game_text, text_params};
use macroquad::prelude::*;

const GUI_SCALE_OPTIONS: &[(&str, u8)] = &[("Auto", 0), ("1", 1), ("2", 2), ("3", 3)];

pub fn update_sliders(
    config: &mut GameConfig,
    slider_x: f32,
    slider_w: f32,
    btn_size: f32,
    panel_top: f32,
    mouse: (f32, f32),
    clicked: bool,
    held: bool,
    s: f32,
) {
    // GUI Scale row
    let gui_row_y = panel_top + 50.0 * s;
    let gui_btn_w = 60.0 * s;
    let gui_btn_h = 28.0 * s;
    let gui_btn_gap = 8.0 * s;
    let gui_start_x = slider_x;

    if clicked {
        for (i, &(_, value)) in GUI_SCALE_OPTIONS.iter().enumerate() {
            let bx = gui_start_x + (gui_btn_w + gui_btn_gap) * (i as f32);
            if mouse.0 >= bx
                && mouse.0 <= bx + gui_btn_w
                && mouse.1 >= gui_row_y
                && mouse.1 <= gui_row_y + gui_btn_h
            {
                config.gui_scale = value;
            }
        }
    }

    // Sensitivity row
    let row_y = gui_row_y + gui_btn_h + 20.0 * s;
    sensitivity::update(
        &mut config.sensitivity,
        slider_x,
        slider_w,
        btn_size,
        row_y,
        mouse,
        clicked,
        held,
        s,
    );

    // FOV row
    let row_y2 = row_y + 60.0 * s;
    fov::update(
        &mut config.fov,
        slider_x,
        slider_w,
        btn_size,
        row_y2,
        mouse,
        clicked,
        held,
        s,
    );

    // Audio mute toggle row
    let row_y3 = row_y2 + 60.0 * s;
    let toggle_w = 130.0 * s;
    let toggle_h = 28.0 * s;
    if clicked
        && mouse.0 >= slider_x
        && mouse.0 <= slider_x + toggle_w
        && mouse.1 >= row_y3
        && mouse.1 <= row_y3 + toggle_h
    {
        config.mute_audio = !config.mute_audio;
    }
}

pub fn draw_sliders(
    config: &GameConfig,
    label_x: f32,
    slider_x: f32,
    slider_w: f32,
    btn_size: f32,
    panel_top: f32,
    s: f32,
) {
    let label_fs = (18.0 * s) as u16;
    let btn_fs = (14.0 * s) as u16;

    // GUI Scale row
    let gui_row_y = panel_top + 50.0 * s;
    draw_text_ex(
        "GUI Scale",
        label_x,
        gui_row_y + 20.0 * s,
        text_params(label_fs, WHITE),
    );

    let gui_btn_w = 60.0 * s;
    let gui_btn_h = 28.0 * s;
    let gui_btn_gap = 8.0 * s;
    let gui_start_x = slider_x;

    let effective = config.effective_gui_scale();

    for (i, &(label, value)) in GUI_SCALE_OPTIONS.iter().enumerate() {
        let bx = gui_start_x + (gui_btn_w + gui_btn_gap) * (i as f32);
        let is_active = config.gui_scale == value;

        let bg = if is_active {
            Color::from_rgba(80, 150, 220, 255)
        } else {
            Color::from_rgba(45, 45, 45, 255)
        };

        draw_rectangle(bx, gui_row_y, gui_btn_w, gui_btn_h, bg);
        draw_rectangle_lines(bx, gui_row_y, gui_btn_w, gui_btn_h, 1.0, GRAY);

        // For auto, show the resolved value
        let display = if value == 0 {
            format!("Auto({})", effective)
        } else {
            label.to_string()
        };

        let tw = measure_game_text(&display, btn_fs);
        draw_text_ex(
            &display,
            bx + (gui_btn_w - tw) / 2.0,
            gui_row_y + gui_btn_h * 0.68,
            text_params(btn_fs, WHITE),
        );
    }

    // Sensitivity row
    let row_y = gui_row_y + gui_btn_h + 20.0 * s;
    sensitivity::draw(
        config.sensitivity,
        label_x,
        slider_x,
        slider_w,
        btn_size,
        row_y,
        s,
    );

    // FOV row
    let row_y2 = row_y + 60.0 * s;
    fov::draw(config.fov, label_x, slider_x, slider_w, btn_size, row_y2, s);

    // Audio mute row
    let row_y3 = row_y2 + 60.0 * s;
    draw_text_ex(
        "Audio",
        label_x,
        row_y3 + 20.0 * s,
        text_params((18.0 * s) as u16, WHITE),
    );
    let toggle_w = 130.0 * s;
    let toggle_h = 28.0 * s;
    let bg = if config.mute_audio {
        Color::from_rgba(220, 90, 90, 255)
    } else {
        Color::from_rgba(80, 150, 220, 255)
    };
    draw_rectangle(slider_x, row_y3, toggle_w, toggle_h, bg);
    draw_rectangle_lines(slider_x, row_y3, toggle_w, toggle_h, 1.0, GRAY);
    let txt = if config.mute_audio { "Muted" } else { "Unmuted" };
    let tw = measure_game_text(txt, (14.0 * s) as u16);
    draw_text_ex(
        txt,
        slider_x + (toggle_w - tw) / 2.0,
        row_y3 + toggle_h * 0.68,
        text_params((14.0 * s) as u16, WHITE),
    );
}
