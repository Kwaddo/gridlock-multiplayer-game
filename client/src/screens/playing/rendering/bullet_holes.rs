use macroquad::models::Vertex;
use macroquad::prelude::*;
use macroquad::window::get_internal_gl;

use super::super::PlayingState;
use crate::weapons::effects::BulletHole;

/// Number of segments for the circular disc geometry.
const DISC_SEGMENTS: usize = 12;
/// Number of scorch rays around each hole.
const NUM_RAYS: usize = 8;
/// Vertices in one disc: the center plus one per segment.
const DISC_VERTS: usize = DISC_SEGMENTS + 1;
/// Vertices of one hole: outer ring, inner hole, dark core, then 3 per scorch ray.
const HOLE_VERTS: usize = DISC_VERTS * 3 + NUM_RAYS * 3;
/// Indices of one hole: a triangle fan per disc, then 2 triangles per ray.
const HOLE_INDICES: usize = DISC_SEGMENTS * 3 * 3 + NUM_RAYS * 6;

/// The index list of one hole. It never changes, so it is built at compile time.
const INDICES: [u16; HOLE_INDICES] = hole_indices();

const fn hole_indices() -> [u16; HOLE_INDICES] {
    let mut out = [0u16; HOLE_INDICES];
    let mut n = 0;
    let mut disc = 0;
    while disc < 3 {
        let base = disc * DISC_VERTS;
        let mut i = 0;
        while i < DISC_SEGMENTS {
            let next = if i + 1 < DISC_SEGMENTS { i + 2 } else { 1 };
            out[n] = base as u16;
            out[n + 1] = (base + i + 1) as u16;
            out[n + 2] = (base + next) as u16;
            n += 3;
            i += 1;
        }
        disc += 1;
    }
    let mut ray = 0;
    while ray < NUM_RAYS {
        let base = DISC_VERTS * 3 + ray * 3;
        // both windings, so the ray shows from either side
        let tri = [0, 1, 2, 2, 1, 0];
        let mut k = 0;
        while k < 6 {
            out[n] = (base + tri[k]) as u16;
            n += 1;
            k += 1;
        }
        ray += 1;
    }
    out
}

/// A bullet hole's vertex positions, worked out once. A hole never moves, so
/// only its colors (the fade) change from frame to frame.
pub(crate) struct BakedBulletHole {
    // what the positions were computed from, to match them to their hole
    position: Vec3,
    normal: Vec3,
    size: f32,
    verts: [Vec3; HOLE_VERTS],
}

impl BakedBulletHole {
    fn matches(&self, hole: &BulletHole) -> bool {
        self.position == hole.position && self.normal == hole.normal && self.size == hole.size
    }

    fn bake(hole: &BulletHole) -> Self {
        let size = hole.size;
        let (tangent, bitangent) = tangent_from_normal(hole.normal);
        let p = hole.position;

        // Use a deterministic seed based on position for per-hole randomness
        let seed = (hole.position.x * 1000.0 + hole.position.z * 7777.0).to_bits();

        let mut verts = [Vec3::ZERO; HOLE_VERTS];
        // --- Outer ring: darker, slightly larger, circular disc with irregular edges ---
        let outer_base = size * 1.4;
        irregular_disc(&mut verts[0..DISC_VERTS], p, tangent, bitangent, outer_base, seed.wrapping_add(1));
        // --- Inner hole: darker center, circular disc ---
        irregular_disc(&mut verts[DISC_VERTS..DISC_VERTS * 2], p, tangent, bitangent, size, seed);
        // --- Dark core: very small dark circle in the center for depth ---
        disc(&mut verts[DISC_VERTS * 2..DISC_VERTS * 3], p, tangent, bitangent, size * 0.4);

        // --- Scorch marks: irregular radial lines around the hole ---
        for i in 0..NUM_RAYS {
            // use deterministic pseudo-random offsets per ray
            let ray_seed = seed.wrapping_mul(31).wrapping_add(i as u32 * 97);
            let angle_offset = pseudo_rand(ray_seed) * 0.4 - 0.2;
            let angle = (i as f32 / NUM_RAYS as f32) * std::f32::consts::TAU + 0.3 + angle_offset;
            let dir_2d = tangent * angle.cos() + bitangent * angle.sin();
            let inner_r = size * 0.7;
            // randomize outer reach per ray for organic look
            let len_variation = 0.6 + pseudo_rand(ray_seed.wrapping_add(53)) * 1.0;
            let outer_r = size * (1.5 + len_variation);
            let half_w = size * 0.12;
            let perp = hole.normal.cross(dir_2d).normalize_or_zero() * half_w;

            let base = DISC_VERTS * 3 + i * 3;
            verts[base] = p + dir_2d * inner_r - perp;
            verts[base + 1] = p + dir_2d * inner_r + perp;
            verts[base + 2] = p + dir_2d * outer_r;
        }

        Self { position: hole.position, normal: hole.normal, size, verts }
    }
}

impl PlayingState {
    /// Bring the baked shapes up to date with the current holes. Call once per
    /// frame before drawing, so `draw_bullet_holes` can be called by every
    /// camera (main view and mirrors) without changing anything.
    pub(crate) fn bake_bullet_holes(&mut self) {
        // Holes are only ever added at the back or removed, so the baked list
        // follows them in order: drop baked entries whose hole is gone, and
        // bake the new holes at the end.
        let baked = &mut self.bullet_hole_geometry;
        // a VecDeque, so dropping the oldest hole is cheap
        let mut cursor = 0;
        for hole in &self.bullet_holes {
            while cursor < baked.len() && !baked[cursor].matches(hole) {
                baked.remove(cursor);
            }
            if cursor == baked.len() {
                baked.push_back(BakedBulletHole::bake(hole));
            }
            cursor += 1;
        }
        baked.truncate(cursor);
    }

    pub(crate) fn draw_bullet_holes(&self) {
        let baked = &self.bullet_hole_geometry;
        let uv = vec2(0.0, 0.0);
        let mut verts = [Vertex::new2(Vec3::ZERO, uv, BLANK); HOLE_VERTS];
        for (hole, geo) in self.bullet_holes.iter().zip(baked.iter()) {
            let fade = (hole.lifetime / hole.max_lifetime).clamp(0.0, 1.0);
            if fade <= 0.0 {
                continue;
            }

            let [cr, cg, cb, _] = hole.color;
            let alpha = (fade * 220.0) as u8;

            let outer_alpha = (fade * 100.0) as u8;
            let outer_color = Color::from_rgba(cr / 2, cg / 2, cb / 2, outer_alpha);
            let center_color = Color::from_rgba(cr, cg, cb, alpha);
            let core_alpha = (fade * 255.0) as u8;
            let core_color = Color::from_rgba(cr / 3, cg / 3, cb / 3, core_alpha);
            let scorch_alpha = (fade * 80.0) as u8;
            let scorch_color = Color::from_rgba(60, 50, 40, scorch_alpha);
            let scorch_tip = Color::from_rgba(60, 50, 40, 0);

            for (i, (v, &pos)) in verts.iter_mut().zip(geo.verts.iter()).enumerate() {
                let color = match i {
                    i if i < DISC_VERTS => outer_color,
                    i if i < DISC_VERTS * 2 => center_color,
                    i if i < DISC_VERTS * 3 => core_color,
                    i if (i - DISC_VERTS * 3) % 3 == 2 => scorch_tip,
                    _ => scorch_color,
                };
                *v = Vertex::new2(pos, uv, color);
            }

            let gl = unsafe { get_internal_gl() };
            gl.quad_gl.texture(None);
            gl.quad_gl.draw_mode(DrawMode::Triangles);
            gl.quad_gl.geometry(&verts, &INDICES);
        }
    }
}

/// Simple deterministic pseudo-random in [0, 1) from a u32 seed.
fn pseudo_rand(seed: u32) -> f32 {
    let s = seed.wrapping_mul(2654435761).wrapping_add(1013904223);
    (s & 0x00FFFFFF) as f32 / 0x01000000 as f32
}

/// Fill a regular circular disc (fan of triangles): the center, then the rim.
fn disc(out: &mut [Vec3], center: Vec3, tangent: Vec3, bitangent: Vec3, radius: f32) {
    // center vertex
    out[0] = center;

    for i in 0..DISC_SEGMENTS {
        let angle = (i as f32 / DISC_SEGMENTS as f32) * std::f32::consts::TAU;
        let offset = tangent * (angle.cos() * radius) + bitangent * (angle.sin() * radius);
        out[i + 1] = center + offset;
    }
}

/// Fill a circular disc with slight per-vertex radius variation for an irregular, organic look.
fn irregular_disc(
    out: &mut [Vec3],
    center: Vec3,
    tangent: Vec3,
    bitangent: Vec3,
    radius: f32,
    seed: u32,
) {
    // center vertex
    out[0] = center;

    for i in 0..DISC_SEGMENTS {
        let angle = (i as f32 / DISC_SEGMENTS as f32) * std::f32::consts::TAU;
        // vary radius by +/- 20% per vertex for irregular shape
        let variation = 0.80 + pseudo_rand(seed.wrapping_add(i as u32 * 173)) * 0.40;
        let r = radius * variation;
        let offset = tangent * (angle.cos() * r) + bitangent * (angle.sin() * r);
        out[i + 1] = center + offset;
    }
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
