use macroquad::prelude::*;
use std::collections::HashSet;

use super::Maze;

mod grid_walk;
#[cfg(test)]
mod reference;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod reference_tests;

/// One maze surface hit: (distance, normal, wall cell if it was a wall,
/// light cell if it was a ceiling light).
pub type SurfaceHit = (f32, Vec3, Option<(u8, u8)>, Option<(u8, u8)>);

impl Maze {
    /// Test if a ray hits any props that block bullets
    /// Returns the distance to the closest blocking prop, or None if no hit
    pub fn ray_test_props(&self, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<f32> {
        use crate::weapons::combat::{ray_hit_aabb_inv, ray_inv_dir};

        let inv_dir = ray_inv_dir(dir);
        let mut closest_dist: Option<f32> = None;

        for prop in &self.props {
            if let Some((center_offset, half_extents)) = prop.kind.bullet_hitbox() {
                let prop_world_pos = self.cell_to_world(prop.x as i32, prop.y as i32);
                let box_center = prop_world_pos + center_offset;

                if let Some(dist) =
                    ray_hit_aabb_inv(origin, dir, inv_dir, box_center, half_extents, max_dist)
                {
                    closest_dist = match closest_dist {
                        None => Some(dist),
                        Some(d) if dist < d => Some(dist),
                        Some(d) => Some(d),
                    };
                }
            }
        }

        closest_dist
    }

    /// Test if a ray hits any walls
    /// Returns (cell_x, cell_y, distance) of the closest wall hit, or None
    pub fn ray_test_walls(
        &self,
        origin: Vec3,
        dir: Vec3,
        max_dist: f32,
    ) -> Option<((u8, u8), f32)> {
        use crate::weapons::combat::{ray_hit_aabb_inv, ray_inv_dir};

        let inv_dir = ray_inv_dir(dir);
        self.closest_wall_hit(origin, dir, max_dist, |center, half| {
            ray_hit_aabb_inv(origin, dir, inv_dir, center, half, max_dist).map(|d| (d, ()))
        })
        .map(|(cell, d, ())| (cell, d))
    }

    /// True when some wall hit lies closer than `before` along the ray. Same
    /// answer as `ray_test_walls(origin, dir, max_dist)` followed by
    /// `wall_dist < before`, but returns at the first such wall instead of
    /// finding the closest one.
    pub fn ray_hits_wall_before(&self, origin: Vec3, dir: Vec3, max_dist: f32, before: f32) -> bool {
        use crate::weapons::combat::{ray_hit_aabb_inv, ray_inv_dir};

        let (y0, y1) = match self.wall_slab_interval(origin, dir, max_dist) {
            Some(range) => range,
            None => return false,
        };
        let inv_dir = ray_inv_dir(dir);
        let mut blocked = false;
        let mut prev = None;
        self.walk_cells(origin, dir, y0, y1, |cx, cy, t_enter| {
            if t_enter > before + grid_walk::walk_slack(before) {
                return false;
            }
            for (x, y) in self.fresh_neighbourhood(cx, cy, prev) {
                if self.cells[self.idx(x as usize, y as usize)] != 0 {
                    continue;
                }
                let (center, half) = self.wall_box(x, y);
                if let Some(d) = ray_hit_aabb_inv(origin, dir, inv_dir, center, half, max_dist) {
                    if d < before {
                        blocked = true;
                        return false;
                    }
                }
            }
            prev = Some((cx, cy));
            true
        });
        blocked
    }

    /// The cells of (cx, cy)'s 3x3 neighbourhood that the neighbourhood of
    /// `prev`, the cell walked just before, did not already cover. Each walk
    /// callback tests every cell it is given before the walk moves on, and
    /// testing a box again gives the same answer, which can neither beat the
    /// best hit it already fed nor trip an early exit it did not trip before.
    /// So skipping the overlap changes nothing but the work: each cell near
    /// the ray is box-tested once instead of up to 5 times.
    fn fresh_neighbourhood(
        &self,
        cx: i32,
        cy: i32,
        prev: Option<(i32, i32)>,
    ) -> impl Iterator<Item = (u8, u8)> {
        self.neighbourhood(cx, cy).filter(move |&(x, y)| match prev {
            Some((px, py)) => (x as i32 - px).abs() > 1 || (y as i32 - py).abs() > 1,
            None => true,
        })
    }

    /// Box of the full-height wall column at cell (x, y), as (center, half extents).
    fn wall_box(&self, x: u8, y: u8) -> (Vec3, Vec3) {
        let wall_center = self.cell_to_world(x as i32, y as i32);
        let box_center = wall_center + vec3(0.0, self.wall_height * 0.5, 0.0);
        let half_extents = vec3(
            self.cell_size * 0.5,
            self.wall_height * 0.5,
            self.cell_size * 0.5,
        );
        (box_center, half_extents)
    }

    /// t-range (within [0, max_dist]) where the ray is at wall height, padded.
    fn wall_slab_interval(&self, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<(f32, f32)> {
        let m = grid_walk::CLIP_MARGIN;
        let (a, b) = grid_walk::slab_interval(origin.y, dir.y, -m, self.wall_height + m)?;
        let (t0, t1) = (a.max(0.0), b.min(max_dist));
        (t0 <= t1).then_some((t0, t1))
    }

    /// Closest wall cell hit by the ray, walking only the cells along it.
    /// `test` is the exact per-box ray test; ties keep the cell the old
    /// row-major scan of every cell would have kept.
    fn closest_wall_hit<T: Copy>(
        &self,
        origin: Vec3,
        dir: Vec3,
        max_dist: f32,
        test: impl Fn(Vec3, Vec3) -> Option<(f32, T)>,
    ) -> Option<((u8, u8), f32, T)> {
        let (y0, y1) = self.wall_slab_interval(origin, dir, max_dist)?;
        let mut closest_hit: Option<((u8, u8), f32, T)> = None;
        let mut prev = None;

        // Check each wall cell for collision
        self.walk_cells(origin, dir, y0, y1, |cx, cy, t_enter| {
            if let Some((_, d, _)) = closest_hit {
                if t_enter > d + grid_walk::walk_slack(d) {
                    return false;
                }
            }
            for (x, y) in self.fresh_neighbourhood(cx, cy, prev) {
                if self.cells[self.idx(x as usize, y as usize)] != 0 {
                    continue;
                }
                // This is a wall
                let (center, half) = self.wall_box(x, y);
                if let Some((dist, extra)) = test(center, half) {
                    let best = closest_hit.map(|(c, d, _)| (c, d));
                    if grid_walk::is_better(dist, (x, y), best) {
                        closest_hit = Some(((x, y), dist, extra));
                    }
                }
            }
            prev = Some((cx, cy));
            true
        });

        closest_hit
    }

    /// Like `ray_test_props` but also returns the surface normal of the hit face.
    pub fn ray_test_props_with_normal(&self, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<(f32, Vec3)> {
        use crate::weapons::combat::{ray_hit_aabb_with_normal_inv, ray_inv_dir};

        let inv_dir = ray_inv_dir(dir);
        // Every prop box lies in one height band: a ray that stays above or
        // below it within range (a level shot from eye height) hits none.
        let (band_lo, band_hi) = super::PropKind::BULLET_HITBOX_Y;
        if !ray_reaches_band(origin, dir, inv_dir, band_lo, band_hi, max_dist) {
            return None;
        }
        let mut closest: Option<(f32, Vec3)> = None;

        for prop in &self.props {
            if let Some((center_offset, half_extents)) = prop.kind.bullet_hitbox() {
                let prop_world_pos = self.cell_to_world(prop.x as i32, prop.y as i32);
                let box_center = prop_world_pos + center_offset;

                if let Some((dist, normal)) = ray_hit_aabb_with_normal_inv(
                    origin,
                    dir,
                    inv_dir,
                    box_center,
                    half_extents,
                    max_dist,
                ) {
                    if closest.map_or(true, |(d, _)| dist < d) {
                        closest = Some((dist, normal));
                    }
                }
            }
        }

        closest
    }

    /// Like `ray_test_walls` but also returns the surface normal of the hit face.
    pub fn ray_test_walls_with_normal(
        &self,
        origin: Vec3,
        dir: Vec3,
        max_dist: f32,
    ) -> Option<((u8, u8), f32, Vec3)> {
        use crate::weapons::combat::{ray_hit_aabb_with_normal_inv, ray_inv_dir};

        let inv_dir = ray_inv_dir(dir);
        self.closest_wall_hit(origin, dir, max_dist, |center, half| {
            ray_hit_aabb_with_normal_inv(origin, dir, inv_dir, center, half, max_dist)
        })
    }

    /// Ray-test the maze floor and ceiling panels. Returns closest hit distance and surface normal.
    pub fn ray_test_floor_ceiling_with_normal(
        &self,
        origin: Vec3,
        dir: Vec3,
        max_dist: f32,
    ) -> Option<(f32, Vec3)> {
        use crate::weapons::combat::{ray_hit_aabb_with_normal_inv, ray_inv_dir};

        let inv_dir = ray_inv_dir(dir);
        let map_w = self.width as f32 * self.cell_size;
        let map_h = self.height as f32 * self.cell_size;

        let mut best: Option<(f32, Vec3)> = None;

        // Floor: thin slab at y = -0.025, size 0.05 thick
        let floor_center = vec3(map_w * 0.5, -0.025, map_h * 0.5);
        let floor_half = vec3(map_w * 0.5, 0.025, map_h * 0.5);
        if let Some((d, n)) =
            ray_hit_aabb_with_normal_inv(origin, dir, inv_dir, floor_center, floor_half, max_dist)
        {
            best = Some((d, n));
        }

        // Ceiling: individual per-cell panels (matches rendering in maze.rs)
        let roof_clearance = 0.05;
        let roof_bottom_y = self.wall_height + roof_clearance;
        let panel_inset = 0.03;
        let panel_thickness = 0.03;
        let panel_drop = 0.018;
        let panel_y = roof_bottom_y - panel_drop;
        let panel_half_xz = self.cell_size * (1.0 - panel_inset) * 0.5;
        let panel_half_y = panel_thickness * 0.5;

        // Only check cells near the ray to avoid iterating the entire maze
        let panel_half = vec3(panel_half_xz, panel_half_y, panel_half_xz);
        let m = grid_walk::CLIP_MARGIN;
        let panel_span = grid_walk::slab_interval(
            origin.y,
            dir.y,
            panel_y - panel_half_y - m,
            panel_y + panel_half_y + m,
        );
        if let Some((a, b)) = panel_span {
            let mut panel_best: Option<((u8, u8), f32, Vec3)> = None;
            let mut prev = None;
            self.walk_cells(origin, dir, a.max(0.0), b.min(max_dist), |cx, cy, t_enter| {
                // Panels only matter when strictly closer than the best so far.
                let bound = match (panel_best, best) {
                    (Some((_, pd, _)), _) => Some(pd),
                    (None, Some((fd, _))) => Some(fd),
                    (None, None) => None,
                };
                if bound.map_or(false, |d| t_enter > d + grid_walk::walk_slack(d)) {
                    return false;
                }
                for (x, y) in self.fresh_neighbourhood(cx, cy, prev) {
                    let idx = self.idx(x as usize, y as usize);
                    if self.cells[idx] == 0 {
                        continue; // wall cells don't have ceiling panels
                    }
                    let center = vec3(
                        (x as f32 + 0.5) * self.cell_size,
                        panel_y,
                        (y as f32 + 0.5) * self.cell_size,
                    );
                    if let Some((d, n)) = ray_hit_aabb_with_normal_inv(
                        origin, dir, inv_dir, center, panel_half, max_dist,
                    ) {
                        let pb = panel_best.map(|(c, pd, _)| (c, pd));
                        if grid_walk::is_better(d, (x, y), pb) {
                            panel_best = Some(((x, y), d, n));
                        }
                    }
                }
                prev = Some((cx, cy));
                true
            });
            // The old scan visited panels after the floor, replacing it only
            // when strictly closer; the earliest closest panel wins ties.
            if let Some((_, d, n)) = panel_best {
                if best.map_or(true, |(b, _)| d < b) {
                    best = Some((d, n));
                }
            }
        }

        // Fallback: the solid roof slab above all panels catches anything that misses gaps
        let roof_thickness = 0.06;
        let roof_top_y = roof_bottom_y + roof_thickness;
        let roof_center = vec3(map_w * 0.5, (roof_bottom_y + roof_top_y) * 0.5, map_h * 0.5);
        let roof_half = vec3(map_w * 0.505, roof_thickness * 0.5, map_h * 0.505);
        if let Some((d, n)) =
            ray_hit_aabb_with_normal_inv(origin, dir, inv_dir, roof_center, roof_half, max_dist)
        {
            if best.map_or(true, |(b, _)| d < b) {
                best = Some((d, n));
            }
        }

        best
    }

    /// Ray-test ceiling lights. Returns the cell (x,y) of the closest hit light and distance + normal.
    pub fn ray_test_ceiling_lights_with_normal(
        &self,
        origin: Vec3,
        dir: Vec3,
        max_dist: f32,
        destroyed_lights: &HashSet<(u8, u8)>,
    ) -> Option<((u8, u8), f32, Vec3)> {
        use crate::weapons::combat::{ray_hit_aabb_with_normal_inv, ray_inv_dir};

        let inv_dir = ray_inv_dir(dir);
        // Every housing sits at the same height (cell_to_world's y is 0 for
        // every cell): a ray that never gets there within range hits none.
        let (band_center, band_half) = self.light_housing(0, 0);
        let (band_lo, band_hi) = (band_center.y - band_half.y, band_center.y + band_half.y);
        if !ray_reaches_band(origin, dir, inv_dir, band_lo, band_hi, max_dist) {
            return None;
        }

        let mut best: Option<((u8, u8), f32, Vec3)> = None;

        for info in &self.ceiling_lights {
            let (housing_center, housing_half) = self.light_housing(info.x, info.y);

            if let Some((d, n)) = ray_hit_aabb_with_normal_inv(
                origin,
                dir,
                inv_dir,
                housing_center,
                housing_half,
                max_dist,
            ) {
                // skip already destroyed lights; only hashed for a hit that would win
                if best.map_or(true, |(_, bd, _)| d < bd)
                    && !destroyed_lights.contains(&(info.x, info.y))
                {
                    best = Some(((info.x, info.y), d, n));
                }
            }
        }

        best
    }

    /// Bullet box of the ceiling light housing in cell (x, y), as (center, half extents).
    fn light_housing(&self, x: u8, y: u8) -> (Vec3, Vec3) {
        let roof_clearance = 0.05;
        let roof_bottom_y = self.wall_height + roof_clearance;
        let light_pos = self.cell_to_world(x as i32, y as i32)
            + vec3(0.0, roof_bottom_y - 0.16, 0.0);
        // housing hitbox: 0.44 x 0.06 x 0.44 centered at light_pos + (0, 0.02, 0)
        let housing_center = light_pos + vec3(0.0, 0.02, 0.0);
        let housing_half = vec3(0.22, 0.05, 0.22);
        (housing_center, housing_half)
    }

    /// The closest maze surface the ray hits within `max_dist`: walls, props,
    /// floor and ceiling, and lights not in `destroyed_lights`.
    pub fn ray_test_surfaces(
        &self,
        origin: Vec3,
        dir: Vec3,
        max_dist: f32,
        destroyed_lights: &HashSet<(u8, u8)>,
    ) -> Option<SurfaceHit> {
        // Tested in this order; a later surface only wins when strictly
        // closer, so on equal distance the earlier one wins, as the stable
        // sort this replaces did.
        fn keep(best: &mut Option<SurfaceHit>, c: SurfaceHit) {
            if !matches!(best, Some(b) if !(c.0 < b.0)) {
                *best = Some(c);
            }
        }
        // A box's hit distance does not depend on max_dist, only whether the
        // hit counts, and only hits strictly closer than `best` can win. So
        // each test only has to reach as far as the best hit so far.
        let reach = |best: &Option<SurfaceHit>| best.map_or(max_dist, |b| b.0.min(max_dist));

        let mut best = self
            .ray_test_walls_with_normal(origin, dir, max_dist)
            .map(|(cell, d, n)| (d, n, Some(cell), None));
        if let Some((d, n)) = self.ray_test_props_with_normal(origin, dir, reach(&best)) {
            keep(&mut best, (d, n, None, None));
        }
        if let Some((d, n)) = self.ray_test_floor_ceiling_with_normal(origin, dir, reach(&best)) {
            keep(&mut best, (d, n, None, None));
        }
        let light_reach = reach(&best);
        if let Some((cell, d, n)) =
            self.ray_test_ceiling_lights_with_normal(origin, dir, light_reach, destroyed_lights)
        {
            keep(&mut best, (d, n, None, Some(cell)));
        }
        best
    }
}

/// False only when the ray is never at a height in `lo..=hi` for t in
/// `[0, max_dist]`. It is the exact slab test `ray_hit_aabb` runs, on a box
/// spanning that band and unbounded in x and z, so its ray interval holds the
/// one of every box inside the band: when it misses, each of those misses.
fn ray_reaches_band(origin: Vec3, dir: Vec3, inv_dir: Vec3, lo: f32, hi: f32, max_dist: f32) -> bool {
    use crate::weapons::combat::ray_hit_bounds;

    // The containment argument needs finite inputs; otherwise test every box.
    if !origin.is_finite() || !dir.is_finite() {
        return true;
    }
    let min = vec3(f32::NEG_INFINITY, lo, f32::NEG_INFINITY);
    let max = vec3(f32::INFINITY, hi, f32::INFINITY);
    ray_hit_bounds(origin, dir, inv_dir, min, max, max_dist).is_some()
}
