use crate::ui::fonts::{measure_game_text, text_params};
use macroquad::prelude::*;

pub(super) fn draw_slider(
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
) -> f32 {
    let label_w = 200.0;
    let slider_x = x + label_w;
    let slider_w = width - label_w - 80.0;
    let slider_h = 16.0;
    let row_h = 28.0;

    draw_text_ex(label, x, y + 20.0, text_params(18, WHITE));

    draw_rectangle(
        slider_x,
        y + 2.0,
        slider_w,
        slider_h,
        Color::from_rgba(40, 40, 40, 255),
    );
    let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
    draw_rectangle(
        slider_x,
        y + 2.0,
        slider_w * t,
        slider_h,
        Color::from_rgba(80, 150, 220, 255),
    );
    draw_rectangle_lines(slider_x, y + 2.0, slider_w, slider_h, 1.0, GRAY);

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

    let precision = if step >= 1.0 {
        0
    } else if step >= 0.1 {
        1
    } else if step >= 0.01 {
        2
    } else {
        3
    };

    let txt = format!("{:.*}", precision, *value);
    draw_text_ex(
        &txt,
        slider_x + slider_w + 8.0,
        y + 14.0,
        text_params(14, WHITE),
    );

    row_h
}

// Like draw_slider but with [-] and [+] nudge buttons on the right.
// Use this for viewmodel sliders where dragging the full range is tedious.
pub(super) fn draw_slider_nudge(
    label: &str,
    value: &mut f32,
    min: f32,
    max: f32,
    step: f32,
    x: f32,
    y: f32,
    width: f32,
    mouse: (f32, f32),
    clicked: bool,
    held: bool,
) -> f32 {
    let label_w = 160.0;
    let btn_w = 20.0;
    let val_w = 45.0;
    let gap = 4.0;
    let slider_w = width - label_w - val_w - btn_w * 2.0 - gap * 3.0;
    let slider_h = 16.0;
    let row_h = 28.0;

    draw_text_ex(label, x, y + 20.0, text_params(14, WHITE));

    let slider_x = x + label_w;
    draw_rectangle(slider_x, y + 2.0, slider_w, slider_h, Color::from_rgba(40, 40, 40, 255));
    let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
    draw_rectangle(slider_x, y + 2.0, slider_w * t, slider_h, Color::from_rgba(80, 150, 220, 255));
    draw_rectangle_lines(slider_x, y + 2.0, slider_w, slider_h, 1.0, GRAY);

    if held && mouse.1 >= y && mouse.1 <= y + row_h && mouse.0 >= slider_x && mouse.0 <= slider_x + slider_w {
        let raw = min + ((mouse.0 - slider_x) / slider_w) * (max - min);
        *value = (raw / step).round() * step;
        *value = value.clamp(min, max);
    }

    let precision = if step >= 1.0 { 0 } else if step >= 0.1 { 1 } else if step >= 0.01 { 2 } else { 3 };
    let txt = format!("{:.*}", precision, *value);
    let val_x = slider_x + slider_w + gap;
    draw_text_ex(&txt, val_x, y + 14.0, text_params(13, WHITE));

    let minus_x = val_x + val_w;
    let plus_x = minus_x + btn_w + gap;
    let btn_y = y + 4.0;

    let mhov = mouse.0 >= minus_x && mouse.0 <= minus_x + btn_w && mouse.1 >= btn_y && mouse.1 <= btn_y + btn_w;
    draw_rectangle(minus_x, btn_y, btn_w, btn_w, if mhov { Color::from_rgba(80, 80, 80, 255) } else { Color::from_rgba(40, 40, 40, 255) });
    draw_rectangle_lines(minus_x, btn_y, btn_w, btn_w, 1.0, GRAY);
    draw_text_ex("-", minus_x + 6.0, btn_y + 14.0, text_params(14, WHITE));
    if clicked && mhov { *value = (*value - step).clamp(min, max); }

    let phov = mouse.0 >= plus_x && mouse.0 <= plus_x + btn_w && mouse.1 >= btn_y && mouse.1 <= btn_y + btn_w;
    draw_rectangle(plus_x, btn_y, btn_w, btn_w, if phov { Color::from_rgba(80, 80, 80, 255) } else { Color::from_rgba(40, 40, 40, 255) });
    draw_rectangle_lines(plus_x, btn_y, btn_w, btn_w, 1.0, GRAY);
    draw_text_ex("+", plus_x + 5.0, btn_y + 14.0, text_params(14, WHITE));
    if clicked && phov { *value = (*value + step).clamp(min, max); }

    row_h
}

pub(super) fn draw_slider_u32(
    label: &str,
    value: &mut u32,
    min: u32,
    max: u32,
    step: u32,
    x: f32,
    y: f32,
    w: f32,
    mouse: (f32, f32),
    clicked: bool,
    held: bool,
) -> f32 {
    let mut fval = *value as f32;
    let h = draw_slider(
        label,
        &mut fval,
        min as f32,
        max as f32,
        step as f32,
        x,
        y,
        w,
        mouse,
        clicked,
        held,
    );
    *value = fval as u32;
    h
}

pub(super) fn draw_toggle(
    label: &str,
    value: &mut bool,
    x: f32,
    y: f32,
    mouse: (f32, f32),
    clicked: bool,
) -> f32 {
    let box_size = 16.0;
    let row_h = 28.0;

    let bx = x;
    let by = y + 2.0;
    draw_rectangle(
        bx,
        by,
        box_size,
        box_size,
        Color::from_rgba(40, 40, 40, 255),
    );
    draw_rectangle_lines(bx, by, box_size, box_size, 1.0, GRAY);
    if *value {
        draw_rectangle(
            bx + 3.0,
            by + 3.0,
            box_size - 6.0,
            box_size - 6.0,
            Color::from_rgba(80, 220, 120, 255),
        );
    }

    if clicked
        && mouse.0 >= bx
        && mouse.0 <= bx + box_size + 200.0
        && mouse.1 >= y
        && mouse.1 <= y + row_h
    {
        *value = !*value;
    }

    draw_text_ex(label, x + box_size + 8.0, y + 14.0, text_params(14, WHITE));

    row_h
}

pub(super) fn draw_section_header(label: &str, x: f32, y: f32) -> f32 {
    draw_text_ex(
        label,
        x,
        y + 16.0,
        text_params(16, Color::from_rgba(80, 150, 220, 255)),
    );
    draw_line(
        x,
        y + 22.0,
        x + 300.0,
        y + 22.0,
        1.0,
        Color::from_rgba(80, 150, 220, 100),
    );
    28.0
}

pub(super) fn draw_action_button(
    label: &str,
    x: f32,
    y: f32,
    w: f32,
    mouse: (f32, f32),
    clicked: bool,
) -> (f32, bool) {
    let h = 28.0;
    let hover = mouse.0 >= x && mouse.0 <= x + w && mouse.1 >= y && mouse.1 <= y + h;
    let color = if hover {
        Color::from_rgba(60, 60, 60, 255)
    } else {
        Color::from_rgba(40, 40, 40, 255)
    };
    draw_rectangle(x, y, w, h, color);
    draw_rectangle_lines(x, y, w, h, 1.0, GRAY);
    let tw = measure_game_text(label, 14);
    draw_text_ex(label, x + (w - tw) / 2.0, y + 18.0, text_params(14, WHITE));
    (32.0, hover && clicked)
}

pub fn draw_section_header_public(label: &str, x: f32, y: f32) -> f32 {
    draw_section_header(label, x, y)
}

pub fn draw_toggle_public(
    label: &str,
    value: &mut bool,
    x: f32,
    y: f32,
    mouse: (f32, f32),
    clicked: bool,
) -> f32 {
    draw_toggle(label, value, x, y, mouse, clicked)
}

pub fn draw_action_button_public(
    label: &str,
    x: f32,
    y: f32,
    w: f32,
    mouse: (f32, f32),
    clicked: bool,
) -> (f32, bool) {
    draw_action_button(label, x, y, w, mouse, clicked)
}
