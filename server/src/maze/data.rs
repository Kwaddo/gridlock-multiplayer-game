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
