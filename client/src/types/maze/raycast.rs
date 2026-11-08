use macroquad::prelude::*;
use std::collections::HashSet;

use super::Maze;

mod grid_walk;
#[cfg(test)]
mod tests;

impl Maze {
    /// Test if a ray hits any props that block bullets
    /// Returns the distance to the closest blocking prop, or None if no hit
    pub fn ray_test_props(&self, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<f32> {
        use crate::weapons::combat::ray_hit_aabb;

        let mut closest_dist: Option<f32> = None;

        for prop in &self.props {
            if let Some((center_offset, half_extents)) = prop.kind.bullet_hitbox() {
                let prop_world_pos = self.cell_to_world(prop.x as i32, prop.y as i32);
                let box_center = prop_world_pos + center_offset;

                if let Some(dist) = ray_hit_aabb(origin, dir, box_center, half_extents, max_dist) {
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
        use crate::weapons::combat::ray_hit_aabb;

        self.closest_wall_hit(origin, dir, max_dist, |center, half| {
            ray_hit_aabb(origin, dir, center, half, max_dist).map(|d| (d, ()))
        })
        .map(|(cell, d, ())| (cell, d))
    }

    /// True when some wall hit lies closer than `before` along the ray. Same
    /// answer as `ray_test_walls(origin, dir, max_dist)` followed by
    /// `wall_dist < before`, but returns at the first such wall instead of
    /// finding the closest one.
    pub fn ray_hits_wall_before(&self, origin: Vec3, dir: Vec3, max_dist: f32, before: f32) -> bool {
        use crate::weapons::combat::ray_hit_aabb;

        let (y0, y1) = match self.wall_slab_interval(origin, dir, max_dist) {
            Some(range) => range,
            None => return false,
        };
        let mut blocked = false;
        self.walk_cells(origin, dir, y0, y1, |cx, cy, t_enter| {
            if t_enter > before + grid_walk::walk_slack(before) {
                return false;
            }
            for (x, y) in self.neighbourhood(cx, cy) {
                if self.cells[self.idx(x as usize, y as usize)] != 0 {
                    continue;
                }
                let (center, half) = self.wall_box(x, y);
                if let Some(d) = ray_hit_aabb(origin, dir, center, half, max_dist) {
                    if d < before {
                        blocked = true;
                        return false;
                    }
                }
            }
            true
        });
        blocked
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

        // Check each wall cell for collision
        self.walk_cells(origin, dir, y0, y1, |cx, cy, t_enter| {
            if let Some((_, d, _)) = closest_hit {
                if t_enter > d + grid_walk::walk_slack(d) {
                    return false;
                }
            }
            for (x, y) in self.neighbourhood(cx, cy) {
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
            true
        });

        closest_hit
    }

    /// Like `ray_test_props` but also returns the surface normal of the hit face.
    pub fn ray_test_props_with_normal(&self, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<(f32, Vec3)> {
        use crate::weapons::combat::ray_hit_aabb_with_normal;

        let mut closest: Option<(f32, Vec3)> = None;

        for prop in &self.props {
            if let Some((center_offset, half_extents)) = prop.kind.bullet_hitbox() {
                let prop_world_pos = self.cell_to_world(prop.x as i32, prop.y as i32);
                let box_center = prop_world_pos + center_offset;

                if let Some((dist, normal)) = ray_hit_aabb_with_normal(origin, dir, box_center, half_extents, max_dist) {
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
        use crate::weapons::combat::ray_hit_aabb_with_normal;

        self.closest_wall_hit(origin, dir, max_dist, |center, half| {
            ray_hit_aabb_with_normal(origin, dir, center, half, max_dist)
        })
    }

    /// Ray-test the maze floor and ceiling panels. Returns closest hit distance and surface normal.
    pub fn ray_test_floor_ceiling_with_normal(
        &self,
        origin: Vec3,
        dir: Vec3,
        max_dist: f32,
    ) -> Option<(f32, Vec3)> {
        use crate::weapons::combat::ray_hit_aabb_with_normal;

        let map_w = self.width as f32 * self.cell_size;
        let map_h = self.height as f32 * self.cell_size;

        let mut best: Option<(f32, Vec3)> = None;

        // Floor: thin slab at y = -0.025, size 0.05 thick
        let floor_center = vec3(map_w * 0.5, -0.025, map_h * 0.5);
        let floor_half = vec3(map_w * 0.5, 0.025, map_h * 0.5);
        if let Some((d, n)) = ray_hit_aabb_with_normal(origin, dir, floor_center, floor_half, max_dist) {
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
                for (x, y) in self.neighbourhood(cx, cy) {
                    let idx = self.idx(x as usize, y as usize);
                    if self.cells[idx] == 0 {
                        continue; // wall cells don't have ceiling panels
                    }
                    let center = vec3(
                        (x as f32 + 0.5) * self.cell_size,
                        panel_y,
                        (y as f32 + 0.5) * self.cell_size,
                    );
                    if let Some((d, n)) =
                        ray_hit_aabb_with_normal(origin, dir, center, panel_half, max_dist)
                    {
                        let pb = panel_best.map(|(c, pd, _)| (c, pd));
                        if grid_walk::is_better(d, (x, y), pb) {
                            panel_best = Some(((x, y), d, n));
                        }
                    }
                }
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
        if let Some((d, n)) = ray_hit_aabb_with_normal(origin, dir, roof_center, roof_half, max_dist) {
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
        use crate::weapons::combat::ray_hit_aabb_with_normal;

        let roof_clearance = 0.05;
        let roof_bottom_y = self.wall_height + roof_clearance;

        let mut best: Option<((u8, u8), f32, Vec3)> = None;

        for info in &self.ceiling_lights {
            // skip already destroyed lights
            if destroyed_lights.contains(&(info.x, info.y)) {
                continue;
            }
            let light_pos = self.cell_to_world(info.x as i32, info.y as i32)
                + vec3(0.0, roof_bottom_y - 0.16, 0.0);
            // housing hitbox: 0.44 x 0.06 x 0.44 centered at light_pos + (0, 0.02, 0)
            let housing_center = light_pos + vec3(0.0, 0.02, 0.0);
            let housing_half = vec3(0.22, 0.05, 0.22);

            if let Some((d, n)) = ray_hit_aabb_with_normal(origin, dir, housing_center, housing_half, max_dist) {
                if best.map_or(true, |(_, bd, _)| d < bd) {
                    best = Some(((info.x, info.y), d, n));
                }
            }
        }

        best
    }
}
