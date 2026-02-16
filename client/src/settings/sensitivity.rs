use crate::ui::fonts::text_params;
use macroquad::prelude::*;

pub const SENS_MIN: f32 = 10.0;
pub const SENS_MAX: f32 = 100.0;
pub const SENS_STEP: f32 = 1.0;

pub fn update(
    value: &mut f32,
    slider_x: f32,
    slider_w: f32,
    btn_size: f32,
    row_y: f32,
    mouse: (f32, f32),
    clicked: bool,
    held: bool,
    s: f32,
) {
    let slider_h = 20.0 * s;

    // < button
    let lt_x = slider_x - btn_size - 5.0 * s;
    if clicked
        && mouse.0 >= lt_x
        && mouse.0 <= lt_x + btn_size
        && mouse.1 >= row_y
        && mouse.1 <= row_y + btn_size
    {
        *value = (*value - SENS_STEP).max(SENS_MIN);
    }

    // > button
    let gt_x = slider_x + slider_w + 5.0 * s;
    if clicked
        && mouse.0 >= gt_x
        && mouse.0 <= gt_x + btn_size
        && mouse.1 >= row_y
        && mouse.1 <= row_y + btn_size
    {
        *value = (*value + SENS_STEP).min(SENS_MAX);
    }

    // Slider drag
    if held
        && mouse.1 >= row_y
        && mouse.1 <= row_y + slider_h
        && mouse.0 >= slider_x
        && mouse.0 <= slider_x + slider_w
    {
        let t = (mouse.0 - slider_x) / slider_w;
        *value = SENS_MIN + t * (SENS_MAX - SENS_MIN);
    }
}

pub fn draw(
    value: f32,
    label_x: f32,
    slider_x: f32,
    slider_w: f32,
    btn_size: f32,
    row_y: f32,
    s: f32,
) {
    let slider_h = 20.0 * s;
    let label_fs = (18.0 * s) as u16;
    let btn_fs = (20.0 * s) as u16;
    let val_fs = (16.0 * s) as u16;

    draw_text_ex(
        "Sensitivity",
        label_x,
        row_y + 16.0 * s,
        text_params(label_fs, WHITE),
    );

    // < button
    let lt_x = slider_x - btn_size - 5.0 * s;
    draw_rectangle(lt_x, row_y, btn_size, btn_size, Color::from_rgba(60, 60, 60, 255));
    draw_text_ex(
        "<",
        lt_x + btn_size * 0.2,
        row_y + btn_size * 0.72,
        text_params(btn_fs, WHITE),
    );

    // Slider bar
    draw_rectangle(
        slider_x,
        row_y,
        slider_w,
        slider_h,
        Color::from_rgba(40, 40, 40, 255),
    );
    let t = ((value - SENS_MIN) / (SENS_MAX - SENS_MIN)).clamp(0.0, 1.0);
    draw_rectangle(
        slider_x,
        row_y,
        slider_w * t,
        slider_h,
        Color::from_rgba(80, 150, 220, 255),
    );
    draw_rectangle_lines(slider_x, row_y, slider_w, slider_h, 1.0, GRAY);

    // > button
    let gt_x = slider_x + slider_w + 5.0 * s;
    draw_rectangle(gt_x, row_y, btn_size, btn_size, Color::from_rgba(60, 60, 60, 255));
    draw_text_ex(
        ">",
        gt_x + btn_size * 0.3,
        row_y + btn_size * 0.72,
        text_params(btn_fs, WHITE),
    );

    // Value text
    draw_text_ex(
        &format!("{:.0}", value),
        slider_x + slider_w + btn_size + 15.0 * s,
        row_y + 16.0 * s,
        text_params(val_fs, WHITE),
    );
}
