use macroquad::prelude::Color;

/// Deterministic jitter so each cell gets a stable color tweak without new state.
pub(super) fn cell_noise(x: i32, y: i32, salt: u32) -> f32 {
    let mut v = (x as u32).wrapping_mul(0x8da6b343) ^ (y as u32).wrapping_mul(0xd8163841) ^ salt;
    v ^= v >> 13;
    v = v.wrapping_mul(0x85ebca6b);
    v ^= v >> 16;
    // Map to [-1, 1]
    (v as f32 / u32::MAX as f32) * 2.0 - 1.0
}

pub(super) fn tint_color(
    base: Color,
    x: i32,
    y: i32,
    salt: u32,
    light_scale: f32,
    warm_scale: f32,
) -> Color {
    let lightness = cell_noise(x, y, salt) * light_scale;
    let warmth = cell_noise(x, y, salt ^ 0x9e3779b9) * warm_scale;

    let r = (base.r + lightness + warmth).clamp(0.0, 1.0);
    let g = (base.g + lightness).clamp(0.0, 1.0);
    let b = (base.b + lightness - warmth).clamp(0.0, 1.0);

    Color { r, g, b, a: base.a }
}
