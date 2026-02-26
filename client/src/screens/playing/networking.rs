use crate::net::protocols::{
    EVT_JUMP_START, EVT_LANDED, EVT_SLIDE_END, EVT_SLIDE_START, FLAG_ADS, FLAG_AIRBORNE,
    FLAG_CROUCHING, FLAG_PRONE, FLAG_SLIDING, FLAG_SPRINTING, MSG_BOT_DESPAWN_ALL, MSG_BOT_SPAWN,
    MSG_HIT, MSG_POSITION, MSG_SND_BOMB_PICKUP, MSG_SND_DEFUSE_START, MSG_SND_DEFUSE_STOP,
    MSG_SND_PLANT_START, MSG_SND_PLANT_STOP, MSG_START_GAME, MSG_WALL_BROKEN, MSG_WEAPON_CHANGE,
    MSG_WORLD_CHANGE,
};
use std::net::UdpSocket;

use super::PlayingState;

impl PlayingState {
    pub fn send_network(
        &mut self,
        socket: &Option<UdpSocket>,
        server_addr: &str,
        config: &crate::core::GameConfig,
    ) {
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
                // compute duration code: 0=default, 1..=65534=seconds, 65535=infinite
                let dur_code: u16 = if config.hide_match_timer {
                    u16::MAX
                } else if let Some(secs) = config.match_time_override_secs {
                    // if user picked a value explicitly; treat 0 as default
                    if secs == 0 {
                        0
                    } else {
                        secs.min(u16::MAX as u32) as u16
                    }
                } else {
                    0
                };
                let mut msg = vec![MSG_START_GAME, mode];
                // always append two bytes for the duration code so server can parse easily
                msg.push((dur_code >> 8) as u8);
                msg.push((dur_code & 0xFF) as u8);
                let _ = s.send_to(&msg, server_addr);
            }

            // Send requested team selection (from lobby console)
            if let Some(team) = self.pending_set_team.take() {
                let msg = vec![crate::net::protocols::MSG_SET_TEAM, team];
                let _ = s.send_to(&msg, server_addr);
            }

            // Public shooting-range button: progresses phases with one start action.
            if self.pending_range_start {
                self.pending_range_start = false;
                let started_phase = self.range_phase;

                // Always reset range bots before loading the next phase.
                let _ = s.send_to(&[MSG_BOT_DESPAWN_ALL], server_addr);

                // Public range button should use server preset paths/behaviors.
                // 255 means "no override" (server maps >2 => None).
                let (behavior, level) = match started_phase {
                    1 => (255u8, 1u8),
                    2 => (255u8, 2u8),
                    3 => (255u8, 3u8),
                    _ => (255u8, 1u8),
                };
                let _ = s.send_to(&[MSG_BOT_SPAWN, behavior, level], server_addr);

                self.range_phase = if self.range_phase >= 3 {
                    1
                } else {
                    self.range_phase + 1
                };
                self.lobby_console_message =
                    Some(format!("Shooting Range: Phase {}", started_phase));
                self.lobby_console_message_timer = 3.0;
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

            // S&D interactions: bomb pickup, plant, defuse
            if self.match_mode == Some(crate::net::protocols::MODE_SEARCH_AND_DESTROY) {
                // Auto bomb pickup: if bomb is dropped and we're close, try to pick it up
                if !self.is_dead {
                    if let Some((bx, _by, bz)) = self.snd_bomb_dropped {
                        let dx = self.position.x - bx;
                        let dz = self.position.z - bz;
                        if (dx * dx + dz * dz).sqrt() <= 2.5 {
                            let _ = s.send_to(&[MSG_SND_BOMB_PICKUP], server_addr);
                        }
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

            // send weapon change if it differs from last sent
            let current_weapon_name = self.weapons.active().stats().name;
            if current_weapon_name != self.last_sent_weapon {
                let bytes = current_weapon_name.as_bytes();
                let mut msg = Vec::with_capacity(2 + bytes.len());
                msg.push(MSG_WEAPON_CHANGE);
                msg.push(bytes.len() as u8);
                msg.extend_from_slice(bytes);
                let _ = s.send_to(&msg, server_addr);
                self.last_sent_weapon = current_weapon_name.to_string();
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
