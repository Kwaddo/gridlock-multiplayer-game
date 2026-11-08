use macroquad::prelude::*;
use std::collections::HashSet;

use super::Maze;

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

        let mut closest_hit: Option<((u8, u8), f32)> = None;

        // Check each wall cell for collision
        for y in 0..self.height {
            for x in 0..self.width {
                if self.cells[self.idx(x as usize, y as usize)] == 0 {
                    // This is a wall
                    let wall_center = self.cell_to_world(x as i32, y as i32);
                    let box_center = wall_center + vec3(0.0, self.wall_height * 0.5, 0.0);
                    let half_extents = vec3(
                        self.cell_size * 0.5,
                        self.wall_height * 0.5,
                        self.cell_size * 0.5,
                    );

                    if let Some(dist) =
                        ray_hit_aabb(origin, dir, box_center, half_extents, max_dist)
                    {
                        let is_closer = closest_hit.map_or(true, |(_, d)| dist < d);
                        if is_closer {
                            closest_hit = Some(((x, y), dist));
                        }
                    }
                }
            }
        }

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

        let mut closest_hit: Option<((u8, u8), f32, Vec3)> = None;

        for y in 0..self.height {
            for x in 0..self.width {
                if self.cells[self.idx(x as usize, y as usize)] == 0 {
                    let wall_center = self.cell_to_world(x as i32, y as i32);
                    let box_center = wall_center + vec3(0.0, self.wall_height * 0.5, 0.0);
                    let half_extents = vec3(
                        self.cell_size * 0.5,
                        self.wall_height * 0.5,
                        self.cell_size * 0.5,
                    );

                    if let Some((dist, normal)) =
                        ray_hit_aabb_with_normal(origin, dir, box_center, half_extents, max_dist)
                    {
                        let is_closer = closest_hit.map_or(true, |(_, d, _)| dist < d);
                        if is_closer {
                            closest_hit = Some(((x, y), dist, normal));
                        }
                    }
                }
            }
        }

        closest_hit
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
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.idx(x as usize, y as usize);
                if self.cells[idx] == 0 {
                    continue; // wall cells don't have ceiling panels
                }
                let center = vec3(
                    (x as f32 + 0.5) * self.cell_size,
                    panel_y,
                    (y as f32 + 0.5) * self.cell_size,
                );
                if let Some((d, n)) = ray_hit_aabb_with_normal(
                    origin, dir, center,
                    vec3(panel_half_xz, panel_half_y, panel_half_xz),
                    max_dist,
                ) {
                    if best.map_or(true, |(b, _)| d < b) {
                        best = Some((d, n));
                    }
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
