use macroquad::prelude::*;
use crate::ui::prims::{draw_circle, draw_circle_lines};

pub(super) fn draw_scope_vignette(cx: f32, cy: f32, radius: f32, alpha: u8) {
    let color = Color::from_rgba(0, 0, 0, alpha);
    let far = screen_width().max(screen_height());
    let steps = 64;

    // each step's end angle is the next one's start, so its trig is reused
    let (mut sin0, mut cos0) = 0.0_f32.sin_cos();
    for i in 0..steps {
        let a1 = (((i + 1) as f32) / (steps as f32)) * std::f32::consts::TAU;
        let (sin1, cos1) = a1.sin_cos();

        let c0 = vec2(cx + cos0 * radius, cy + sin0 * radius);
        let c1 = vec2(cx + cos1 * radius, cy + sin1 * radius);

        let e0 = vec2(cx + cos0 * far, cy + sin0 * far);
        let e1 = vec2(cx + cos1 * far, cy + sin1 * far);
        (sin0, cos0) = (sin1, cos1);

        draw_triangle(c0, c1, e0, color);
        draw_triangle(c1, e0, e1, color);
    }

    draw_circle_lines(cx, cy, radius, 3.0, Color::from_rgba(40, 40, 40, alpha));
    draw_circle_lines(
        cx,
        cy,
        radius - 1.5,
        1.0,
        Color::from_rgba(80, 80, 80, alpha),
    );
    draw_circle(
        cx,
        cy,
        radius - 2.0,
        Color::from_rgba(0, 10, 20, ((alpha as f32) * 0.12) as u8),
    );
}
