use macroquad::prelude::*;

pub(super) fn draw_scope_vignette(cx: f32, cy: f32, radius: f32, alpha: u8) {
    let color = Color::from_rgba(0, 0, 0, alpha);
    let far = screen_width().max(screen_height());
    let steps = 64;

    for i in 0..steps {
        let a0 = ((i as f32) / (steps as f32)) * std::f32::consts::TAU;
        let a1 = (((i + 1) as f32) / (steps as f32)) * std::f32::consts::TAU;

        let c0 = vec2(cx + a0.cos() * radius, cy + a0.sin() * radius);
        let c1 = vec2(cx + a1.cos() * radius, cy + a1.sin() * radius);

        let e0 = vec2(cx + a0.cos() * far, cy + a0.sin() * far);
        let e1 = vec2(cx + a1.cos() * far, cy + a1.sin() * far);

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
