use macroquad::models::Vertex;
use macroquad::prelude::*;
use macroquad::window::get_internal_gl;

use super::super::PlayingState;

/// Number of segments for the circular disc geometry.
const DISC_SEGMENTS: usize = 12;

impl PlayingState {
    pub(crate) fn draw_bullet_holes(&self) {
        for hole in &self.bullet_holes {
            let fade = (hole.lifetime / hole.max_lifetime).clamp(0.0, 1.0);
            if fade <= 0.0 {
                continue;
            }

            let [cr, cg, cb, _] = hole.color;
            let alpha = (fade * 220.0) as u8;
            let size = hole.size;

            let (tangent, bitangent) = tangent_from_normal(hole.normal);
            let p = hole.position;
            let uv = vec2(0.0, 0.0);

            // Use a deterministic seed based on position for per-hole randomness
            let seed = (hole.position.x * 1000.0 + hole.position.z * 7777.0).to_bits();

            // --- Outer ring: darker, slightly larger, circular disc with irregular edges ---
            let outer_base = size * 1.4;
            let outer_alpha = (fade * 100.0) as u8;
            let outer_color = Color::from_rgba(cr / 2, cg / 2, cb / 2, outer_alpha);
            draw_irregular_disc(
                p, tangent, bitangent, outer_base, outer_color, uv, seed.wrapping_add(1),
            );

            // --- Inner hole: darker center, circular disc ---
            let center_color = Color::from_rgba(cr, cg, cb, alpha);
            draw_irregular_disc(p, tangent, bitangent, size, center_color, uv, seed);

            // --- Dark core: very small dark circle in the center for depth ---
            let core_alpha = (fade * 255.0) as u8;
            let core_color = Color::from_rgba(cr / 3, cg / 3, cb / 3, core_alpha);
            draw_disc(p, tangent, bitangent, size * 0.4, core_color, uv);

            // --- Scorch marks: irregular radial lines around the hole ---
            let scorch_alpha = (fade * 80.0) as u8;
            let scorch_color = Color::from_rgba(60, 50, 40, scorch_alpha);
            let num_rays = 8;
            for i in 0..num_rays {
                // use deterministic pseudo-random offsets per ray
                let ray_seed = seed.wrapping_mul(31).wrapping_add(i as u32 * 97);
                let angle_offset = pseudo_rand(ray_seed) * 0.4 - 0.2;
                let angle =
                    (i as f32 / num_rays as f32) * std::f32::consts::TAU + 0.3 + angle_offset;
                let dir_2d = tangent * angle.cos() + bitangent * angle.sin();
                let inner_r = size * 0.7;
                // randomize outer reach per ray for organic look
                let len_variation = 0.6 + pseudo_rand(ray_seed.wrapping_add(53)) * 1.0;
                let outer_r = size * (1.5 + len_variation);
                let half_w = size * 0.12;
                let perp = hole.normal.cross(dir_2d).normalize_or_zero() * half_w;

                let a = p + dir_2d * inner_r - perp;
                let b_pt = p + dir_2d * inner_r + perp;
                let c = p + dir_2d * outer_r;

                let ray_verts = [
                    Vertex::new2(a, uv, scorch_color),
                    Vertex::new2(b_pt, uv, scorch_color),
                    Vertex::new2(c, uv, Color::from_rgba(60, 50, 40, 0)),
                ];
                let tri_indices: [u16; 6] = [0, 1, 2, 2, 1, 0];

                let gl = unsafe { get_internal_gl() };
                gl.quad_gl.texture(None);
                gl.quad_gl.draw_mode(DrawMode::Triangles);
                gl.quad_gl.geometry(&ray_verts, &tri_indices);
            }
        }
    }
}

/// Simple deterministic pseudo-random in [0, 1) from a u32 seed.
fn pseudo_rand(seed: u32) -> f32 {
    let s = seed.wrapping_mul(2654435761).wrapping_add(1013904223);
    (s & 0x00FFFFFF) as f32 / 0x01000000 as f32
}

/// Draw a regular circular disc (fan of triangles).
fn draw_disc(center: Vec3, tangent: Vec3, bitangent: Vec3, radius: f32, color: Color, uv: Vec2) {
    let mut verts = Vec::with_capacity(DISC_SEGMENTS + 1);
    let mut indices = Vec::with_capacity(DISC_SEGMENTS * 3);

    // center vertex
    verts.push(Vertex::new2(center, uv, color));

    for i in 0..DISC_SEGMENTS {
        let angle = (i as f32 / DISC_SEGMENTS as f32) * std::f32::consts::TAU;
        let offset = tangent * (angle.cos() * radius) + bitangent * (angle.sin() * radius);
        verts.push(Vertex::new2(center + offset, uv, color));

        let next = if i + 1 < DISC_SEGMENTS { i + 2 } else { 1 };
        indices.push(0u16);
        indices.push((i + 1) as u16);
        indices.push(next as u16);
    }

    let gl = unsafe { get_internal_gl() };
    gl.quad_gl.texture(None);
    gl.quad_gl.draw_mode(DrawMode::Triangles);
    gl.quad_gl.geometry(&verts, &indices);
}

/// Draw a circular disc with slight per-vertex radius variation for an irregular, organic look.
fn draw_irregular_disc(
    center: Vec3,
    tangent: Vec3,
    bitangent: Vec3,
    radius: f32,
    color: Color,
    uv: Vec2,
    seed: u32,
) {
    let mut verts = Vec::with_capacity(DISC_SEGMENTS + 1);
    let mut indices = Vec::with_capacity(DISC_SEGMENTS * 3);

    // center vertex
    verts.push(Vertex::new2(center, uv, color));

    for i in 0..DISC_SEGMENTS {
        let angle = (i as f32 / DISC_SEGMENTS as f32) * std::f32::consts::TAU;
        // vary radius by +/- 20% per vertex for irregular shape
        let variation = 0.80 + pseudo_rand(seed.wrapping_add(i as u32 * 173)) * 0.40;
        let r = radius * variation;
        let offset = tangent * (angle.cos() * r) + bitangent * (angle.sin() * r);
        verts.push(Vertex::new2(center + offset, uv, color));

        let next = if i + 1 < DISC_SEGMENTS { i + 2 } else { 1 };
        indices.push(0u16);
        indices.push((i + 1) as u16);
        indices.push(next as u16);
    }

    let gl = unsafe { get_internal_gl() };
    gl.quad_gl.texture(None);
    gl.quad_gl.draw_mode(DrawMode::Triangles);
    gl.quad_gl.geometry(&verts, &indices);
}

/// Compute tangent and bitangent vectors from a surface normal.
fn tangent_from_normal(normal: Vec3) -> (Vec3, Vec3) {
    let up = if normal.y.abs() > 0.9 {
        vec3(1.0, 0.0, 0.0)
    } else {
        vec3(0.0, 1.0, 0.0)
    };
    let tangent = normal.cross(up).normalize();
    let bitangent = normal.cross(tangent).normalize();
    (tangent, bitangent)
}
