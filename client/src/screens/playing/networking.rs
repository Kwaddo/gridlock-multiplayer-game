use crate::net::protocols::{
    EVT_JUMP_START, EVT_LANDED, EVT_SLIDE_END, EVT_SLIDE_START, FLAG_ADS, FLAG_AIRBORNE,
    FLAG_CROUCHING, FLAG_PRONE, FLAG_SLIDING, FLAG_SPRINTING, MSG_HIT, MSG_POSITION,
    MSG_SND_BOMB_PICKUP, MSG_SND_DEFUSE_START, MSG_SND_DEFUSE_STOP, MSG_SND_PLANT_START,
    MSG_SND_PLANT_STOP, MSG_START_GAME, MSG_WALL_BROKEN, MSG_WORLD_CHANGE,
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

            while let Some((victim_id, damage)) = self.pending_hits.pop() {
                let mut msg = vec![MSG_HIT];
                msg.extend_from_slice(&victim_id.to_be_bytes());
                msg.extend_from_slice(&damage.to_be_bytes());
                // append active weapon name so server can include it in kill feed
                let weapon_name = self.weapons.active().stats().name.as_bytes();
                msg.push(weapon_name.len() as u8);
                msg.extend_from_slice(weapon_name);
                let _ = s.send_to(&msg, server_addr);
            }

            // S&D interactions: bomb pickup, plant, defuse
            if self.match_mode == Some(crate::net::protocols::MODE_SEARCH_AND_DESTROY) {
                // Auto bomb pickup: if bomb is dropped and we're close, try to pick it up
                if let Some((bx, _by, bz)) = self.snd_bomb_dropped {
                    let dx = self.position.x - bx;
                    let dz = self.position.z - bz;
                    if (dx * dx + dz * dz).sqrt() <= 2.5 {
                        let _ = s.send_to(&[MSG_SND_BOMB_PICKUP], server_addr);
                    }
                }

                // Plant/defuse: hold E key near bomb site
                let near_site = {
                    let dx = self.position.x - self.snd_bomb_site.0;
                    let dz = self.position.z - self.snd_bomb_site.2;
                    (dx * dx + dz * dz).sqrt() <= 3.5
                };

                let e_held = macroquad::prelude::is_key_down(macroquad::prelude::KeyCode::E);

                // Planting: we have the bomb, near site, hold E
                if self.snd_bomb_carrier_id != 0
                    && !self.snd_bomb_planted
                    && near_site
                    && e_held
                    && !self.is_dead
                {
                    if !self.snd_is_planting {
                        let _ = s.send_to(&[MSG_SND_PLANT_START], server_addr);
                        self.snd_is_planting = true;
                    }
                } else if self.snd_is_planting {
                    let _ = s.send_to(&[MSG_SND_PLANT_STOP], server_addr);
                    self.snd_is_planting = false;
                }

                // Defusing: bomb is planted, near site, hold E
                if self.snd_bomb_planted && near_site && e_held && !self.is_dead {
                    if !self.snd_is_defusing {
                        let _ = s.send_to(&[MSG_SND_DEFUSE_START], server_addr);
                        self.snd_is_defusing = true;
                    }
                } else if self.snd_is_defusing {
                    let _ = s.send_to(&[MSG_SND_DEFUSE_STOP], server_addr);
                    self.snd_is_defusing = false;
                }
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
            // Placeholders for future states (currently not implemented)
            if false {
                state_flags |= FLAG_PRONE;
            }
            if false {
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
            // Placeholders for future events
            if false {
                event_flags |= EVT_SLIDE_START;
            }
            if false {
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
        }
    }
}
