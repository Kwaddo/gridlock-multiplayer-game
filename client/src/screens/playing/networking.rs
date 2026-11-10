use crate::net::protocols::{
    ClientMsg, EVT_JUMP_START, EVT_LANDED, EVT_SLIDE_END, EVT_SLIDE_START, FLAG_ADS,
    FLAG_AIRBORNE, FLAG_CROUCHING, FLAG_PRONE, FLAG_SLIDING, FLAG_SPRINTING, LobbySettings,
    MODE_GUN_GAME,
};
use transport::Link;

use super::PlayingState;

const POSITION_SEND_INTERVAL: platform::Duration = platform::Duration::from_millis(33);
/// How often to repeat a bomb pickup request while standing on the dropped bomb.
const BOMB_PICKUP_INTERVAL: platform::Duration = platform::Duration::from_millis(200);

impl PlayingState {
    pub fn send_network(
        &mut self,
        link: Option<&mut dyn Link>,
        config: &crate::core::GameConfig,
    ) {
        if let Some(s) = link {
            if self.pending_respawn {
                self.pending_respawn = false;
                self.request_respawn(s);
            }

            if let Some(target) = self.pending_world_change.take() {
                let msg = ClientMsg::WorldChange { target: target.as_u8() }.encode();
                s.send(&msg);
            }

            if std::mem::take(&mut self.pending_end_match) {
                s.send(&ClientMsg::EndMatch.encode());
            }

            for text in self.chat.take_outgoing() {
                s.send(&ClientMsg::Chat { text }.encode());
            }

            // Lobby console: send start-game request to server (include selected gamemode)
            if self.pending_start_request {
                self.pending_start_request = false;
                let settings = self.lobby_settings_now(config);
                // auto-fill: Red/Blue bot counts in team modes, a total in
                // Gun Game (0 = no bots); always sent so the server parses easily
                let blue_bots = (settings.mode != MODE_GUN_GAME).then_some(settings.blue_bots);
                let msg = ClientMsg::StartGame {
                    mode: Some(settings.mode),
                    duration: Some(settings.duration),
                    autofill: Some((settings.red_bots, settings.diff_mask)),
                    blue_bots,
                }
                .encode();
                s.send(&msg);
            }

            // The host's lobby console changed: tell everyone what's next.
            if let Some(settings) = self.pending_lobby_settings.take() {
                s.send(&ClientMsg::LobbySettings(settings).encode());
            }

            // Send requested team selection (from lobby console)
            if let Some(team) = self.pending_set_team.take() {
                let msg = ClientMsg::SetTeam { team }.encode();
                s.send(&msg);
            }

            // Public shooting-range button: progresses phases with one start action.
            if self.pending_range_start {
                self.pending_range_start = false;
                let started_phase = self.range_phase;

                // Always reset range bots before loading the next phase.
                s.send(&ClientMsg::BotDespawnAll.encode());

                // Public range button should use server preset paths/behaviors.
                // 255 means "no override" (server maps >2 => None).
                let (behavior, level) = match started_phase {
                    1 => (255u8, 1u8),
                    2 => (255u8, 2u8),
                    3 => (255u8, 3u8),
                    _ => (255u8, 1u8),
                };
                s.send(&ClientMsg::BotSpawn { behavior, level }.encode());

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
                let msg = ClientMsg::WallBroken { x, y }.encode();
                s.send(&msg);
            }

            while let Some((victim_id, damage, is_backstab, is_headshot)) = self.pending_hits.pop() {
                // include active weapon name so server can include it in kill feed
                let msg = ClientMsg::Hit {
                    victim_id,
                    damage,
                    weapon: Some(self.weapons.active().stats().name.to_string()),
                    backstab: Some(is_backstab),
                    // so the kill feed can say headshot and everyone picks the headshot death, before this it was never sent
                    headshot: Some(is_headshot),
                }
                .encode();
                s.send(&msg);
            }

            // S&D interactions: bomb pickup, plant, defuse
            if self.match_mode == Some(crate::net::protocols::MODE_SEARCH_AND_DESTROY) {
                // Auto bomb pickup: if bomb is dropped and we're close, try to pick it up.
                // The server only lets living attackers pick it up, so only they ask,
                // and at most every BOMB_PICKUP_INTERVAL rather than every frame.
                let is_attacker =
                    self.snd_attacking_team != 0 && self.local_team == self.snd_attacking_team;
                if !self.is_dead && is_attacker {
                    if let Some((bx, _by, bz)) = self.snd_bomb_dropped {
                        let dx = self.position.x - bx;
                        let dz = self.position.z - bz;
                        let due = self
                            .snd_last_pickup_sent
                            .map_or(true, |t| t.elapsed() >= BOMB_PICKUP_INTERVAL);
                        if (dx * dx + dz * dz).sqrt() <= 2.5 && due {
                            s.send(&ClientMsg::SndBombPickup.encode());
                            self.snd_last_pickup_sent = Some(platform::Instant::now());
                        }
                    }
                }

                // Plant/defuse: hold E key near bomb site
                let near_site = {
                    let dx = self.position.x - self.snd_bomb_site.0;
                    let dz = self.position.z - self.snd_bomb_site.2;
                    (dx * dx + dz * dz).sqrt() <= 3.5
                };

                let e_held = !self.chat.is_open()
                    && macroquad::prelude::is_key_down(macroquad::prelude::KeyCode::E);

                // Planting: we have the bomb, near site, hold E
                if self.snd_bomb_carrier_id != 0
                    && !self.snd_bomb_planted
                    && near_site
                    && e_held
                    && !self.is_dead
                {
                    if !self.snd_is_planting {
                        s.send(&ClientMsg::SndPlantStart.encode());
                        self.snd_is_planting = true;
                    }
                } else if self.snd_is_planting {
                    s.send(&ClientMsg::SndPlantStop.encode());
                    self.snd_is_planting = false;
                }

                // Defusing: bomb is planted, near site, hold E
                if self.snd_bomb_planted && near_site && e_held && !self.is_dead {
                    if !self.snd_is_defusing {
                        s.send(&ClientMsg::SndDefuseStart.encode());
                        self.snd_is_defusing = true;
                    }
                } else if self.snd_is_defusing {
                    s.send(&ClientMsg::SndDefuseStop.encode());
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
                let msg = ClientMsg::WeaponChange { name: current_weapon_name.to_string() }.encode();
                s.send(&msg);
                self.last_sent_weapon = current_weapon_name.to_string();
            }

            // Position goes out at most 30 times a second (the server sends
            // the world back at that rate too), but at once for a movement
            // event or a change of stance, so those are never delayed.
            let due = self
                .last_position_sent
                .map_or(true, |t| t.elapsed() >= POSITION_SEND_INTERVAL);
            if due || event_flags != 0 || state_flags != self.last_state_flags_sent {
                let msg = ClientMsg::Position {
                    position: (self.position.x, self.position.y, self.position.z),
                    yaw: self.yaw,
                    state_flags: Some(state_flags),
                    event_flags: Some(event_flags),
                    // where you look up or down, so others see it, before this it was never sent
                    pitch: Some(self.pitch),
                }
                .encode();
                s.send(&msg);
                self.last_position_sent = Some(platform::Instant::now());
                self.last_state_flags_sent = state_flags;
            }
            self.prev_was_sliding = self.movement_state.is_sliding;
        }
    }
}

impl PlayingState {
    /// The match this client's lobby console is set up for, as sent with
    /// `StartGame` and shared with the other players as `LobbySettings`.
    pub(super) fn lobby_settings_now(&self, config: &crate::core::GameConfig) -> LobbySettings {
        let mode = self.pending_start_gamemode;
        // duration code: 0 = mode default, 1..=65534 = seconds, 65535 = unlimited
        let duration = if config.hide_match_timer {
            u16::MAX
        } else {
            config.match_time_override_secs.map_or(0, |secs| secs.min(u16::MAX as u32) as u16)
        };
        // difficulty mask: bit0=Recruit, bit1=Easy, bit2=Normal, bit3=Hard, bit4=Nightmare
        let diff_mask = [
            self.autofill_diff_recruit,
            self.autofill_diff_easy,
            self.autofill_diff_normal,
            self.autofill_diff_hard,
            self.autofill_diff_nightmare,
        ]
        .iter()
        .enumerate()
        .fold(0u8, |mask, (bit, on)| if *on { mask | 1 << bit } else { mask });
        let (red_bots, blue_bots) = if mode == MODE_GUN_GAME {
            (self.autofill_ffa_bots, 0)
        } else {
            (self.autofill_red_bots, self.autofill_blue_bots)
        };
        LobbySettings { mode, duration, red_bots, blue_bots, diff_mask }
    }
}
