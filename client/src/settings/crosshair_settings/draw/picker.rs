//! Cached geometry for the colour picker: the 20x20 saturation/value square,
//! the 36-step hue bar and the 4x4 swatch checkerboard. They are the same
//! quads with the same colours `draw_rectangle` would emit, built once into a
//! reused mesh and rebuilt only when the hue (SV square only), size or
//! position changes, so a still panel does no hsv_to_rgb or per-quad work.

use macroquad::models::Vertex;
use macroquad::prelude::*;
use std::cell::RefCell;

use super::super::shared::hsv_to_rgb;

/// A mesh plus the inputs it was built from (f32 bit patterns).
struct Cached {
    key: Option<[u32; 5]>,
    mesh: Mesh,
}

impl Cached {
    const fn new() -> Self {
        Self {
            key: None,
            mesh: Mesh { vertices: Vec::new(), indices: Vec::new(), texture: None },
        }
    }

    /// Rebuilds the mesh with `build` when `key` differs, then draws it.
    fn draw(&mut self, key: [f32; 5], build: impl FnOnce(&mut Mesh)) {
        let key = key.map(f32::to_bits);
        if self.key != Some(key) {
            self.mesh.vertices.clear();
            self.mesh.indices.clear();
            build(&mut self.mesh);
            self.key = Some(key);
        }
        draw_mesh(&self.mesh);
    }
}

thread_local! {
    static SV_SQUARE: RefCell<Cached> = const { RefCell::new(Cached::new()) };
    static HUE_BAR: RefCell<Cached> = const { RefCell::new(Cached::new()) };
    static CHECKER: RefCell<Cached> = const { RefCell::new(Cached::new()) };
}

/// The same four vertices and six indices as `draw_rectangle`.
fn push_rect(mesh: &mut Mesh, x: f32, y: f32, w: f32, h: f32, color: Color) {
    let base = mesh.vertices.len() as u16;
    mesh.vertices.extend_from_slice(&[
        Vertex::new(x, y, 0., 0.0, 0.0, color),
        Vertex::new(x + w, y, 0., 1.0, 0.0, color),
        Vertex::new(x + w, y + h, 0., 1.0, 1.0, color),
        Vertex::new(x, y + h, 0., 0.0, 1.0, color),
    ]);
    mesh.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

/// Saturation (x) / value (y) square for `hue`, `picker_size` wide.
pub(super) fn draw_sv_square(hue: f32, sv_x: f32, sv_y: f32, picker_size: f32) {
    SV_SQUARE.with_borrow_mut(|cache| {
        cache.draw([hue, sv_x, sv_y, picker_size, 0.0], |mesh| {
            let steps = 20u32;
            let cell_w = picker_size / steps as f32;
            let cell_h = picker_size / steps as f32;
            for sy in 0..steps {
                for sx in 0..steps {
                    let sv_s = (sx as f32 + 0.5) / steps as f32;
                    let v = 1.0 - (sy as f32 + 0.5) / steps as f32;
                    let (r, g, b) = hsv_to_rgb(hue, sv_s, v);
                    push_rect(
                        mesh,
                        sv_x + sx as f32 * cell_w,
                        sv_y + sy as f32 * cell_h,
                        cell_w + 1.0,
                        cell_h + 1.0,
                        Color::new(r, g, b, 1.0),
                    );
                }
            }
        });
    });
}

/// Vertical hue bar, red at the top.
pub(super) fn draw_hue_bar(hue_x: f32, sv_y: f32, hue_bar_w: f32, picker_size: f32) {
    HUE_BAR.with_borrow_mut(|cache| {
        cache.draw([hue_x, sv_y, hue_bar_w, picker_size, 0.0], |mesh| {
            let hue_steps = 36u32;
            let hue_cell_h = picker_size / hue_steps as f32;
            for i in 0..hue_steps {
                let h = (i as f32 / hue_steps as f32) * 360.0;
                let (r, g, b) = hsv_to_rgb(h, 1.0, 1.0);
                push_rect(
                    mesh,
                    hue_x,
                    sv_y + i as f32 * hue_cell_h,
                    hue_bar_w,
                    hue_cell_h + 1.0,
                    Color::new(r, g, b, 1.0),
                );
            }
        });
    });
}

/// 4x4 checkerboard behind the swatch, cells `check` wide.
pub(super) fn draw_checker(swatch_x: f32, sv_y: f32, check: f32) {
    CHECKER.with_borrow_mut(|cache| {
        cache.draw([swatch_x, sv_y, check, 0.0, 0.0], |mesh| {
            for cy_i in 0..4 {
                for cx_i in 0..4 {
                    let col = if (cx_i + cy_i) % 2 == 0 {
                        Color::from_rgba(180, 180, 180, 255)
                    } else {
                        Color::from_rgba(120, 120, 120, 255)
                    };
                    push_rect(
                        mesh,
                        swatch_x + cx_i as f32 * check,
                        sv_y + cy_i as f32 * check,
                        check,
                        check,
                        col,
                    );
                }
            }
        });
    });
}
