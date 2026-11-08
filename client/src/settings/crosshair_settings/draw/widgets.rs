use crate::ui::fonts::{measure_game_text, text_params};
use macroquad::prelude::*;

pub(super) fn draw_section_header(label: &str, x: f32, y: f32, s: f32) -> f32 {
    let fs = (18.0 * s) as u16;
    draw_text_ex(label, x, y + 18.0 * s, text_params(fs, Color::from_rgba(100, 180, 255, 255)));
    draw_line(x, y + 25.0 * s, x + 300.0 * s, y + 25.0 * s, 1.0, GRAY);
    35.0 * s
}

pub(in crate::settings::crosshair_settings) fn draw_slider(
    label: &str,
    value: &mut f32,
    min: f32,
    max: f32,
    step: f32,
    x: f32,
    y: f32,
    width: f32,
    mouse: (f32, f32),
    _clicked: bool,
    held: bool,
    fill_color: Option<Color>,
    s: f32,
) -> f32 {
    let label_w = 100.0 * s;
    let slider_x = x + label_w;
    let slider_w = width - label_w - 50.0 * s;
    let slider_h = 14.0 * s;
    let row_h = 28.0 * s;
    let fs = (14.0 * s) as u16;
    let val_fs = (12.0 * s) as u16;

    draw_text_ex(label, x, y + 14.0 * s, text_params(fs, WHITE));

    draw_rectangle(slider_x, y + 1.0, slider_w, slider_h, Color::from_rgba(35, 35, 35, 255));
    let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
    let fc = fill_color.unwrap_or(Color::from_rgba(80, 150, 220, 255));
    draw_rectangle(slider_x, y + 1.0, slider_w * t, slider_h, fc);
    draw_rectangle_lines(slider_x, y + 1.0, slider_w, slider_h, 1.0, Color::from_rgba(60, 60, 60, 255));

    if held
        && mouse.1 >= y
        && mouse.1 <= y + row_h
        && mouse.0 >= slider_x
        && mouse.0 <= slider_x + slider_w
    {
        let raw = min + ((mouse.0 - slider_x) / slider_w) * (max - min);
        *value = (raw / step).round() * step;
        *value = value.clamp(min, max);
    }

    let precision = if step >= 1.0 { 0 } else if step >= 0.1 { 1 } else { 2 };
    let txt = format!("{:.*}", precision, *value);
    draw_text_ex(&txt, slider_x + slider_w + 6.0 * s, y + 12.0 * s, text_params(val_fs, GRAY));

    row_h
}

pub(super) fn draw_slider_display(
    label: &str,
    value: f32,
    min: f32,
    max: f32,
    x: f32,
    y: f32,
    width: f32,
    fill_color: Option<Color>,
    s: f32,
) {
    let label_w = 100.0 * s;
    let slider_x = x + label_w;
    let slider_w = width - label_w - 50.0 * s;
    let slider_h = 14.0 * s;
    let fs = (14.0 * s) as u16;
    let val_fs = (12.0 * s) as u16;

    draw_text_ex(label, x, y + 14.0 * s, text_params(fs, WHITE));

    draw_rectangle(slider_x, y + 1.0, slider_w, slider_h, Color::from_rgba(35, 35, 35, 255));
    let t = ((value - min) / (max - min)).clamp(0.0, 1.0);
    let fc = fill_color.unwrap_or(Color::from_rgba(80, 150, 220, 255));
    draw_rectangle(slider_x, y + 1.0, slider_w * t, slider_h, fc);
    draw_rectangle_lines(slider_x, y + 1.0, slider_w, slider_h, 1.0, Color::from_rgba(60, 60, 60, 255));

    let step = if max > 100.0 { 1.0 } else if max > 5.0 { 0.1 } else { 0.05 };
    let precision = if step >= 1.0 { 0 } else if step >= 0.1 { 1 } else { 2 };
    let txt = format!("{:.*}", precision, value);
    draw_text_ex(&txt, slider_x + slider_w + 6.0 * s, y + 12.0 * s, text_params(val_fs, GRAY));
}

pub(in crate::settings::crosshair_settings) fn draw_toggle(
    label: &str,
    value: &mut bool,
    x: f32,
    y: f32,
    mouse: (f32, f32),
    clicked: bool,
    s: f32,
) -> f32 {
    let box_size = 18.0 * s;
    let row_h = 28.0 * s;
    let fs = (14.0 * s) as u16;
    let checkbox_color = if *value {
        Color::from_rgba(80, 150, 220, 255)
    } else {
        Color::from_rgba(35, 35, 35, 255)
    };
    draw_rectangle(x, y, box_size, box_size, checkbox_color);
    draw_rectangle_lines(x, y, box_size, box_size, 1.0, Color::from_rgba(60, 60, 60, 255));
    if clicked && mouse.0 >= x && mouse.0 <= x + box_size && mouse.1 >= y && mouse.1 <= y + box_size {
        *value = !*value;
    }
    draw_text_ex(label, x + box_size + 8.0 * s, y + 14.0 * s, text_params(fs, WHITE));
    row_h
}

pub(super) fn draw_toggle_display(label: &str, value: bool, x: f32, y: f32, s: f32) {
    let box_size = 18.0 * s;
    let fs = (14.0 * s) as u16;
    let checkbox_color = if value {
        Color::from_rgba(80, 150, 220, 255)
    } else {
        Color::from_rgba(35, 35, 35, 255)
    };
    draw_rectangle(x, y, box_size, box_size, checkbox_color);
    draw_rectangle_lines(x, y, box_size, box_size, 1.0, Color::from_rgba(60, 60, 60, 255));
    draw_text_ex(label, x + box_size + 8.0 * s, y + 14.0 * s, text_params(fs, WHITE));
}

pub(in crate::settings::crosshair_settings) fn draw_dropdown(
    label: &str,
    options: &[&str],
    selected: &mut usize,
    x: f32,
    y: f32,
    width: f32,
    mouse: (f32, f32),
    clicked: bool,
    s: f32,
) -> f32 {
    let label_w = 100.0 * s;
    let dropdown_x = x + label_w;
    let dropdown_w = width - label_w - 10.0 * s;
    let dropdown_h = 28.0 * s;
    let fs = (14.0 * s) as u16;

    draw_text_ex(label, x, y + 18.0 * s, text_params(fs, WHITE));
    draw_rectangle(dropdown_x, y, dropdown_w, dropdown_h, Color::from_rgba(35, 35, 35, 255));
    draw_rectangle_lines(dropdown_x, y, dropdown_w, dropdown_h, 1.0, Color::from_rgba(60, 60, 60, 255));

    let current_text = options[*selected];
    let text_w = measure_game_text(current_text, fs);
    draw_text_ex(current_text, dropdown_x + (dropdown_w - text_w) / 2.0, y + 18.0 * s, text_params(fs, WHITE));

    if clicked
        && mouse.0 >= dropdown_x
        && mouse.0 <= dropdown_x + dropdown_w
        && mouse.1 >= y
        && mouse.1 <= y + dropdown_h
    {
        *selected = (*selected + 1) % options.len();
    }
    32.0 * s
}

pub(super) fn draw_dropdown_display(label: &str, value: &str, x: f32, y: f32, width: f32, s: f32) {
    let label_w = 100.0 * s;
    let dropdown_x = x + label_w;
    let dropdown_w = width - label_w - 10.0 * s;
    let dropdown_h = 28.0 * s;
    let fs = (14.0 * s) as u16;

    draw_text_ex(label, x, y + 18.0 * s, text_params(fs, WHITE));
    draw_rectangle(dropdown_x, y, dropdown_w, dropdown_h, Color::from_rgba(35, 35, 35, 255));
    draw_rectangle_lines(dropdown_x, y, dropdown_w, dropdown_h, 1.0, Color::from_rgba(60, 60, 60, 255));

    let text_w = measure_game_text(value, fs);
    draw_text_ex(value, dropdown_x + (dropdown_w - text_w) / 2.0, y + 18.0 * s, text_params(fs, WHITE));
}
