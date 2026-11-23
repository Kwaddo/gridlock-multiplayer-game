//! Impact puffs: a few sparks and a little dust where a shot hits a wall or
//! prop, and a small red puff where it hits a body. A fixed pool (no
//! allocation once it is full) drawn as camera-facing quads in one batch.

use std::cell::RefCell;

use macroquad::models::Vertex;
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use macroquad::window::get_internal_gl;

/// Most particles alive at once; a new one past this replaces an old one.
/// 4 vertices each, so a full pool is 512, far under macroquad's
/// 16,384-vertex draw call.
pub(super) const MAX_PARTICLES: usize = 128;
const _: () = assert!(MAX_PARTICLES * 4 <= 16_384);

/// What a shot hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImpactKind {
    /// A wall, prop, floor or the lobby's geometry: sparks and dust.
    Surface,
    /// A player, bot or the target dummy: a small red puff.
    Body,
}

#[derive(Clone, Copy)]
struct Particle {
    pos: Vec3,
    vel: Vec3,
    life: f32,
    max_life: f32,
    size: f32,
    color: [f32; 4],
    /// Downward pull (negative drifts up, like dust).
    gravity: f32,
    /// Drawn as a streak along its velocity (sparks) instead of a square.
    streak: bool,
    /// How much it grows by the end of its life (1 = not at all).
    grow: f32,
}

#[derive(Default)]
pub(super) struct Particles {
    list: Vec<Particle>,
    /// Which slot the next particle takes once the pool is full.
    next: usize,
}

thread_local! {
    static SCRATCH: RefCell<(Vec<Vertex>, Vec<u16>)> = const { RefCell::new((Vec::new(), Vec::new())) };
}

fn scatter() -> Vec3 {
    vec3(gen_range(-1.0, 1.0), gen_range(-1.0, 1.0), gen_range(-1.0, 1.0))
}

impl Particles {
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.list.len()
    }

    #[cfg(test)]
    pub(super) fn capacity(&self) -> usize {
        self.list.capacity()
    }

    fn push(&mut self, p: Particle) {
        if self.list.capacity() < MAX_PARTICLES {
            self.list.reserve_exact(MAX_PARTICLES - self.list.len());
        }
        if self.list.len() < MAX_PARTICLES {
            self.list.push(p);
        } else {
            self.next %= MAX_PARTICLES;
            self.list[self.next] = p;
            self.next += 1;
        }
    }

    /// A puff at `at`; `normal` points back out of what was hit (towards the shooter for a body).
    pub(super) fn spawn(&mut self, at: Vec3, normal: Vec3, kind: ImpactKind) {
        let n = normal.normalize_or_zero();
        match kind {
            ImpactKind::Surface => {
                for _ in 0..5 {
                    let life = gen_range(0.1, 0.22);
                    self.push(Particle {
                        pos: at,
                        vel: n * gen_range(2.0, 5.0) + scatter() * gen_range(1.5, 4.0),
                        life,
                        max_life: life,
                        size: gen_range(0.008, 0.014),
                        color: [1.0, gen_range(0.75, 0.9), 0.4, 1.0],
                        gravity: 9.0,
                        streak: true,
                        grow: 1.0,
                    });
                }
                for _ in 0..4 {
                    let life = gen_range(0.3, 0.5);
                    self.push(Particle {
                        pos: at + n * 0.02,
                        vel: n * gen_range(0.4, 1.2) + scatter() * 0.4,
                        life,
                        max_life: life,
                        size: gen_range(0.03, 0.05),
                        color: [0.62, 0.59, 0.54, 0.5],
                        gravity: -0.3,
                        streak: false,
                        grow: 2.2,
                    });
                }
            }
            ImpactKind::Body => {
                for _ in 0..6 {
                    let life = gen_range(0.16, 0.28);
                    self.push(Particle {
                        pos: at,
                        vel: n * gen_range(0.6, 1.8) + scatter() * 0.8,
                        life,
                        max_life: life,
                        size: gen_range(0.025, 0.045),
                        color: [0.72, 0.06, 0.06, 0.85],
                        gravity: 3.0,
                        streak: false,
                        grow: 1.6,
                    });
                }
            }
        }
    }

    pub(super) fn update(&mut self, dt: f32) {
        if self.list.is_empty() {
            return;
        }
        for p in &mut self.list {
            p.life -= dt;
            p.vel.y -= p.gravity * dt;
            p.vel *= 1.0 - (4.0 * dt).min(1.0);
            p.pos += p.vel * dt;
        }
        self.list.retain(|p| p.life > 0.0);
        if self.next > self.list.len() {
            self.next = 0;
        }
    }

    /// Every particle as quads facing `eye`, in one batch.
    pub(super) fn draw(&self, eye: Vec3) {
        if self.list.is_empty() {
            return;
        }
        SCRATCH.with_borrow_mut(|(verts, indices)| {
            verts.clear();
            indices.clear();
            let uv = vec2(0.0, 0.0);
            for p in &self.list {
                let to_cam = (eye - p.pos).normalize_or_zero();
                if to_cam == Vec3::ZERO {
                    continue;
                }
                let fade = (p.life / p.max_life).clamp(0.0, 1.0);
                let [r, g, b, a] = p.color;
                let color = Color::new(r, g, b, a * fade);
                let size = p.size * (1.0 + (p.grow - 1.0) * (1.0 - fade));
                let base = verts.len() as u16;
                if p.streak {
                    // head at the spark, tail back along its path, fading out
                    let len = (p.vel.length() * 0.025).clamp(0.02, 0.15);
                    let tail = p.pos - p.vel.normalize_or_zero() * len;
                    let side = (p.pos - tail).cross(to_cam).normalize_or_zero() * size;
                    let clear = Color::new(r, g, b, 0.0);
                    verts.push(Vertex::new2(p.pos - side, uv, color));
                    verts.push(Vertex::new2(p.pos + side, uv, color));
                    verts.push(Vertex::new2(tail + side, uv, clear));
                    verts.push(Vertex::new2(tail - side, uv, clear));
                } else {
                    let right = to_cam.cross(Vec3::Y).normalize_or(Vec3::X) * size;
                    let up = right.cross(to_cam).normalize_or_zero() * size;
                    verts.push(Vertex::new2(p.pos - right - up, uv, color));
                    verts.push(Vertex::new2(p.pos + right - up, uv, color));
                    verts.push(Vertex::new2(p.pos + right + up, uv, color));
                    verts.push(Vertex::new2(p.pos - right + up, uv, color));
                }
                // both windings, so a quad shows whichever way it ends up facing
                for k in [0, 1, 2, 0, 2, 3, 2, 1, 0, 3, 2, 0] {
                    indices.push(base + k);
                }
            }
            if verts.is_empty() {
                return;
            }
            let gl = unsafe { get_internal_gl() };
            gl.quad_gl.texture(None);
            gl.quad_gl.draw_mode(DrawMode::Triangles);
            gl.quad_gl.geometry(verts, indices);
        });
    }
}
