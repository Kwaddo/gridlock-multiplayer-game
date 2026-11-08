//! Conservative culling: a mesh is skipped only when none of it can reach
//! the screen (wholly outside one frustum plane) or every cell it belongs to
//! is past the render distance.

use macroquad::prelude::*;

use super::constants::{DISTANCE_SLACK, FRUSTUM_MARGIN};
use super::mesh::Bounds;

/// The six clip planes of a view-projection matrix, as (normal, distance);
/// a point is inside a plane when `normal.dot(p) + distance >= 0`.
pub(super) struct Frustum {
    planes: [Vec4; 6],
}

impl Frustum {
    /// Planes of `view_proj` (Gribb-Hartmann, OpenGL clip space).
    pub fn new(view_proj: Mat4) -> Self {
        let (r0, r1, r2, r3) = (view_proj.row(0), view_proj.row(1), view_proj.row(2), view_proj.row(3));
        Self { planes: [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r3 + r2, r3 - r2] }
    }

    /// False only when the box (`min`, `max`) grown by the margin is wholly
    /// outside some plane, so nothing in it can be drawn on screen.
    pub fn may_see(&self, min: Vec3, max: Vec3) -> bool {
        let (min, max) = (min - Vec3::splat(FRUSTUM_MARGIN), max + Vec3::splat(FRUSTUM_MARGIN));
        if !(min.is_finite() && max.is_finite()) {
            return true;
        }
        self.planes.iter().all(|p| {
            let n = p.truncate();
            if !(n.is_finite() && p.w.is_finite()) {
                return true;
            }
            // The box corner farthest along the plane normal.
            let far = vec3(
                if n.x >= 0.0 { max.x } else { min.x },
                if n.y >= 0.0 { max.y } else { min.y },
                if n.z >= 0.0 { max.z } else { min.z },
            );
            n.dot(far) + p.w >= 0.0
        })
    }
}

/// What the per-cell distance test needs, as the old loop computed it.
#[derive(Clone, Copy)]
pub(super) struct DistanceCull {
    pub viewer: Vec2,
    pub max_dist_sq: f32,
}

impl DistanceCull {
    /// Whether the cell centred at `c` is drawn.
    pub fn keeps(&self, c: Vec2) -> bool {
        let dx = c.x - self.viewer.x;
        let dz = c.y - self.viewer.y;
        dx * dx + dz * dz <= self.max_dist_sq
    }

    /// Whether geometry centred at `c` in a mesh with bounds `b` is drawn
    /// (always, when the mesh is not culled by distance).
    pub fn keeps_in(&self, b: &Bounds, c: Vec2) -> bool {
        !b.cullable || self.keeps(c)
    }

    /// The squared distance the vertex shader culls past, for a mesh with
    /// bounds `b`.
    pub fn shader_limit(&self, b: &Bounds) -> f32 {
        if b.cullable {
            self.max_dist_sq
        } else {
            f32::MAX
        }
    }

    /// Whether any cull centre of `mesh` can be within range.
    pub fn may_keep(&self, b: &Bounds) -> bool {
        if !b.cullable || !b.cull_min.cmple(b.cull_max).all() {
            return true;
        }
        let nearest = self.viewer.clamp(b.cull_min, b.cull_max);
        let dx = nearest.x - self.viewer.x;
        let dz = nearest.y - self.viewer.y;
        // A little slack so float rounding never drops a cell the exact
        // per-cell test would keep.
        dx * dx + dz * dz <= self.max_dist_sq * (1.0 + DISTANCE_SLACK) + DISTANCE_SLACK
    }
}

pub(super) fn visible(b: &Bounds, frustum: &Frustum, dist: &DistanceCull) -> bool {
    dist.may_keep(b) && frustum.may_see(b.min, b.max)
}
