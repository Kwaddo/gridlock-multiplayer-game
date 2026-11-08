//! Inbound packet dispatch: one `ClientMsg` to the handler that owns it.

use rand::Rng;
use std::time::Duration;
use platform::Instant;

use super::{broadcast, game, handlers, Server};
use crate::bot::ai::BotState;
use crate::net::PeerId;
use crate::player::Player;
use crate::player::WorldId;
use crate::protocols::*;

impl Server {
    /// Process one inbound packet (`[msg_type][payload]`) from `src_addr`.
    pub fn handle_packet(&mut self, src_addr: PeerId, packet: &[u8]) {
        // a client of another version that was turned away gets no further
        if self.handle_rejected(src_addr, packet) {
            return;
        }
        let Some(msg) = ClientMsg::decode(packet) else {
            if let Some(msg_type) = packet.first() {
                log!("Unknown or malformed message type: {}", msg_type);
            }
            return;
        };

        match msg {
            ClientMsg::Join { username, version } => {
                if version != Some(PROTOCOL_VERSION) {
                    self.reject_join(src_addr, &username, version);
                    return;
                }
                self.rejected.remove(&src_addr);
                handlers::handle_join(
                    &self.outbox,
                    &mut self.players,
                    self.maze.as_ref(),
                    self.match_running,
                    src_addr,
                    &username,
                );
                // the newcomer needs it now, not at the next periodic resend
                self.send_match_status(true);
                if game::balance_lobby_teams(&mut self.players) {
                    broadcast::broadcast_player_list(&self.outbox, &self.players);
                }
                let settings = ServerMsg::LobbySettings(self.lobby_settings).encode();
                let _ = self.outbox.send_to(&settings, src_addr);
            }
            ClientMsg::LobbySettings(settings) => self.handle_lobby_settings(src_addr, settings),
            ClientMsg::Disconnect => {
                handlers::handle_disconnect(&self.outbox, &mut self.players, src_addr)
            }
            ClientMsg::Position { position, yaw, state_flags, event_flags } => {
                handlers::handle_position(
                    &self.outbox,
                    &mut self.players,
                    src_addr,
                    position,
                    yaw,
                    state_flags,
                    event_flags,
                );
            },

            // client-requested world change (teleporter)
            ClientMsg::WorldChange { target } => {
                let was_in_maze =
                    self.players.get(&src_addr).is_some_and(|p| p.world == WorldId::Maze);
                handlers::handle_world_change(
                    &self.outbox,
                    self.maze.as_ref(),
                    &mut self.players,
                    src_addr,
                    target,
                );
                if was_in_maze && target == WorldId::Lobby.as_u8() {
                    if let Some(p) = self.players.get(&src_addr) {
                        let text = format!("{} left the match", p.username);
                        handlers::broadcast_notice(&self.outbox, &self.players, &text);
                        // the maze view no longer includes them
                        broadcast::broadcast_state(&self.outbox, &self.players);
                        broadcast::broadcast_player_list(&self.outbox, &self.players);
                    }
                }
            },
            ClientMsg::WeaponChange { name } => {
                // client tells us which weapon they're now holding; update
                // player state and broadcast to everyone so world models
                // stay in sync.
                if let Some(p) = self.players.get_mut(&src_addr) {
                    p.current_weapon = name;
                    self.sync_player_info();
                }
            },
            // client requests to start a match from the lobby (or join the running one)
            ClientMsg::StartGame { mode, duration, autofill, blue_bots } => {
                self.handle_start_request(src_addr, mode, duration, autofill, blue_bots)
            }
            ClientMsg::WallBroken { x, y } => {
                if let Some(ref mut m) = self.maze {
                    handlers::handle_wall_broken(&self.outbox, m, &self.players, src_addr, x, y);
                }
            }
            ClientMsg::Discover => handlers::handle_discover(&self.outbox, &self.players, &self.name, src_addr),
            ClientMsg::Hit { victim_id, damage, weapon, backstab } => handlers::handle_hit(
                &self.outbox,
                &mut self.players,
                src_addr,
                victim_id,
                damage,
                weapon,
                backstab,
                &mut self.maze,
                &mut self.match_running,
                &mut self.match_start_time,
                &mut self.match_end_pending,
                self.match_mode,
                &mut self.snd_state,
                self.bot_manager.bot_states_mut(),
                &mut self.team_score,
            ),
            ClientMsg::RespawnRequest => {
                handlers::handle_respawn_request(
                    &self.outbox,
                    self.maze.as_ref(),
                    &mut self.players,
                    src_addr,
                    self.match_mode,
                    &self.snd_state,
                );
            }
            ClientMsg::SetTeam { team } => {
                handlers::handle_set_team(&self.outbox, &mut self.players, src_addr, team);
            }
            ClientMsg::SndBombPickup => {
                if let Some(ref mut snd) = self.snd_state {
                    snd.try_pickup_bomb(src_addr, &self.players);
                }
            }
            ClientMsg::SndPlantStart => {
                if let Some(ref mut snd) = self.snd_state {
                    snd.try_start_plant(src_addr, &self.players);
                }
            }
            ClientMsg::SndPlantStop => {
                if let Some(ref mut snd) = self.snd_state {
                    snd.stop_plant(src_addr);
                }
            }
            ClientMsg::SndDefuseStart => {
                if let Some(ref mut snd) = self.snd_state {
                    snd.try_start_defuse(src_addr, &self.players);
                }
            }
            ClientMsg::SndDefuseStop => {
                if let Some(ref mut snd) = self.snd_state {
                    snd.stop_defuse(src_addr);
                }
            }
            ClientMsg::BotSpawn { behavior: behavior_code, level } => {
                let mut rng = rand::thread_rng();
                let behavior_override = if behavior_code <= 2 {
                    Some(behavior_code)
                } else {
                    None
                };
                let preset =
                    crate::bot::shooting_range::level_config(level, behavior_override);
                let range_deadline = preset
                    .time_limit_secs
                    .map(|secs| Instant::now() + Duration::from_secs(secs));
                self.bot_manager.set_range_challenge_deadline(range_deadline);
                let base_bot_count = self.players.values().filter(|p| p.is_bot).count();
                for (i, bot_cfg) in preset.bots.iter().enumerate() {
                    let bot_index = base_bot_count + i;
                    let bot_id = 9000u32 + (bot_index as u32);
                    let bot_name = format!("Bot{}", bot_index + 1);
                    let shooting_cfg =
                        bot_cfg.shooting.unwrap_or(preset.default_shooting);
                    let weapon = bot_cfg.weapon.unwrap_or(preset.default_weapon);
                    let hp = bot_cfg.health.unwrap_or(preset.default_health);
                    let can_shoot =
                        bot_cfg.can_shoot.unwrap_or(preset.default_can_shoot);
                    let bot_player = Player {
                        id: bot_id,
                        username: bot_name,
                        position: bot_cfg.position,
                        yaw: std::f32::consts::PI,
                        pitch: 0.0,
                        health: hp,
                        max_health: hp,
                        is_alive: true,
                        kills: 0,
                        deaths: 0,
                        gun_level: 1,
                        last_activity: Instant::now(),
                        death_time: None,
                        world: WorldId::Lobby,
                        post_teleport_freeze_until: None,
                        is_sprinting: false,
                        is_ads: false,
                        is_crouched: false,
                        is_airborne: false,
                        is_prone: false,
                        is_sliding: false,
                        evt_jump_start: false,
                        evt_landed: false,
                        team: bot_cfg.team,
                        current_weapon: weapon.to_string(),
                        is_bot: true,
                        is_silent: false,
                        auto_respawn: false,
                        regen_resume_at: None,

                    };

                    let bot_state = BotState {
                        movement: crate::physics::BotMovementState {
                            velocity: glam::Vec3::ZERO,
                            is_grounded: true,
                            height: 1.6,
                        },
                        behavior: bot_cfg.behavior.clone(),
                        can_shoot,
                        shooting: crate::bot::shooting::BotShootingState {
                            // Start each bot with slight offset so they don't sync-fire.
                            fire_rate: rng
                                .gen_range(0.0f32..shooting_cfg.fire_rate.max(0.01)),
                            has_target: false,
                            reaction_remaining: shooting_cfg.reaction_time
                                + rng.gen_range(0.0f32..0.35),
                        },
                        shooting_config: shooting_cfg,
                        target_id: None,
                        reaction_timer: 0.0,
                        // new fields required by BotState
                        personality: crate::bot::personality::BotPersonality {
                            name: "range", // generic
                            archetype:
                                crate::bot::personality::BotArchetype::AllRounder,
                            weapon,
                            range_min: 0.0,
                            range_max: 100.0,
                            aggression: 0.5,
                            flank_weight: 0.5,
                            hold_angle_weight: 0.5,
                            push_weight: 0.5,
                            retreat_health_threshold: 0.5,
                            peek_time: 1.0,
                            reposition_interval: 3.0,
                            noise_investigation_weight: 0.5,
                            base_aim_offset: 0.0,
                            base_reaction_time: shooting_cfg.reaction_time,
                            base_fov_degrees: 90.0,
                            base_repath_interval: 2.0,
                            base_turn_rate: 4.0,
                        },
                        difficulty: crate::bot::personality::BotDifficulty::Normal,
                        archetype: crate::bot::personality::BotArchetype::AllRounder,
                        personality_id: None,
                        last_known_enemy_pos: None,
                        behavior_decision_timer: 0.0,
                        stuck_timer: 0.0,
                        idle_look_timer: 0.0,
                        current_yaw: 0.0,
                        current_pitch: 0.0,
                        tactics: Default::default(),
                    };

                    self.bot_manager.spawn_bot(&mut self.players, bot_player, bot_state);
                }
                broadcast::broadcast_state(&self.outbox, &self.players);
                broadcast::broadcast_player_list(&self.outbox, &self.players);
            }

            ClientMsg::BotDespawnAll => {
                self.bot_manager.despawn_all(&mut self.players);
                broadcast::broadcast_state(&self.outbox, &self.players);
                broadcast::broadcast_player_list(&self.outbox, &self.players);
            }

            ClientMsg::Chat { text } => handlers::handle_chat(
                &self.outbox,
                &self.players,
                &mut self.last_chat,
                src_addr,
                &text,
            ),
            ClientMsg::EndMatch => self.handle_end_match(src_addr),

        }
    }
}
