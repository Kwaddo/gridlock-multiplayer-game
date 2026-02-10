use std::net::{SocketAddr, UdpSocket};

use rand::Rng;

use crate::maze::generation::generate_maze;
use crate::maze::types::{Prop, PropKind, Room};
use crate::protocols::MSG_MAZE;

pub struct MazeData {
    pub width: u8,
    pub height: u8,
    pub cells: Vec<u8>,
    pub seed: u64,
    pub rooms: Vec<Room>,
    pub props: Vec<Prop>,
    pub teleporter: (u8, u8),
}

impl MazeData {
    pub fn new(width: u8, height: u8) -> Self {
        let seed = rand::thread_rng().gen();
        let (cells, rooms, props, teleporter) = generate_maze(width, height, seed);
        Self {
            width,
            height,
            cells,
            seed,
            rooms,
            props,
            teleporter,
        }
    }

    pub fn send(&self, socket: &UdpSocket, addr: SocketAddr) {
        let header_len = 5;
        let rooms_len = 1 + self.rooms.len() * 4;
        let props_len = 1 + self.props.len() * 3;
        let mut msg = Vec::with_capacity(header_len + rooms_len + props_len + self.cells.len());

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
}
