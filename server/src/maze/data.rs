use crate::net::{Outbox, PeerId};

use rand::Rng;

use crate::maze::generation::generate_maze;
use crate::maze::types::{MazeLight, Prop, Room};
use crate::maze::analysis::{ self, MazeAnalysis };
use crate::protocols::{self as wire, ServerMsg};

pub struct MazeData {
    pub width: u8,
    pub height: u8,
    pub cells: Vec<u8>,
    pub seed: u64,
    pub rooms: Vec<Room>,
    pub props: Vec<Prop>,
    pub lights: Vec<MazeLight>,
    pub teleporter: (u8, u8),
    pub analysis: MazeAnalysis,
}

impl MazeData {
    pub fn new(width: u8, height: u8) -> Self {
        let seed = rand::thread_rng().gen();
        let (cells, rooms, props, lights, teleporter) = generate_maze(width, height, seed);
        let maze_analysis = analysis::compute(&cells, width, height, &rooms);
        Self {
            width,
            height,
            cells,
            seed,
            rooms,
            props,
            lights,
            teleporter,
            analysis: maze_analysis,
        }
    }

    pub fn send(&self, socket: &Outbox, addr: PeerId) {
        let layout = wire::MazeLayout {
            width: self.width,
            height: self.height,
            teleporter: self.teleporter,
            rooms: self.rooms.iter().map(|r| wire::MazeRoom { x: r.x, y: r.y, w: r.w, h: r.h }).collect(),
            props: self
                .props
                .iter()
                .map(|p| wire::MazeProp { kind: p.kind.as_u8(), x: p.x, y: p.y })
                .collect(),
            lights: self
                .lights
                .iter()
                .map(|l| wire::MazeLight { x: l.x, y: l.y, variant: l.variant })
                .collect(),
            cells: self.cells.clone(),
        };
        let msg = ServerMsg::Maze(layout).encode();
        let _ = socket.send_to(&msg, addr);
    }

    pub fn teleporter_world(&self, cell_size: f32) -> (f32, f32, f32) {
        let (tx, ty) = self.teleporter;
        (
            (tx as f32 + 0.5) * cell_size,
            0.0,
            (ty as f32 + 0.5) * cell_size,
        )
    }

    /// Determine two deterministic team spawn positions (world coords).
    /// Picks two room centers (if rooms available) that are farthest apart.
    /// Falls back to opposite maze corners if rooms are absent.
    pub fn team_spawn_positions(&self, cell_size: f32) -> ((f32, f32, f32), (f32, f32, f32)) {
        // Collect candidate centers from rooms
        let mut centers: Vec<(f32, f32)> = Vec::new();
        for r in &self.rooms {
            // center cell of the room (may be fractional when width/height even)
            let cx = r.x as f32 + (r.w as f32) / 2.0;
            let cy = r.y as f32 + (r.h as f32) / 2.0;
            centers.push((cx, cy));
        }

        // Helper: check 3x3 clearance around a cell and ensure no props occupy that area
        let width_usize = self.width as usize;
        let height_usize = self.height as usize;
        let has_clearance = |cx: usize, cy: usize| -> bool {
            if cx == 0 || cy == 0 || cx + 1 >= width_usize || cy + 1 >= height_usize {
                return false;
            }
            // cell walkability check
            for dy in (cy - 1)..=(cy + 1) {
                for dx in (cx - 1)..=(cx + 1) {
                    let idx = dy * width_usize + dx;
                    if self.cells.get(idx).copied().unwrap_or(0) == 0 {
                        return false;
                    }
                }
            }
            // ensure no prop sits inside the 3x3 area (avoid spawning on/next to props)
            for prop in &self.props {
                let px = prop.x as usize;
                let py = prop.y as usize;
                if (px >= cx.saturating_sub(1) && px <= cx + 1)
                    && (py >= cy.saturating_sub(1) && py <= cy + 1)
                {
                    return false;
                }
                // also check a small world-space distance to avoid edge collisions
                let cand_x = (cx as f32 + 0.5) * cell_size;
                let cand_z = (cy as f32 + 0.5) * cell_size;
                let prop_x = (prop.x as f32 + 0.5) * cell_size;
                let prop_z = (prop.y as f32 + 0.5) * cell_size;
                let dx = prop_x - cand_x;
                let dz = prop_z - cand_z;
                let min_dist = cell_size * 0.9; // slightly less than a cell
                if dx * dx + dz * dz <= min_dist * min_dist {
                    return false;
                }
            }
            true
        };

        // If we have at least two room centers, pick the pair with max distance
        if centers.len() >= 2 {
            let mut best: Option<(usize, usize, f32)> = None;
            for i in 0..centers.len() {
                for j in (i + 1)..centers.len() {
                    let dx = centers[i].0 - centers[j].0;
                    let dy = centers[i].1 - centers[j].1;
                    let d2 = dx * dx + dy * dy;
                    if best.as_ref().map_or(true, |b| d2 > b.2) {
                        best = Some((i, j, d2));
                    }
                }
            }
            if let Some((i, j, _)) = best {
                let a = centers[i];
                let b = centers[j];
                // Try to find a nearby clear spawn cell inside each room near the center.
                let find_spawn_near = |cx: f32, cy: f32| -> (f32, f32, f32) {
                    let rcx = cx.floor() as isize;
                    let rcy = cy.floor() as isize;
                    // search a small radius around the room center for a clear cell
                    for r in 0..=((self.width.max(self.height)) as isize) {
                        for oy in -r..=r {
                            for ox in -r..=r {
                                let sx = rcx + ox;
                                let sy = rcy + oy;
                                if sx < 1 || sy < 1 {
                                    continue;
                                }
                                let ux = sx as usize;
                                let uy = sy as usize;
                                if ux + 1 >= width_usize || uy + 1 >= height_usize {
                                    continue;
                                }
                                if has_clearance(ux, uy) {
                                    return (
                                        (ux as f32 + 0.5) * cell_size,
                                        0.0_f32,
                                        (uy as f32 + 0.5) * cell_size,
                                    );
                                }
                            }
                        }
                    }
                    // fallback to center cell world coords
                    ((cx + 0.5) * cell_size, 0.0_f32, (cy + 0.5) * cell_size)
                };

                let a_world = find_spawn_near(a.0, a.1);
                let b_world = find_spawn_near(b.0, b.1);
                return (a_world, b_world);
            }
        }

        // Fallback: choose corners but ensure they have clearance; prefer scanning inward
        let try_corner = |cx: usize, cy: usize| -> (f32, f32, f32) {
            // search inward from corner for any cell with clearance
            for ry in 1..(height_usize - 1) {
                for rx in 1..(width_usize - 1) {
                    let sx = if cx == 0 { rx } else { width_usize - 1 - rx };
                    let sy = if cy == 0 { ry } else { height_usize - 1 - ry };
                    if has_clearance(sx, sy) {
                        return (
                            (sx as f32 + 0.5) * cell_size,
                            0.0_f32,
                            (sy as f32 + 0.5) * cell_size,
                        );
                    }
                }
            }
            // final fallback: teleporter
            let (tx, ty, tz) = self.teleporter_world(cell_size);
            (tx, ty, tz)
        };

        let a_world = try_corner(0, 0);
        let b_world = try_corner(1, 1);
        (a_world, b_world)
    }

    /// Pick a random walkable cell that has a 3x3 clearance and convert to world coords.
    /// Falls back to the teleporter position if no suitable spot is found.
    /// Returns (x, y, z, used_fallback)
    /// used_fallback == true when the teleporter fallback was used.
    pub fn random_spawn_world(&self, cell_size: f32) -> (f32, f32, f32, bool) {
        if self.width <= 2 || self.height <= 2 {
            let (tx, ty, tz) = self.teleporter_world(cell_size);
            return (tx, ty, tz, true);
        }
        let mut rng = rand::thread_rng();
        let width_usize = self.width as usize;
        let height_usize = self.height as usize;

        // Helper: check 3x3 clearance around a cell and ensure no props occupy that area
        let has_clearance = |cx: usize, cy: usize| -> bool {
            if cx == 0 || cy == 0 || cx + 1 >= width_usize || cy + 1 >= height_usize {
                return false;
            }
            // cell walkability check
            for dy in (cy - 1)..=(cy + 1) {
                for dx in (cx - 1)..=(cx + 1) {
                    let idx = dy * width_usize + dx;
                    if self.cells.get(idx).copied().unwrap_or(0) == 0 {
                        return false;
                    }
                }
            }
            // ensure no prop sits inside the 3x3 area (avoid spawning on/next to props)
            for prop in &self.props {
                let px = prop.x as usize;
                let py = prop.y as usize;
                if (px >= cx.saturating_sub(1) && px <= cx + 1)
                    && (py >= cy.saturating_sub(1) && py <= cy + 1)
                {
                    return false;
                }
                // also check a small world-space distance to avoid edge collisions
                let cand_x = (cx as f32 + 0.5) * cell_size;
                let cand_z = (cy as f32 + 0.5) * cell_size;
                let prop_x = (prop.x as f32 + 0.5) * cell_size;
                let prop_z = (prop.y as f32 + 0.5) * cell_size;
                let dx = prop_x - cand_x;
                let dz = prop_z - cand_z;
                let min_dist = cell_size * 0.9; // slightly less than a cell
                if dx * dx + dz * dz <= min_dist * min_dist {
                    return false;
                }
            }
            true
        };

        // Try multiple random attempts first
        for _ in 0..200 {
            let rx = rng.gen_range(1..(self.width.saturating_sub(1)) as u8) as usize;
            let ry = rng.gen_range(1..(self.height.saturating_sub(1)) as u8) as usize;
            let idx = ry * width_usize + rx;
            if idx < self.cells.len() && self.cells[idx] == 1 && has_clearance(rx, ry) {
                return (
                    (rx as f32 + 0.5) * cell_size,
                    0.0,
                    (ry as f32 + 0.5) * cell_size,
                    false,
                );
            }
        }

        // Fallback: scan for any room cell with clearance
        for ry in 1..(height_usize - 1) {
            for rx in 1..(width_usize - 1) {
                let idx = ry * width_usize + rx;
                if self.cells[idx] == 1 && has_clearance(rx, ry) {
                    return (
                        (rx as f32 + 0.5) * cell_size,
                        0.0,
                        (ry as f32 + 0.5) * cell_size,
                        false,
                    );
                }
            }
        }

        // Final fallback: teleporter
        let (tx, ty, tz) = self.teleporter_world(cell_size);
        (tx, ty, tz, true)
    }

    /// Find the bomb site position for Search & Destroy mode.
    /// Picks the room closest to the center of the map and returns its center in world coords.
    pub fn bomb_site_position(&self, cell_size: f32) -> (f32, f32, f32) {
        let map_cx = self.width as f32 / 2.0;
        let map_cy = self.height as f32 / 2.0;

        if !self.rooms.is_empty() {
            // Find room whose center is closest to the map center
            let mut best_room_idx = 0;
            let mut best_dist = f32::MAX;
            for (i, r) in self.rooms.iter().enumerate() {
                let rx = r.x as f32 + r.w as f32 / 2.0;
                let ry = r.y as f32 + r.h as f32 / 2.0;
                let dx = rx - map_cx;
                let dy = ry - map_cy;
                let d = dx * dx + dy * dy;
                if d < best_dist {
                    best_dist = d;
                    best_room_idx = i;
                }
            }
            let r = &self.rooms[best_room_idx];
            let cx = r.x as f32 + r.w as f32 / 2.0;
            let cy = r.y as f32 + r.h as f32 / 2.0;
            (
                (cx + 0.5) * cell_size,
                0.0,
                (cy + 0.5) * cell_size,
            )
        } else {
            // Fallback: center of the map
            (
                (map_cx + 0.5) * cell_size,
                0.0,
                (map_cy + 0.5) * cell_size,
            )
        }
    }

    pub fn break_wall(&mut self, x: u8, y: u8) -> bool {
        let idx = y as usize * self.width as usize + x as usize;
        if idx >= self.cells.len() {
            return false;
        }
        if self.cells[idx] == 0 {
            self.cells[idx] = 1;
            return true;
        }
        false
    }
}
