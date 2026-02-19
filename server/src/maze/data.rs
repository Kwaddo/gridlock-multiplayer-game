use std::net::{SocketAddr, UdpSocket};

use rand::Rng;

use crate::maze::generation::generate_maze;
use crate::maze::types::{MazeLight, Prop, Room};
use crate::protocols::MSG_MAZE;

pub struct MazeData {
    pub width: u8,
    pub height: u8,
    pub cells: Vec<u8>,
    pub seed: u64,
    pub rooms: Vec<Room>,
    pub props: Vec<Prop>,
    pub lights: Vec<MazeLight>,
    pub teleporter: (u8, u8),
}

impl MazeData {
    pub fn new(width: u8, height: u8) -> Self {
        let seed = rand::thread_rng().gen();
        let (cells, rooms, props, lights, teleporter) = generate_maze(width, height, seed);
        Self {
            width,
            height,
            cells,
            seed,
            rooms,
            props,
            lights,
            teleporter,
        }
    }

    pub fn send(&self, socket: &UdpSocket, addr: SocketAddr) {
        let header_len = 5;
        let rooms_len = 1 + self.rooms.len() * 4;
        let props_len = 1 + self.props.len() * 3;
        let lights_len = 1 + self.lights.len().min(255) * 3;
        let mut msg =
            Vec::with_capacity(header_len + rooms_len + props_len + lights_len + self.cells.len());

        msg.push(MSG_MAZE);
        msg.push(self.width);
        msg.push(self.height);
        msg.push(self.teleporter.0);
        msg.push(self.teleporter.1);

        msg.push(self.rooms.len() as u8);
        for room in &self.rooms {
            msg.push(room.x);
            msg.push(room.y);
            msg.push(room.w);
            msg.push(room.h);
        }

        msg.push(self.props.len() as u8);
        for prop in &self.props {
            msg.push(prop.kind.as_u8());
            msg.push(prop.x);
            msg.push(prop.y);
        }

        let light_count = self.lights.len().min(255);
        msg.push(light_count as u8);
        for light in self.lights.iter().take(light_count) {
            msg.push(light.x);
            msg.push(light.y);
            msg.push(light.variant);
        }

        msg.extend_from_slice(&self.cells);
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

    /// Pick a random walkable cell that has a 3x3 clearance and convert to world coords.
    /// Falls back to the teleporter position if no suitable spot is found.
    /// Returns (x, y, z, used_fallback)
    /// used_fallback == true when the teleporter fallback was used.
    pub fn random_spawn_world(&self, cell_size: f32) -> (f32, f32, f32, bool) {
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
                if (px >= cx.saturating_sub(1) && px <= cx + 1) && (py >= cy.saturating_sub(1) && py <= cy + 1) {
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
                return ((rx as f32 + 0.5) * cell_size, 0.0, (ry as f32 + 0.5) * cell_size, false);
            }
        }

        // Fallback: scan for any room cell with clearance
        for ry in 1..(height_usize - 1) {
            for rx in 1..(width_usize - 1) {
                let idx = ry * width_usize + rx;
                if self.cells[idx] == 1 && has_clearance(rx, ry) {
                    return ((rx as f32 + 0.5) * cell_size, 0.0, (ry as f32 + 0.5) * cell_size, false);
                }
            }
        }

        // Final fallback: teleporter
        let (tx, ty, tz) = self.teleporter_world(cell_size);
        (tx, ty, tz, true)
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
