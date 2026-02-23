use crate::core::GameConfig;
use crate::settings::fov;
use crate::settings::sensitivity;
use crate::ui::fonts::{measure_game_text, text_params};
use macroquad::prelude::*;

const GUI_SCALE_OPTIONS: &[(&str, u8)] = &[("Auto", 0), ("1", 1), ("2", 2), ("3", 3)];
const MINIMAP_SIZE_OPTIONS: &[(&str, u8)] = &[("Small", 0), ("Medium", 1), ("Large", 2)];
const MINIMAP_SHAPE_OPTIONS: &[(&str, u8)] = &[("Circle", 0), ("Square", 1)];

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

    // Minimap Size row
    let minimap_row_y = row_y2 + 60.0 * s;
    if clicked {
        for (i, &(_, value)) in MINIMAP_SIZE_OPTIONS.iter().enumerate() {
            let bx = gui_start_x + (gui_btn_w + gui_btn_gap) * (i as f32);
            if mouse.0 >= bx
                && mouse.0 <= bx + gui_btn_w
                && mouse.1 >= minimap_row_y
                && mouse.1 <= minimap_row_y + gui_btn_h
            {
                config.minimap_size = value;
            }
        }
    }

    // Minimap Shape row
    let shape_row_y = minimap_row_y + gui_btn_h + 12.0 * s;
    if clicked {
        for (i, &(_, value)) in MINIMAP_SHAPE_OPTIONS.iter().enumerate() {
            let bx = gui_start_x + (gui_btn_w + gui_btn_gap) * (i as f32);
            if mouse.0 >= bx
                && mouse.0 <= bx + gui_btn_w
                && mouse.1 >= shape_row_y
                && mouse.1 <= shape_row_y + gui_btn_h
            {
                config.minimap_shape = value;
            }
        }
    }

    // Match Time override / infinite toggle row
    let time_row_y = shape_row_y + gui_btn_h + 18.0 * s;
    if clicked {
        // Default (use server)
        let bx0 = gui_start_x;
        if mouse.0 >= bx0
            && mouse.0 <= bx0 + gui_btn_w
            && mouse.1 >= time_row_y
            && mouse.1 <= time_row_y + gui_btn_h
        {
            config.match_time_override_secs = None;
            config.hide_match_timer = false;
        }
        // 15 minutes
        let bx1 = gui_start_x + (gui_btn_w + gui_btn_gap) * 1.0;
        if mouse.0 >= bx1
            && mouse.0 <= bx1 + gui_btn_w
            && mouse.1 >= time_row_y
            && mouse.1 <= time_row_y + gui_btn_h
        {
            config.match_time_override_secs = Some(15 * 60);
            config.hide_match_timer = false;
        }
        // 30 minutes
        let bx2 = gui_start_x + (gui_btn_w + gui_btn_gap) * 2.0;
        if mouse.0 >= bx2
            && mouse.0 <= bx2 + gui_btn_w
            && mouse.1 >= time_row_y
            && mouse.1 <= time_row_y + gui_btn_h
        {
            config.match_time_override_secs = Some(30 * 60);
            config.hide_match_timer = false;
        }
        // Infinite (hide)
        let bx3 = gui_start_x + (gui_btn_w + gui_btn_gap) * 3.0;
        if mouse.0 >= bx3
            && mouse.0 <= bx3 + gui_btn_w
            && mouse.1 >= time_row_y
            && mouse.1 <= time_row_y + gui_btn_h
        {
            config.match_time_override_secs = None;
            config.hide_match_timer = true;
        }
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
    // Recompute layout values (must match update)
    let gui_row_y = panel_top + 50.0 * s;
    let gui_btn_w = 60.0 * s;
    let gui_btn_h = 28.0 * s;
    let gui_btn_gap = 8.0 * s;
    let gui_start_x = slider_x;

    let label_fs = (18.0 * s) as u16;
    let btn_fs = (20.0 * s) as u16;

    // GUI Scale row (draw)
    draw_text_ex(
        "UI Scale",
        label_x,
        gui_row_y + 16.0 * s,
        text_params(label_fs, WHITE),
    );
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

    // Sensitivity row (draw)
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

    // FOV row (draw)
    let row_y2 = row_y + 60.0 * s;
    fov::draw(config.fov, label_x, slider_x, slider_w, btn_size, row_y2, s);

    // Minimap Size row (draw)
    let minimap_row_y = row_y2 + 60.0 * s;
    draw_text_ex(
        "Minimap Size",
        label_x,
        minimap_row_y + 20.0 * s,
        text_params(label_fs, WHITE),
    );
    draw_button_row(
        MINIMAP_SIZE_OPTIONS,
        config.minimap_size,
        slider_x,
        minimap_row_y,
        gui_btn_w,
        gui_btn_h,
        gui_btn_gap,
        btn_fs,
    );

    // Minimap Shape row (draw)
    let shape_row_y = minimap_row_y + gui_btn_h + 12.0 * s;
    draw_text_ex(
        "Minimap Shape",
        label_x,
        shape_row_y + 20.0 * s,
        text_params(label_fs, WHITE),
    );
    draw_button_row(
        MINIMAP_SHAPE_OPTIONS,
        config.minimap_shape,
        slider_x,
        shape_row_y,
        gui_btn_w,
        gui_btn_h,
        gui_btn_gap,
        btn_fs,
    );

    // Match Time row (draw)
    let time_row_y = shape_row_y + gui_btn_h + 18.0 * s;
    draw_text_ex(
        "Match Time",
        label_x,
        time_row_y + 20.0 * s,
        text_params(label_fs, WHITE),
    );
    let options = ["Default", "15m", "30m", "Infinite"];
    for (i, &label) in options.iter().enumerate() {
        let bx = slider_x + (gui_btn_w + gui_btn_gap) * (i as f32);
        let active = if label == "Infinite" {
            config.hide_match_timer
        } else if label == "Default" {
            config.match_time_override_secs.is_none() && !config.hide_match_timer
        } else if label == "15m" {
            config.match_time_override_secs == Some(15 * 60)
        } else if label == "30m" {
            config.match_time_override_secs == Some(30 * 60)
        } else {
            false
        };

        let bg = if active {
            Color::from_rgba(80, 150, 220, 255)
        } else {
            Color::from_rgba(45, 45, 45, 255)
        };
        draw_rectangle(bx, time_row_y, gui_btn_w, gui_btn_h, bg);
        draw_rectangle_lines(bx, time_row_y, gui_btn_w, gui_btn_h, 1.0, GRAY);
        let tw = measure_game_text(label, btn_fs);
        draw_text_ex(
            label,
            bx + (gui_btn_w - tw) / 2.0,
            time_row_y + gui_btn_h * 0.68,
            text_params(btn_fs, WHITE),
        );
    }
}

fn draw_button_row(
    options: &[(&str, u8)],
    active: u8,
    start_x: f32,
    row_y: f32,
    btn_w: f32,
    btn_h: f32,
    gap: f32,
    fs: u16,
) {
    for (i, &(label, value)) in options.iter().enumerate() {
        let bx = start_x + (btn_w + gap) * (i as f32);
        let bg = if active == value {
            Color::from_rgba(80, 150, 220, 255)
        } else {
            Color::from_rgba(45, 45, 45, 255)
        };
        draw_rectangle(bx, row_y, btn_w, btn_h, bg);
        draw_rectangle_lines(bx, row_y, btn_w, btn_h, 1.0, GRAY);
        let tw = measure_game_text(label, fs);
        draw_text_ex(
            label,
            bx + (btn_w - tw) / 2.0,
            row_y + btn_h * 0.68,
            text_params(fs, WHITE),
        );
    }
}
