use crate::ui::fonts::{measure_game_text, text_params};
use macroquad::prelude::*;

pub(in crate::settings::crosshair_settings) fn draw_shotgun_type_warning(
    x: f32,
    y: f32,
    _btn_w: f32,
    _btn_h: f32,
    mouse: (f32, f32),
    clicked: bool,
    s: f32,
) -> bool {
    let box_w = 340.0 * s;
    let box_h = 80.0 * s;
    let box_x = x;
    let box_y = y - box_h - 10.0 * s;
    let fs = (14.0 * s) as u16;
    let ok_fs = (14.0 * s) as u16;

    draw_rectangle(box_x, box_y, box_w, box_h, Color::from_rgba(60, 30, 10, 240));
    draw_rectangle_lines(box_x, box_y, box_w, box_h, 2.0, Color::from_rgba(255, 160, 0, 255));
    draw_text_ex(
        "Switching to a non-standard crosshair",
        box_x + 10.0 * s,
        box_y + 22.0 * s,
        text_params(fs, Color::from_rgba(255, 200, 80, 255)),
    );
    draw_text_ex(
        "for shotgun may affect gameplay.",
        box_x + 10.0 * s,
        box_y + 42.0 * s,
        text_params(fs, Color::from_rgba(255, 200, 80, 255)),
    );

    let ok_w = 50.0 * s;
    let ok_h = 25.0 * s;
    let ok_x = box_x + box_w - ok_w - 10.0 * s;
    let ok_y = box_y + box_h - ok_h - 8.0 * s;

    draw_rectangle(ok_x, ok_y, ok_w, ok_h, Color::from_rgba(80, 150, 220, 255));
    draw_rectangle_lines(ok_x, ok_y, ok_w, ok_h, 1.0, GRAY);
    let ok_tw = measure_game_text("OK", ok_fs);
    draw_text_ex(
        "OK",
        ok_x + (ok_w - ok_tw) / 2.0,
        ok_y + ok_h * 0.68,
        text_params(ok_fs, WHITE),
    );

    clicked
        && mouse.0 >= ok_x
        && mouse.0 <= ok_x + ok_w
        && mouse.1 >= ok_y
        && mouse.1 <= ok_y + ok_h
}
