use std::collections::HashSet;
use std::net::UdpSocket;

use crate::protocols::{
    MSG_POSITION, MSG_RESPAWN_REQUEST, MSG_WALL_BROKEN, MSG_WORLD_CHANGE,
};
use macroquad::prelude::*;

use super::PlayingState;

impl PlayingState {
    pub fn merge_other_players(&mut self, updates: Vec<crate::types::player::OtherPlayer>) {
        let mut seen: HashSet<u32> = HashSet::new();

        for upd in updates {
            seen.insert(upd.id);
            if let Some(p) = self.other_players.iter_mut().find(|p| p.id == upd.id) {
                let target = vec3(upd.x, upd.y, upd.z);
                let current = vec3(p.x, p.y, p.z);
                if (target - current).length() > 8.0 {
                    p.x = target.x;
                    p.y = target.y;
                    p.z = target.z;
                }
                p.target_x = upd.x;
                p.target_y = upd.y;
                p.target_z = upd.z;
                p.target_yaw = upd.yaw;
                p.target_pitch = upd.pitch;
                p.world = upd.world;
                p.username = upd.username;
                p.color = upd.color;
                p.hitboxes = upd.hitboxes;
                p.max_hp = upd.max_hp;
                p.hp = upd.hp;
            } else {
                let mut p = upd;
                p.target_x = p.x;
                p.target_y = p.y;
                p.target_z = p.z;
                p.target_yaw = p.yaw;
                p.target_pitch = p.pitch;
                self.other_players.push(p);
            }
        }

        self.other_players.retain(|p| seen.contains(&p.id));
    }

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
            msg.extend_from_slice(&self.pitch.to_be_bytes());
            let _ = s.send_to(&msg, server_addr);
        }
    }

    pub fn request_respawn(&self, socket: &UdpSocket, server_addr: &str) {
        let msg = vec![MSG_RESPAWN_REQUEST];
        let _ = socket.send_to(&msg, server_addr);
    }

    pub(super) fn update_other_players_smoothing(&mut self, delta: f32) {
        let pos_rate = 10.0;
        let rot_rate = 12.0;

        let pos_t = 1.0 - (-pos_rate * delta).exp();
        let rot_t = 1.0 - (-rot_rate * delta).exp();

        for p in &mut self.other_players {
            p.x += (p.target_x - p.x) * pos_t;
            p.y += (p.target_y - p.y) * pos_t;
            p.z += (p.target_z - p.z) * pos_t;

            p.yaw = Self::lerp_angle(p.yaw, p.target_yaw, rot_t);
            p.pitch = Self::lerp_angle(p.pitch, p.target_pitch, rot_t);

            if p.hit_timer > 0.0 {
                p.hit_timer = (p.hit_timer - delta).max(0.0);
            }
        }
    }

    pub(super) fn lerp_angle(current: f32, target: f32, t: f32) -> f32 {
        let mut diff = target - current;
        while diff > std::f32::consts::PI {
            diff -= std::f32::consts::TAU;
        }
        while diff < -std::f32::consts::PI {
            diff += std::f32::consts::TAU;
        }
        current + diff * t
    }
}
