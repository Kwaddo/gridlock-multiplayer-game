use ::rand::{thread_rng, Rng};
use macroquad::prelude::*;
use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Clone, Copy)]
pub struct Room {
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
}

#[derive(Clone, Copy, Debug)]
pub enum PropKind {
    Crate = 1,
    Barrel = 2,
    SupplyCrate = 3,
    Lamp = 4,
    Desk = 5,
    Monitor = 6,
    OfficeChair = 7,
    Foosball = 8,
}

impl PropKind {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(PropKind::Crate),
            2 => Some(PropKind::Barrel),
            3 => Some(PropKind::SupplyCrate),
            4 => Some(PropKind::Lamp),
            5 => Some(PropKind::Desk),
            6 => Some(PropKind::Monitor),
            7 => Some(PropKind::OfficeChair),
            8 => Some(PropKind::Foosball),
            _ => None,
        }
    }

    /// Get collision radius for this prop type (0.0 = no collision)
    pub fn collision_radius(self) -> f32 {
        match self {
            PropKind::Crate => 0.35,
            PropKind::Barrel => 0.28,
            PropKind::SupplyCrate => 0.35,
            PropKind::Lamp => 0.0, // lamps don't block movement
            PropKind::Desk => 0.40,
            PropKind::Monitor => 0.0,     // monitors are on desks
            PropKind::OfficeChair => 0.0, // chairs can be pushed through
            PropKind::Foosball => 0.45,
        }
    }

    /// Returns true if this prop blocks bullets
    pub fn blocks_bullets(self) -> bool {
        match self {
            PropKind::Crate => true,
            PropKind::Barrel => true,
            PropKind::SupplyCrate => true,
            PropKind::Lamp => false,
            PropKind::Desk => true,
            PropKind::Monitor => false,
            PropKind::OfficeChair => false,
            PropKind::Foosball => true,
        }
    }

    /// Get the bullet collision box for this prop (center_offset, half_extents)
    /// Returns None if prop doesn't block bullets
    pub fn bullet_hitbox(self) -> Option<(Vec3, Vec3)> {
        if !self.blocks_bullets() {
            return None;
        }

        // center_offset is relative to cell center, half_extents define box size
        match self {
            PropKind::Crate => Some((vec3(0.0, 0.35, 0.0), vec3(0.33, 0.36, 0.33))),
            PropKind::Barrel => Some((vec3(0.0, 0.45, 0.0), vec3(0.29, 0.46, 0.29))),
            PropKind::SupplyCrate => Some((vec3(0.0, 0.35, 0.0), vec3(0.34, 0.36, 0.34))),
            // Desk: just the desktop surface (thin slab) — avoids bullet holes floating under the desk
            PropKind::Desk => Some((vec3(0.0, 0.65, 0.0), vec3(0.55, 0.04, 0.50))),
            PropKind::Foosball => Some((vec3(0.0, 0.68, 0.0), vec3(0.57, 0.12, 0.36))),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Prop {
    pub kind: PropKind,
    pub x: u8,
    pub y: u8,
}

#[derive(Clone, Copy)]
pub struct CeilingLightInfo {
    pub x: u8,
    pub y: u8,
    pub variant: u8,
}

#[derive(Clone)]
pub struct BreakableWall {
    pub health: f32,
    pub max_health: f32,
}

impl BreakableWall {
    pub fn new() -> Self {
        Self {
            health: 150.0, // Takes ~3-4 shots from basic weapons
            max_health: 150.0,
        }
    }

    pub fn is_destroyed(&self) -> bool {
        self.health <= 0.0
    }
}

pub struct Maze {
    pub width: u8,
    pub height: u8,
    pub cells: Vec<u8>, // 1 = open path, 0 = wall
    pub cell_size: f32,
    pub wall_height: f32,
    pub teleporter: (u8, u8),
    pub rooms: Vec<Room>,
    pub props: Vec<Prop>,
    pub ceiling_lights: Vec<CeilingLightInfo>,
    pub breakable_walls: HashMap<(u8, u8), BreakableWall>, // (x, y) -> wall health
    pub broken_wall_scars: HashSet<(u8, u8)>,
}

impl Maze {
    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width as usize + x
    }

    pub fn is_open(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return false;
        }
        self.cells[self.idx(x as usize, y as usize)] == 1
    }

    pub fn is_wall(&self, x: u8, y: u8) -> bool {
        let idx = self.idx(x as usize, y as usize);
        idx < self.cells.len() && self.cells[idx] == 0
    }

    pub fn get_prop_at(&self, x: u8, y: u8) -> Option<&Prop> {
        self.props.iter().find(|p| p.x == x && p.y == y)
    }

    pub fn cell_from_world(&self, pos: Vec3) -> (i32, i32) {
        let cx = (pos.x / self.cell_size).floor() as i32;
        let cy = (pos.z / self.cell_size).floor() as i32;
        (cx, cy)
    }

    pub fn is_blocked(&self, pos: Vec3, radius: f32) -> bool {
        // Check walls
        for &(ox, oz) in [
            (-radius, -radius),
            (-radius, radius),
            (radius, -radius),
            (radius, radius),
        ]
        .iter()
        {
            let sample = vec3(pos.x + ox, pos.y, pos.z + oz);
            let (cx, cy) = self.cell_from_world(sample);
            if !self.is_open(cx, cy) {
                return true;
            }
        }

        // Check prop collisions
        for prop in &self.props {
            let prop_radius = prop.kind.collision_radius();
            if prop_radius > 0.0 {
                let prop_pos = self.cell_to_world(prop.x as i32, prop.y as i32);
                let dx = pos.x - prop_pos.x;
                let dz = pos.z - prop_pos.z;
                let dist_sq = dx * dx + dz * dz;
                let collision_dist = radius + prop_radius;
                if dist_sq < collision_dist * collision_dist {
                    return true;
                }
            }
        }

        false
    }

    pub fn clamp_move(&self, current: Vec3, desired: Vec3, radius: f32) -> Vec3 {
        let mut result = desired;
        if self.is_blocked(result, radius) {
            result.x = current.x;
            if self.is_blocked(result, radius) {
                result.x = desired.x;
                result.z = current.z;
                if self.is_blocked(result, radius) {
                    result = current;
                }
            }
        }
        result
    }

    pub fn cell_to_world(&self, x: i32, y: i32) -> Vec3 {
        vec3(
            (x as f32 + 0.5) * self.cell_size,
            0.0,
            (y as f32 + 0.5) * self.cell_size,
        )
    }

    pub fn teleporter_world(&self) -> Vec3 {
        self.cell_to_world(self.teleporter.0 as i32, self.teleporter.1 as i32)
    }

    pub fn spawn_point(&self) -> Vec3 {
        let mut open_cells = Vec::new();
        for y in 1..(self.height as i32 - 1) {
            for x in 1..(self.width as i32 - 1) {
                if self.is_open(x, y) {
                    open_cells.push((x, y));
                }
            }
        }

        if !open_cells.is_empty() {
            let mut rng = thread_rng();
            let idx = rng.gen_range(0..open_cells.len());
            let (x, y) = open_cells[idx];
            self.cell_to_world(x, y)
        } else {
            vec3(0.0, 0.0, 0.0)
        }
    }

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

    /// Damage a wall at the given cell position
    /// Returns true if the wall was destroyed
    pub fn damage_wall(&mut self, cell_x: u8, cell_y: u8, damage: f32) -> bool {
        // Check if it's a breakable wall
        if let Some(wall) = self.breakable_walls.get_mut(&(cell_x, cell_y)) {
            wall.health -= damage;

            if wall.is_destroyed() {
                // Remove the wall from the maze
                let idx = self.idx(cell_x as usize, cell_y as usize);
                self.cells[idx] = 1; // Make it open
                self.breakable_walls.remove(&(cell_x, cell_y));
                self.broken_wall_scars.insert((cell_x, cell_y));
                return true;
            }
        }
        false
    }

    pub fn force_break_wall(&mut self, cell_x: u8, cell_y: u8) {
        let idx = self.idx(cell_x as usize, cell_y as usize);
        if idx < self.cells.len() {
            self.cells[idx] = 1;
        }
        self.breakable_walls.remove(&(cell_x, cell_y));
        self.broken_wall_scars.insert((cell_x, cell_y));
    }

    pub fn is_broken_wall(&self, cell_x: u8, cell_y: u8) -> bool {
        self.broken_wall_scars.contains(&(cell_x, cell_y))
    }

    /// Check if a wall is breakable
    pub fn is_breakable(&self, cell_x: u8, cell_y: u8) -> bool {
        self.breakable_walls.contains_key(&(cell_x, cell_y))
    }

    /// Get wall health percentage (0.0 to 1.0)
    pub fn wall_health_percent(&self, cell_x: u8, cell_y: u8) -> Option<f32> {
        self.breakable_walls
            .get(&(cell_x, cell_y))
            .map(|w| w.health / w.max_health)
    }

    /// Spawn random breakable walls (about 2-3% of walls)
    pub fn spawn_breakable_walls(&mut self) {
        self.breakable_walls.clear();
        self.broken_wall_scars.clear();

        for y in 0..self.height {
            for x in 0..self.width {
                if self.cells[self.idx(x as usize, y as usize)] == 0 {
                    // Only make interior walls breakable (not outer edges)
                    let is_interior = x > 0 && x < self.width - 1 && y > 0 && y < self.height - 1;

                    if is_interior {
                        let hash = ((x as u32).wrapping_mul(73856093))
                            ^ ((y as u32).wrapping_mul(19349663))
                            ^ ((self.width as u32).wrapping_mul(83492791))
                            ^ ((self.height as u32).wrapping_mul(2654435761));
                        let roll = (hash % 1000) as f32 / 1000.0;

                        // Don't make walls adjacent to teleporter breakable
                        let dist_to_tele = ((x as i32 - self.teleporter.0 as i32).abs()
                            + (y as i32 - self.teleporter.1 as i32).abs())
                            as u8;

                        if dist_to_tele > 3 && roll < 0.025 {
                            self.breakable_walls.insert((x, y), BreakableWall::new());
                        }
                    }
                }
            }
        }
    }
}
