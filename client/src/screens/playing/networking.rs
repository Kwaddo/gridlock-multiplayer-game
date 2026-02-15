use crate::protocols::{MSG_POSITION, MSG_WALL_BROKEN, MSG_WORLD_CHANGE};

use std::net::UdpSocket;

use super::PlayingState;

impl PlayingState {
    pub fn send_network(&mut self, socket: &Option<UdpSocket>, server_addr: &str) {
        if let Some(ref s) = socket {
            if self.pending_respawn {
                self.pending_respawn = false;
                self.request_respawn(s, server_addr);
            }

            if let Some(target) = self.pending_world_change.take() {
                let msg = vec![MSG_WORLD_CHANGE, target.as_u8()];
                let _ = s.send_to(&msg, server_addr);
            }

            while let Some((x, y)) = self.pending_broken_walls.pop() {
                let msg = vec![MSG_WALL_BROKEN, x, y];
                let _ = s.send_to(&msg, server_addr);
            }

            let mut msg = vec![MSG_POSITION];
            msg.extend_from_slice(&self.position.x.to_be_bytes());
            msg.extend_from_slice(&self.position.y.to_be_bytes());
            msg.extend_from_slice(&self.position.z.to_be_bytes());
            msg.extend_from_slice(&self.yaw.to_be_bytes());
            let _ = s.send_to(&msg, server_addr);
        }
    }
}
