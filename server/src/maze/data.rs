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

    /// Determine two deterministic team spawn positions (world coords): the
    /// two cells with a clear 3x3 area around them (open, no props) that are
    /// farthest apart. Only rooms have such areas, so the teams start in the
    /// two rooms farthest from each other. Falls back to opposite maze
    /// corners if there are fewer than two.
    pub fn team_spawn_positions(&self, cell_size: f32) -> ((f32, f32, f32), (f32, f32, f32)) {
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

        // The pair of `cells` farthest apart, with its squared distance in cells.
        let farthest_pair = |cells: &[(usize, usize)]| {
            let mut best: Option<((usize, usize), (usize, usize), usize)> = None;
            for (i, &a) in cells.iter().enumerate() {
                for &b in &cells[i + 1..] {
                    let d2 = a.0.abs_diff(b.0).pow(2) + a.1.abs_diff(b.1).pow(2);
                    if best.map_or(true, |(_, _, best_d2)| d2 > best_d2) {
                        best = Some((a, b, d2));
                    }
                }
            }
            best
        };
        let cells_where = |keep: &dyn Fn(usize, usize) -> bool| -> Vec<(usize, usize)> {
            (1..height_usize.saturating_sub(1))
                .flat_map(|y| (1..width_usize.saturating_sub(1)).map(move |x| (x, y)))
                .filter(|&(x, y)| keep(x, y))
                .collect()
        };

        // Rooms first: open 3x3 areas. Props can fill a small maze's rooms,
        // leaving clear spots in one room only; then any open cell with no
        // prop next to it will do, so the teams still start far apart.
        let min_apart = width_usize.min(height_usize) * 2 / 5;
        let mut best = farthest_pair(&cells_where(&|x, y| has_clearance(x, y)));
        if best.map_or(true, |(_, _, d2)| d2 < min_apart * min_apart) {
            let prop_free = |x: usize, y: usize| {
                self.cells[y * width_usize + x] == 1
                    && !self.props.iter().any(|p| (p.x as usize).abs_diff(x) <= 1 && (p.y as usize).abs_diff(y) <= 1)
            };
            let wider = farthest_pair(&cells_where(&prop_free));
            if wider.map_or(false, |(_, _, d2)| best.map_or(true, |(_, _, b2)| d2 > b2)) {
                best = wider;
            }
        }
        if let Some((a, b, _)) = best {
            let world = |(x, y): (usize, usize)| {
                ((x as f32 + 0.5) * cell_size, 0.0_f32, (y as f32 + 0.5) * cell_size)
            };
            return (world(a), world(b));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::game::{maze_side_for_players, MAZE_CELL_SIZE};

    fn open_at(maze: &MazeData, pos: (f32, f32, f32)) -> bool {
        let x = (pos.0 / MAZE_CELL_SIZE).floor() as usize;
        let y = (pos.2 / MAZE_CELL_SIZE).floor() as usize;
        x < maze.width as usize && y < maze.height as usize && maze.cells[y * maze.width as usize + x] == 1
    }

    #[test]
    fn spawns_and_bomb_site_work_at_every_size() {
        for players in 1..=16 {
            let side = maze_side_for_players(players);
            for _ in 0..10 {
                let maze = MazeData::new(side, side);
                let (red, blue) = maze.team_spawn_positions(MAZE_CELL_SIZE);
                assert!(open_at(&maze, red) && open_at(&maze, blue), "side {side}: team spawn in a wall");
                let apart = ((red.0 - blue.0).powi(2) + (red.2 - blue.2).powi(2)).sqrt();
                let min_apart = (side as usize * 2 / 5) as f32 * MAZE_CELL_SIZE;
                assert!(apart >= min_apart, "side {side}: team spawns only {apart} m apart");
                assert!(open_at(&maze, maze.bomb_site_position(MAZE_CELL_SIZE)), "side {side}: bomb site in a wall");
                let (x, y, z, _) = maze.random_spawn_world(MAZE_CELL_SIZE);
                assert!(open_at(&maze, (x, y, z)), "side {side}: random spawn in a wall");
                assert!(open_at(&maze, maze.teleporter_world(MAZE_CELL_SIZE)), "side {side}: teleporter in a wall");
            }
        }
    }
}
