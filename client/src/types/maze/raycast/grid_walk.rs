//! Grid traversal (Amanatides-Woo DDA) over the maze's XZ cell grid, so ray
//! tests only look at cells near the ray instead of every cell in the maze.
//!
//! The walk is conservative: callers test the 3x3 neighbourhood of every cell
//! it steps through with the exact same AABB test the brute-force loop used,
//! and keep the closest hit with the same tie-break (lowest row, then
//! column). Any box the ray touches, even on a shared face or a corner, lies
//! in the neighbourhood of a walked cell, so the result is bit-identical.

use macroquad::prelude::*;

use super::super::Maze;

/// Direction components below this are treated as parallel to that axis,
/// matching `ray_hit_aabb`.
const PARALLEL_EPS: f32 = 1e-8;

/// Extra world-space margin around clip slabs, far above f32 rounding of the
/// box bounds, so clipping never drops a box the exact test would hit.
pub(super) const CLIP_MARGIN: f32 = 1e-3;

/// The t-interval in which `o + t * d` lies within `[lo, hi]` on one axis, or
/// None when a parallel ray sits outside it.
pub(super) fn slab_interval(o: f32, d: f32, lo: f32, hi: f32) -> Option<(f32, f32)> {
    if d.abs() < PARALLEL_EPS {
        if o < lo || o > hi {
            None
        } else {
            Some((f32::NEG_INFINITY, f32::INFINITY))
        }
    } else {
        let t1 = (lo - o) / d;
        let t2 = (hi - o) / d;
        Some((t1.min(t2), t1.max(t2)))
    }
}

/// Slack on "the next cell starts past the best hit" before a walk stops.
pub(super) fn walk_slack(best: f32) -> f32 {
    1e-3 + best.abs() * 1e-5
}

/// Strict "closer" with the brute-force tie-break: equal distances go to the
/// cell met first in row-major order (lowest y, then lowest x).
pub(super) fn is_better(d: f32, cell: (u8, u8), best: Option<((u8, u8), f32)>) -> bool {
    match best {
        None => true,
        Some(((bx, by), bd)) => d < bd || (d == bd && (cell.1, cell.0) < (by, bx)),
    }
}

impl Maze {
    /// In-bounds cells of the 3x3 neighbourhood around grid cell (cx, cy).
    pub(super) fn neighbourhood(&self, cx: i32, cy: i32) -> impl Iterator<Item = (u8, u8)> {
        let (w, h) = (self.width as i32, self.height as i32);
        (cy - 1..=cy + 1)
            .filter(move |&y| y >= 0 && y < h)
            .flat_map(move |y| {
                (cx - 1..=cx + 1)
                    .filter(move |&x| x >= 0 && x < w)
                    .map(move |x| (x as u8, y as u8))
            })
    }

    /// Walks, in ray order, the XZ grid cells the ray passes through for t in
    /// `[t0, t1]` (clipped to the grid plus a one-cell border). Calls
    /// `visit(cell_x, cell_y, t_enter)` per cell, which may be out of bounds;
    /// returning false stops the walk.
    pub(super) fn walk_cells(
        &self,
        origin: Vec3,
        dir: Vec3,
        t0: f32,
        t1: f32,
        mut visit: impl FnMut(i32, i32, f32) -> bool,
    ) {
        // Degenerate input would make the walk meaningless; nothing to hit.
        if !origin.is_finite() || !dir.is_finite() || !(t0 <= t1) {
            return;
        }
        let cs = self.cell_size;
        let (Some(sx), Some(sz)) = (
            slab_interval(origin.x, dir.x, -cs, (self.width as f32 + 1.0) * cs),
            slab_interval(origin.z, dir.z, -cs, (self.height as f32 + 1.0) * cs),
        ) else {
            return;
        };
        let start = t0.max(sx.0).max(sz.0);
        let end = t1.min(sx.1).min(sz.1);
        if !(start <= end) || !start.is_finite() {
            return;
        }

        let p = origin + dir * start;
        let mut cx = (p.x / cs).floor() as i32;
        let mut cy = (p.z / cs).floor() as i32;
        let par_x = dir.x.abs() < PARALLEL_EPS;
        let par_z = dir.z.abs() < PARALLEL_EPS;
        let step_x = if dir.x > 0.0 { 1 } else { -1 };
        let step_y = if dir.z > 0.0 { 1 } else { -1 };
        let mut t_enter = start;

        loop {
            if !visit(cx, cy, t_enter) {
                return;
            }
            // Crossing times of the next cell boundary on each axis, computed
            // from the origin each step so no error accumulates.
            let next_x = if par_x {
                f32::INFINITY
            } else {
                let bx = (if step_x > 0 { cx + 1 } else { cx }) as f32 * cs;
                (bx - origin.x) / dir.x
            };
            let next_z = if par_z {
                f32::INFINITY
            } else {
                let bz = (if step_y > 0 { cy + 1 } else { cy }) as f32 * cs;
                (bz - origin.z) / dir.z
            };
            let t_next = next_x.min(next_z);
            if !t_next.is_finite() || t_next > end {
                return;
            }
            if next_x <= next_z {
                cx += step_x;
            } else {
                cy += step_y;
            }
            t_enter = t_enter.max(t_next);
        }
    }
}
