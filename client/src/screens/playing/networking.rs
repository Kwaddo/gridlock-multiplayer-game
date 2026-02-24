use crate::net::protocols::{
    EVT_JUMP_START, EVT_LANDED, EVT_SLIDE_END, EVT_SLIDE_START, FLAG_ADS, FLAG_AIRBORNE,
    FLAG_CROUCHING, FLAG_PRONE, FLAG_SLIDING, FLAG_SPRINTING, MSG_HIT, MSG_POSITION,
    MSG_START_GAME, MSG_WALL_BROKEN, MSG_WORLD_CHANGE,
};
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
                let msg: Vec<u8> = vec![MSG_WORLD_CHANGE, target.as_u8()];
                let _ = s.send_to(&msg, server_addr);
            }

            // Lobby console: send start-game request to server (include selected gamemode)
            if self.pending_start_request {
                self.pending_start_request = false;
                let mode = self.pending_start_gamemode;
                let msg = vec![MSG_START_GAME, mode];
                let _ = s.send_to(&msg, server_addr);
            }

            // Send requested team selection (from lobby console)
            if let Some(team) = self.pending_set_team.take() {
                let msg = vec![crate::net::protocols::MSG_SET_TEAM, team];
                let _ = s.send_to(&msg, server_addr);
            }

            while let Some((x, y)) = self.pending_broken_walls.pop() {
                let msg = vec![MSG_WALL_BROKEN, x, y];
                let _ = s.send_to(&msg, server_addr);
            }

            while let Some((victim_id, damage, is_backstab)) = self.pending_hits.pop() {
                let mut msg = vec![MSG_HIT];
                msg.extend_from_slice(&victim_id.to_be_bytes());
                msg.extend_from_slice(&damage.to_be_bytes());
                // append active weapon name so server can include it in kill feed
                let weapon_name = self.weapons.active().stats().name.as_bytes();
                msg.push(weapon_name.len() as u8);
                msg.extend_from_slice(weapon_name);
                msg.push(if is_backstab { 1 } else { 0 });
                let _ = s.send_to(&msg, server_addr);
            }

            // Continuous state flags
            let mut state_flags = 0u8;
            if self.movement_state.is_sprinting {
                state_flags |= FLAG_SPRINTING;
            }
            if self.movement_state.is_crouching {
                state_flags |= FLAG_CROUCHING;
            }
            if self.weapons.active().ads_progress > 0.5 {
                state_flags |= FLAG_ADS;
            }
            if !self.movement_state.is_grounded {
                state_flags |= FLAG_AIRBORNE;
            }
            if self.movement_state.is_prone {
                state_flags |= FLAG_PRONE;
            }
            if self.movement_state.is_sliding {
                state_flags |= FLAG_SLIDING;
            }

            // One-frame event flags
            let mut event_flags = 0u8;
            if self.just_jumped {
                event_flags |= EVT_JUMP_START;
            }
            if self.just_landed {
                event_flags |= EVT_LANDED;
            }
            if self.movement_state.is_sliding && !self.prev_was_sliding {
                event_flags |= EVT_SLIDE_START;
            }
            if !self.movement_state.is_sliding && self.prev_was_sliding {
                event_flags |= EVT_SLIDE_END;
            }

            // pack position message into a small stack buffer to avoid per-frame heap allocations
            let mut buf = [0u8; 32];
            let mut p = 0usize;
            buf[p] = MSG_POSITION;
            p += 1;
            buf[p..p + 4].copy_from_slice(&self.position.x.to_be_bytes());
            p += 4;
            buf[p..p + 4].copy_from_slice(&self.position.y.to_be_bytes());
            p += 4;
            buf[p..p + 4].copy_from_slice(&self.position.z.to_be_bytes());
            p += 4;
            buf[p..p + 4].copy_from_slice(&self.yaw.to_be_bytes());
            p += 4;
            buf[p] = state_flags;
            p += 1;
            buf[p] = event_flags;
            p += 1;
            let _ = s.send_to(&buf[..p], server_addr);
            self.prev_was_sliding = self.movement_state.is_sliding;
        }
    }
}
