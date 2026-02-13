use crate::ui::fonts::{measure_game_text, text_params};
use macroquad::prelude::*;

pub mod fonts;
pub mod hud;
pub use crate::types::player::OtherPlayer;

/// Draws a thick arc using triangle strips. Used by both HUD crosshairs and crosshair settings preview.
pub fn draw_arc(
    cx: f32,
    cy: f32,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    thickness: f32,
    segments: u32,
    color: Color,
) {
    let angle_range = end_angle - start_angle;
    let angle_step = angle_range / (segments as f32);

    for i in 0..segments {
        let a1 = start_angle + (i as f32) * angle_step;
        let a2 = start_angle + ((i + 1) as f32) * angle_step;

        let x1_inner = cx + (radius - thickness / 2.0) * a1.cos();
        let y1_inner = cy - (radius - thickness / 2.0) * a1.sin();
        let x2_inner = cx + (radius - thickness / 2.0) * a2.cos();
        let y2_inner = cy - (radius - thickness / 2.0) * a2.sin();

        let x1_outer = cx + (radius + thickness / 2.0) * a1.cos();
        let y1_outer = cy - (radius + thickness / 2.0) * a1.sin();
        let x2_outer = cx + (radius + thickness / 2.0) * a2.cos();
        let y2_outer = cy - (radius + thickness / 2.0) * a2.sin();

        draw_triangle(
            vec2(x1_inner, y1_inner),
            vec2(x2_inner, y2_inner),
            vec2(x1_outer, y1_outer),
            color
        );
        draw_triangle(
            vec2(x2_inner, y2_inner),
            vec2(x2_outer, y2_outer),
            vec2(x1_outer, y1_outer),
            color
        );
    }
}

pub fn is_button_clicked(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    label: &str,
    color: Color,
    hover_color: Color,
) -> bool {
    let mouse = mouse_position();
    let hover = mouse.0 >= x && mouse.0 <= x + w && mouse.1 >= y && mouse.1 <= y + h;
    draw_rectangle(x, y, w, h, if hover { hover_color } else { color });
    draw_rectangle_lines(x, y, w, h, 1.0, WHITE);
    let tw = measure_game_text(label, 20);
    draw_text_ex(label, x + (w - tw) / 2.0, y + 26.0, text_params(20, WHITE));
    hover && is_mouse_button_pressed(MouseButton::Left)
}
