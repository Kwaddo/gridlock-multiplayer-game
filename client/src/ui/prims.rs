//! Allocation-free 2D discs and rings. macroquad's `draw_circle` and
//! `draw_circle_lines` build fresh vertex and index Vecs on every call; these
//! reuse one thread-local scratch buffer and push straight into the batcher.

use std::cell::RefCell;

use macroquad::models::Vertex;
use macroquad::prelude::*;
use macroquad::window::get_internal_gl;

struct Scratch {
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
}

thread_local! {
    static SCRATCH: RefCell<Scratch> =
        const { RefCell::new(Scratch { vertices: Vec::new(), indices: Vec::new() }) };
}

fn submit(build: impl FnOnce(&mut Vec<Vertex>, &mut Vec<u16>)) {
    SCRATCH.with(|cell| {
        let mut scratch = cell.borrow_mut();
        let Scratch { vertices, indices } = &mut *scratch;
        vertices.clear();
        indices.clear();
        build(vertices, indices);
        let gl = unsafe { get_internal_gl() };
        gl.quad_gl.texture(None);
        gl.quad_gl.draw_mode(DrawMode::Triangles);
        gl.quad_gl.geometry(vertices, indices);
    });
}

/// A filled disc of `sides` segments, the same shape as `draw_circle`
/// (which always uses 20).
pub fn fill_disc(x: f32, y: f32, radius: f32, sides: u16, color: Color) {
    let sides = sides.max(3);
    submit(|vertices, indices| {
        vertices.push(Vertex::new(x, y, 0., 0., 0., color));
        for i in 0..sides {
            let dir = Vec2::from_angle(i as f32 / sides as f32 * std::f32::consts::TAU);
            vertices.push(Vertex::new(x + radius * dir.x, y + radius * dir.y, 0., dir.x, dir.y, color));
            let next = (i + 1) % sides;
            indices.extend_from_slice(&[0, i + 1, next + 1]);
        }
    });
}

/// A ring from `radius` out to `radius + thickness`, the same shape as
/// `draw_circle_lines` (which always uses 30 sides).
pub fn ring(x: f32, y: f32, radius: f32, thickness: f32, sides: u16, color: Color) {
    let sides = sides.max(3);
    submit(|vertices, indices| {
        for i in 0..sides {
            let dir = Vec2::from_angle(i as f32 / sides as f32 * std::f32::consts::TAU);
            let inner = vec2(x, y) + radius * dir;
            let outer = vec2(x, y) + (radius + thickness) * dir;
            vertices.push(Vertex::new(inner.x, inner.y, 0., 0., 0., color));
            vertices.push(Vertex::new(outer.x, outer.y, 0., 0., 0., color));
            let (a, b) = (2 * i, 2 * i + 1);
            let next = (i + 1) % sides;
            let (c, d) = (2 * next, 2 * next + 1);
            indices.extend_from_slice(&[a, b, c, c, b, d]);
        }
    });
}

/// Drop-in for macroquad's `draw_circle` (same 20-sided shape), without the
/// per-call allocations. Import it by name to shadow the prelude one.
pub fn draw_circle(x: f32, y: f32, r: f32, color: Color) {
    fill_disc(x, y, r, 20, color);
}

/// Drop-in for macroquad's `draw_circle_lines` (same 30-sided ring), without
/// the per-call allocations. Import it by name to shadow the prelude one.
pub fn draw_circle_lines(x: f32, y: f32, r: f32, thickness: f32, color: Color) {
    ring(x, y, r, thickness, 30, color);
}
