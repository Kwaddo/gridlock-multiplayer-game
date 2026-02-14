use macroquad::prelude::*;

/// Lighting configuration: position, color tint, and intensity
pub struct CeilingLight {
    pub pos: Vec3,
    pub color: Vec3,
    pub intensity: f32,
}

/// Apply lighting from multiple ceiling lights to a surface point
pub fn apply_lighting(lights: &[CeilingLight], c: Color, p: Vec3) -> Color {
    let ambient = 0.35; // Higher ambient so areas aren't too dark
    let falloff = 0.18; // Faster falloff for more localized lighting

    let mut lit = vec3(c.r, c.g, c.b) * ambient;

    for light in lights {
        let d = p - light.pos;
        let dist = (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();

        // Quadratic falloff with minimum distance to avoid extreme brightness
        let effective_dist = dist.max(1.0);
        let atten = 1.0
            / (1.0 + effective_dist * falloff + effective_dist * effective_dist * falloff * 0.3);
        let strength = atten * light.intensity;

        // Blend both light intensity and color tint - reduced contribution
        let light_contrib = vec3(c.r, c.g, c.b) * strength * 0.5 + light.color * strength * 0.08;
        lit = lit + light_contrib;
    }

    Color {
        r: lit.x.min(1.0),
        g: lit.y.min(1.0),
        b: lit.z.min(1.0),
        a: c.a,
    }
}

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
