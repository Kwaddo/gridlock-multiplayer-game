use rand::Rng;
use std::collections::HashMap;
use std::time::Duration;
use platform::Instant;

pub mod broadcast;
pub mod game;
pub mod handlers;
pub mod snd;
pub mod utils;

use crate::bot::ai::BotState;
use crate::bot::manager::BotManager;
use crate::maze::MazeData;
use crate::net::{Outbox, PeerId};
use crate::player::Player;
use crate::player::WorldId;
use crate::protocols::*;

/// Authoritative game state. Previously the locals of a blocking `run()` loop;
/// now driven by whichever host owns the transport.
pub struct Server {
    outbox: Outbox,
    players: HashMap<PeerId, Player>,
    bot_manager: BotManager,
    // cached assets manifest, sent to each client on join
    // shown to clients browsing for games
    name: String,
    // Do not generate a full gameplay maze at server start — create per-match instead.
    maze: Option<MazeData>, // Some(...) when a match is running
    match_running: bool,
    match_start_time: Option<Instant>,
    // current running match length (None=infinite); initially use the default
    match_duration: Option<Duration>,
    // warmup / countdown before match actually begins
    match_warmup_end: Option<Instant>,
    // scheduled match end (winner, delayed return-to-lobby)
    match_end_pending: Option<(Instant, u32, String)>,
    // last sent authoritative match timer (to avoid spamming identical values)
    last_match_time_sent: Option<u16>,
    // last sent warmup countdown (MSG_MATCH_COUNTDOWN)
    last_warmup_sent: Option<u8>,
    // last sent match-end countdown (MSG_MATCH_END countdown byte)
    last_end_countdown_sent: Option<u8>,
    // current match mode (defaults to Team Deathmatch)
    match_mode: u8,
    // how many players entered the maze during this match (for last-man-standing detection)
    match_total_maze_entries: usize,
    // Search & Destroy state (only Some when S&D mode is active)
    snd_state: Option<snd::SndState>,
    // Auto-fill bot settings (set by the client that starts the match)
    autofill_bots_per_team: u8, // 0=disabled, 1-8=per team
    autofill_diff_mask: u8,     // all difficulties enabled by default
    // Track last tick time for delta-time calculation
    last_tick: Instant,
    // Regen fires at most once per second to avoid per-packet health spikes
    last_regen_tick: Instant,
    last_state_broadcast: Option<Instant>,
    /// Player count at the last state broadcast: a join or leave sends at once.
    last_broadcast_players: usize,
    /// When each player last chatted (chat rate limit).
    last_chat: HashMap<PeerId, Instant>,
    /// Last `MSG_MATCH_STATUS` broadcast: value and time.
    last_status_sent: Option<(u8, Instant)>,
    /// Team kills this match (team modes).
    team_score: game::TeamScore,
    /// Last `MSG_TEAM_SCORE` broadcast: value and time.
    last_team_score_sent: Option<(game::TeamScore, Instant)>,
}

impl Server {
    pub fn new() -> Self {
        Server {
            outbox: Outbox::default(),
            players: HashMap::new(),
            bot_manager: BotManager::new(),
            name: "Gridlock".to_string(),
            maze: None,
            match_running: false,
            match_start_time: None,
            match_duration: game::MATCH_DURATION,
            match_warmup_end: None,
            match_end_pending: None,
            last_match_time_sent: None,
            last_warmup_sent: None,
            last_end_countdown_sent: None,
            match_mode: MODE_TEAM_DEATHMATCH,
            match_total_maze_entries: 0,
            snd_state: None,
            autofill_bots_per_team: 0,
            autofill_diff_mask: 0b11111,
            last_tick: Instant::now(),
            last_regen_tick: Instant::now(),
            last_state_broadcast: None,
            last_broadcast_players: 0,
            last_chat: HashMap::new(),
            last_status_sent: None,
            team_score: game::TeamScore::default(),
            last_team_score_sent: None,
        }
    }

    /// Name shown to clients browsing for games.
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Connected players, bots included (the same count discovery reports).
    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    /// Packets queued since the last call, as `(destination, bytes)`.
    pub fn drain_outbox(&mut self) -> Vec<(PeerId, Vec<u8>)> {
        self.outbox.drain()
    }

    /// Whether `peer` currently owns a player (joined and not timed out).
    pub fn has_peer(&self, peer: PeerId) -> bool {
        self.players.contains_key(&peer)
    }

    /// Transport reported the peer gone (e.g. WebRTC channel closed).
    pub fn peer_disconnected(&mut self, src_addr: PeerId) {
        handlers::handle_disconnect(&self.outbox, &mut self.players, src_addr);
    }

    /// Process one inbound packet (`[msg_type][payload]`) from `src_addr`.
    pub fn handle_packet(&mut self, src_addr: PeerId, packet: &[u8]) {
        let Some(msg) = ClientMsg::decode(packet) else {
            if let Some(msg_type) = packet.first() {
                log!("Unknown or malformed message type: {}", msg_type);
            }
            return;
        };

        match msg {
            ClientMsg::Join { username } => {
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
            }
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
                    broadcast::broadcast_state(&self.outbox, &self.players);
                }
            },
            // client requests to start a match from the lobby (or join the running one)
            ClientMsg::StartGame { mode, duration, autofill } => {
                self.handle_start_request(src_addr, mode, duration, autofill)
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

    /// `MSG_START_GAME`: the host starts a match from the lobby; while one runs,
    /// anyone in the lobby joins it. The mode, duration and bot settings only
    /// apply to a new match: a player joining never changes the one running.
    fn handle_start_request(
        &mut self,
        src_addr: PeerId,
        mode: Option<u8>,
        duration: Option<u16>,
        autofill: Option<(u8, u8)>,
    ) {
        self.end_match_if_abandoned();
        let new_match = !self.match_running
            && self.match_warmup_end.is_none()
            && self.match_end_pending.is_none()
            && game::host_addr(&self.players) == Some(src_addr);
        let mut requested_duration = self.match_duration;
        if new_match {
            if let Some(requested) = mode {
                if requested == MODE_GUN_GAME
                    || requested == MODE_TEAM_DEATHMATCH
                    || requested == MODE_SEARCH_AND_DESTROY
                {
                    self.match_mode = requested;
                }
            }
            // auto-fill settings: bots_per_team, diff_mask
            if let Some((bots_per_team, diff_mask)) = autofill {
                self.autofill_bots_per_team = bots_per_team.min(8);
                self.autofill_diff_mask = diff_mask & 0b11111;
            } else {
                self.autofill_bots_per_team = 0;
                self.autofill_diff_mask = 0b11111;
            }
            // duration code: 0=default, 65535=unlimited; a non-zero value
            // overrides the mode default
            let dur_code: u16 = duration.unwrap_or(0);
            requested_duration = if dur_code == u16::MAX {
                None // unlimited
            } else if dur_code > 0 {
                Some(Duration::from_secs(dur_code as u64))
            } else {
                match self.match_mode {
                    MODE_TEAM_DEATHMATCH => Some(Duration::from_secs(900)),
                    MODE_SEARCH_AND_DESTROY => None,
                    _ => game::MATCH_DURATION,
                }
            };
        }
        handlers::handle_start_game(
            &self.outbox,
            &mut self.players,
            &mut self.maze,
            &mut self.match_running,
            &mut self.match_start_time,
            &mut self.match_warmup_end,
            &mut self.match_end_pending,
            &mut self.match_total_maze_entries,
            self.match_mode,
            &mut self.match_duration,
            requested_duration,
            src_addr,
        );
        self.send_match_status(false);
    }

    /// The match phase as clients see it (`MATCH_STATUS_*`).
    fn match_status(&self) -> u8 {
        if self.match_end_pending.is_some() {
            MATCH_STATUS_ENDING
        } else if self.match_running {
            MATCH_STATUS_RUNNING
        } else if self.match_warmup_end.is_some() {
            MATCH_STATUS_STARTING
        } else {
            MATCH_STATUS_NONE
        }
    }

    /// Broadcast the match phase when it changed, and every few seconds anyway
    /// (packets get lost, and new players need it). `force` sends now.
    fn send_match_status(&mut self, force: bool) {
        let status = self.match_status();
        let due = match self.last_status_sent {
            Some((sent, at)) => sent != status || at.elapsed() >= Duration::from_secs(2),
            None => true,
        };
        if force || due {
            let msg = ServerMsg::MatchStatus { status }.encode();
            broadcast::to_humans(&self.outbox, &self.players, &msg);
            self.last_status_sent = Some((status, Instant::now()));
        }
    }

    /// End a running match once every human has left the Maze (quit or went
    /// back to the lobby), so bots don't play on by themselves.
    fn end_match_if_abandoned(&mut self) {
        if !self.match_running {
            return;
        }
        let humans_in_maze = self
            .players
            .values()
            .filter(|p| p.world == WorldId::Maze && !p.is_bot)
            .count();
        if humans_in_maze == 0 {
            log!("No players remain in match — ending match early");
            self.end_match_now();
        }
    }

    /// Handler for `MSG_END_MATCH`: the host stops the match and everyone goes
    /// back to the lobby, still connected. Ignored when no match is on.
    fn handle_end_match(&mut self, src_addr: PeerId) {
        if !self.match_running && self.match_end_pending.is_none() {
            return;
        }
        if game::host_addr(&self.players) != Some(src_addr) {
            let msg = ServerMsg::Chat {
                from_id: 0,
                name: String::new(),
                text: "Only the host can end the match".to_string(),
            }
            .encode();
            let _ = self.outbox.send_to(&msg, src_addr);
            return;
        }
        let host = self.players.get(&src_addr).map(|p| p.username.clone()).unwrap_or_default();
        log!("Match ended by host '{}'", host);
        self.end_match_now();
        handlers::broadcast_notice(&self.outbox, &self.players, &format!("{} ended the match", host));
    }

    /// Stop the current match at once: bots despawn, every player returns to
    /// the lobby and match state resets. Connections are kept.
    fn end_match_now(&mut self) {
        handlers::execute_scheduled_match_end(
            &self.outbox,
            &mut self.players,
            &mut self.maze,
            &mut self.match_running,
            &mut self.match_start_time,
            &mut self.match_end_pending,
            &mut self.last_match_time_sent,
            &mut self.match_total_maze_entries,
            &mut self.match_duration,
            String::new(),
            &mut self.snd_state,
            &mut self.bot_manager,
        );
        self.send_match_status(false);
    }

    /// Periodic simulation step; call after each packet and at least every 100ms.
    pub fn tick(&mut self) {
        let now_tick = Instant::now();
        let dt = now_tick.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now_tick;

        // Auto-respawn dead players EXCEPT in Search & Destroy mode (one life per round)
        if self.match_mode != MODE_SEARCH_AND_DESTROY {
            game::process_auto_respawns(&self.outbox, self.maze.as_ref(), &mut self.players, self.match_mode);
        }

        // S&D tick: advance round state
        if self.match_running && self.match_mode == MODE_SEARCH_AND_DESTROY {
            if let Some(ref mut snd) = self.snd_state {
                if let Some((winner_name, winner_id)) = snd.tick(dt, &self.outbox, &mut self.players) {
                    // S&D match is over
                    log!("S&D match over: {} wins", winner_name);
                    let end_msg = ServerMsg::MatchEnd {
                        winner_id: winner_id,
                        winner_name: winner_name.to_string(),
                        countdown: Some(game::MATCH_END_COUNTDOWN_SECS),
                    }
                    .encode();
                    for (addr, _) in self.players.iter() {
                        let _ = self.outbox.send_to(&end_msg, *addr);
                    }
                    self.match_end_pending = Some((
                        Instant::now() + Duration::from_secs(game::MATCH_END_COUNTDOWN_SECS as u64),
                        winner_id,
                        winner_name,
                    ));
                    self.match_running = false;
                    self.snd_state = None;
                }
            }
        }
        // tick bots
        self.bot_manager.tick(
            &self.outbox,
            &mut self.players,
            &mut self.match_end_pending,
            self.match_mode,
            self.maze.as_ref(),
            &mut self.team_score,
        );
        if self.bot_manager.handle_range_challenge_timeout(&mut self.players) {
            broadcast::broadcast_state(&self.outbox, &self.players);
            broadcast::broadcast_player_list(&self.outbox, &self.players);
        }
        if self.last_regen_tick.elapsed() >= Duration::from_secs(1) {
            game::process_lobby_regen(&mut self.players);
            self.last_regen_tick = Instant::now();
        }
        let due = self
            .last_state_broadcast
            .map_or(true, |t| t.elapsed() >= game::STATE_BROADCAST_INTERVAL);
        if due || self.players.len() != self.last_broadcast_players {
            self.last_state_broadcast = Some(Instant::now());
            self.last_broadcast_players = self.players.len();
            broadcast::broadcast_state_with_events(&self.outbox, &self.players);
            // One-shot movement events have now gone out; they stay set until
            // then, however many position updates arrive in between.
            for player in self.players.values_mut() {
                player.evt_jump_start = false;
                player.evt_landed = false;
            }
        }

        let timed_out = game::cleanup_inactive_players(&mut self.players);
        if !timed_out.is_empty() {
            for p in &timed_out {
                handlers::announce_left(&self.outbox, &self.players, p, LEAVE_TIMED_OUT);
            }
            // players were removed — broadcast so other clients drop their characters
            broadcast::broadcast_state(&self.outbox, &self.players);
            broadcast::broadcast_player_list(&self.outbox, &self.players);
        }
        if self.last_chat.len() > self.players.len() {
            let players = &self.players;
            self.last_chat.retain(|addr, _| players.contains_key(addr));
        }

        self.end_match_if_abandoned();

        // Warmup expiry: actually start the match when the warmup countdown ends
        if self.match_warmup_end.is_some() && !self.match_running {
            // every match starts from nil
            self.team_score = game::TeamScore::default();
            handlers::maybe_start_match(
                &self.outbox,
                &mut self.players,
                &mut self.maze,
                &mut self.match_running,
                &mut self.match_start_time,
                &mut self.match_warmup_end,
                &mut self.last_match_time_sent,
                &mut self.match_total_maze_entries,
                self.match_mode,
                &mut self.snd_state,
                self.match_duration,
                &mut self.bot_manager,
                self.autofill_bots_per_team,
                self.autofill_diff_mask,
            );
        }

        // Periodically rebroadcast warmup countdown so clients that missed
        // the original packet can catch up (helps offscreen/paused clients).
        if let Some(end) = self.match_warmup_end {
            let secs = if Instant::now() >= end {
                0u8
            } else {
                end.saturating_duration_since(Instant::now())
                    .as_secs()
                    .min(255) as u8
            };
            if self.last_warmup_sent != Some(secs) {
                let cd = ServerMsg::MatchCountdown { secs, mode: Some(self.match_mode) }.encode();
                for (addr, _) in self.players.iter() {
                    let _ = self.outbox.send_to(&cd, *addr);
                }
                self.last_warmup_sent = Some(secs);
            }
        } else {
            self.last_warmup_sent = None;
        }

        // If a match is running, check player counts for early end conditions (skip for S&D, it handles its own)
        if self.match_running && self.match_end_pending.is_none() && self.match_mode != MODE_SEARCH_AND_DESTROY {
            handlers::check_early_end_conditions(
                &self.outbox,
                &mut self.players,
                &mut self.maze,
                &mut self.match_running,
                &mut self.match_end_pending,
                &mut self.match_total_maze_entries,
                &mut self.match_start_time,
                self.match_mode,
                &mut self.match_duration,
                self.team_score,
            );
        }

        // Basic match timer: end match after MATCH_DURATION (skip for S&D, rounds handle timing)
        if self.match_running && self.match_mode != MODE_SEARCH_AND_DESTROY {
            handlers::update_match_timer(
                &self.outbox,
                &mut self.players,
                &mut self.maze,
                &mut self.match_running,
                &mut self.match_start_time,
                &mut self.last_match_time_sent,
                &mut self.match_total_maze_entries,
                &mut self.match_end_pending,
                self.match_mode,
                self.match_duration,
                self.team_score,
            );
        }

        // If a match-end was scheduled (winner reached score or last-man-standing), execute it when the countdown expires
        if self.match_end_pending.as_ref().map_or(false, |(when, _, _)| Instant::now() >= *when) {
            let winner_name = self.match_end_pending.as_ref().map(|(_, _, n)| n.clone()).unwrap_or_default();
            handlers::execute_scheduled_match_end(
                &self.outbox,
                &mut self.players,
                &mut self.maze,
                &mut self.match_running,
                &mut self.match_start_time,
                &mut self.match_end_pending,
                &mut self.last_match_time_sent,
                &mut self.match_total_maze_entries,
                &mut self.match_duration,
                winner_name,
                &mut self.snd_state,
                &mut self.bot_manager,
            );
        }

        // Rebroadcast match-end countdown periodically so offscreen clients
        // display the remaining end-of-match timer even if they missed the
        // original MSG_MATCH_END packet.
        if let Some((when, winner_id, winner_name)) = self.match_end_pending.as_ref() {
            let secs = if Instant::now() >= *when {
                0u8
            } else {
                when.saturating_duration_since(Instant::now())
                    .as_secs()
                    .min(255) as u8
            };
            if self.last_end_countdown_sent != Some(secs) {
                let end_msg = ServerMsg::MatchEnd {
                    winner_id: *winner_id,
                    winner_name: winner_name.to_string(),
                    countdown: Some(secs),
                }
                .encode();
                for (addr, _) in self.players.iter() {
                    let _ = self.outbox.send_to(&end_msg, *addr);
                }
                self.last_end_countdown_sent = Some(secs);
            }
        } else {
            self.last_end_countdown_sent = None;
        }
        self.send_match_status(false);
        self.send_team_score();
    }

    /// Broadcast the team score during a Team Deathmatch: when it changes, and
    /// every few seconds anyway (packets get lost, and new players need it).
    fn send_team_score(&mut self) {
        let in_play = self.match_mode == MODE_TEAM_DEATHMATCH
            && (self.match_running || self.match_end_pending.is_some());
        if !in_play {
            self.last_team_score_sent = None;
            return;
        }
        let score = self.team_score;
        let due = match self.last_team_score_sent {
            Some((sent, at)) => sent != score || at.elapsed() >= Duration::from_secs(2),
            None => true,
        };
        if due {
            let clamp = |v: u32| v.min(u16::MAX as u32) as u16;
            let msg = ServerMsg::TeamScore {
                red: clamp(score.red),
                blue: clamp(score.blue),
                to_win: clamp(game::MATCH_SCORE_TO_WIN),
            }
            .encode();
            broadcast::to_humans(&self.outbox, &self.players, &msg);
            self.last_team_score_sent = Some((score, Instant::now()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to(out: &[(PeerId, Vec<u8>)], peer: PeerId) -> Vec<ServerMsg> {
        out.iter().filter(|(p, _)| *p == peer).filter_map(|(_, b)| ServerMsg::decode(b)).collect()
    }

    fn start_match(server: &mut Server, host: PeerId) {
        let start = ClientMsg::StartGame { mode: Some(MODE_TEAM_DEATHMATCH), duration: Some(0), autofill: None };
        server.handle_packet(host, &start.encode());
        // skip the warmup countdown
        server.match_warmup_end = Some(Instant::now());
        server.tick();
        assert!(server.match_running);
    }

    #[test]
    fn host_ends_match_and_everyone_stays_connected() {
        let mut server = Server::new();
        let (a, b) = (PeerId(1), PeerId(2));
        server.handle_packet(a, &ClientMsg::Join { username: "alice".into() }.encode());
        server.handle_packet(b, &ClientMsg::Join { username: "bob".into() }.encode());
        start_match(&mut server, a);
        server.drain_outbox();

        // only the host may end it
        server.handle_packet(b, &ClientMsg::EndMatch.encode());
        assert!(server.match_running);
        assert!(!to(&server.drain_outbox(), b).iter().any(|m| matches!(m, ServerMsg::WorldChange { .. })));

        server.handle_packet(a, &ClientMsg::EndMatch.encode());
        let out = server.drain_outbox();
        for peer in [a, b] {
            let msgs = to(&out, peer);
            assert!(msgs.iter().any(|m| matches!(m, ServerMsg::WorldChange { world: 0, .. })));
            assert!(msgs.iter().any(|m| matches!(m, ServerMsg::Chat { from_id: 0, .. })));
        }
        assert!(!server.match_running && server.maze.is_none());
        assert!(server.has_peer(a) && server.has_peer(b));

        // the lobby is still up: a new match can start straight away
        start_match(&mut server, a);
    }

    #[test]
    fn match_ends_once_every_human_has_left_it() {
        let mut server = Server::new();
        let (a, b) = (PeerId(1), PeerId(2));
        server.handle_packet(a, &ClientMsg::Join { username: "alice".into() }.encode());
        server.handle_packet(b, &ClientMsg::Join { username: "bob".into() }.encode());
        start_match(&mut server, a);
        server.drain_outbox();

        let lobby = ClientMsg::WorldChange { target: WorldId::Lobby.as_u8() };
        server.handle_packet(b, &lobby.encode());
        server.tick();
        assert!(server.match_running, "alice is still playing");
        let notice = ServerMsg::Chat { from_id: 0, name: String::new(), text: "bob left the match".into() };
        assert!(to(&server.drain_outbox(), a).contains(&notice));

        server.handle_packet(a, &lobby.encode());
        server.tick();
        assert!(!server.match_running && server.maze.is_none());
    }

    fn statuses(out: &[(PeerId, Vec<u8>)], peer: PeerId) -> Vec<u8> {
        to(out, peer)
            .into_iter()
            .filter_map(|m| match m {
                ServerMsg::MatchStatus { status } => Some(status),
                _ => None,
            })
            .collect()
    }

    fn two_players() -> (Server, PeerId, PeerId) {
        let mut server = Server::new();
        let (a, b) = (PeerId(1), PeerId(2));
        server.handle_packet(a, &ClientMsg::Join { username: "alice".into() }.encode());
        server.handle_packet(b, &ClientMsg::Join { username: "bob".into() }.encode());
        (server, a, b)
    }

    #[test]
    fn host_stays_host_while_in_the_match() {
        let (mut server, a, b) = two_players();
        start_match(&mut server, a);
        // bob goes back to the lobby; alice, in the maze, is still the host
        server.handle_packet(b, &ClientMsg::WorldChange { target: WorldId::Lobby.as_u8() }.encode());
        let leaders: Vec<u32> = to(&server.drain_outbox(), b)
            .into_iter()
            .filter_map(|m| match m {
                ServerMsg::LobbyLeader { id } => Some(id),
                _ => None,
            })
            .collect();
        assert!(!leaders.is_empty());
        assert!(leaders.iter().all(|id| *id == 1), "host changed: {leaders:?}");
    }

    #[test]
    fn anyone_in_the_lobby_can_join_a_running_match_without_changing_it() {
        let (mut server, a, b) = two_players();
        start_match(&mut server, a);
        let (mode, duration) = (server.match_mode, server.match_duration);
        server.handle_packet(b, &ClientMsg::WorldChange { target: WorldId::Lobby.as_u8() }.encode());
        server.drain_outbox();

        // bob (not the host) rejoins, asking for other settings
        let start = ClientMsg::StartGame {
            mode: Some(MODE_GUN_GAME),
            duration: Some(30),
            autofill: Some((8, 0b1)),
        };
        server.handle_packet(b, &start.encode());
        let out = server.drain_outbox();
        assert!(to(&out, b).iter().any(|m| matches!(m, ServerMsg::WorldChange { world: 1, .. })));
        assert_eq!(server.players[&b].world, WorldId::Maze);
        assert_eq!((server.match_mode, server.match_duration), (mode, duration));
        assert_eq!(server.autofill_bots_per_team, 0);
    }

    #[test]
    fn start_requests_while_starting_or_ending_are_refused_with_a_reason() {
        let (mut server, a, b) = two_players();
        let start = ClientMsg::StartGame { mode: Some(MODE_TEAM_DEATHMATCH), duration: Some(0), autofill: None };
        server.handle_packet(a, &start.encode());
        server.drain_outbox();
        server.handle_packet(b, &start.encode());
        assert!(to(&server.drain_outbox(), b)
            .contains(&ServerMsg::StartDenied { reason: Some(START_DENIED_STARTING) }));

        server.match_warmup_end = Some(Instant::now());
        server.tick();
        server.match_end_pending = Some((Instant::now() + Duration::from_secs(60), 1, "alice".into()));
        server.handle_packet(b, &ClientMsg::WorldChange { target: WorldId::Lobby.as_u8() }.encode());
        server.drain_outbox();
        server.handle_packet(b, &start.encode());
        assert!(to(&server.drain_outbox(), b)
            .contains(&ServerMsg::StartDenied { reason: Some(START_DENIED_ENDING) }));
        assert_eq!(server.players[&b].world, WorldId::Lobby);
    }

    #[test]
    fn match_status_follows_the_match() {
        let (mut server, a, b) = two_players();
        assert_eq!(statuses(&server.drain_outbox(), b).last(), Some(&MATCH_STATUS_NONE));

        let start = ClientMsg::StartGame { mode: Some(MODE_TEAM_DEATHMATCH), duration: Some(0), autofill: None };
        server.handle_packet(a, &start.encode());
        assert_eq!(statuses(&server.drain_outbox(), b).last(), Some(&MATCH_STATUS_STARTING));

        server.match_warmup_end = Some(Instant::now());
        server.tick();
        assert_eq!(statuses(&server.drain_outbox(), b).last(), Some(&MATCH_STATUS_RUNNING));

        server.handle_packet(a, &ClientMsg::EndMatch.encode());
        assert_eq!(statuses(&server.drain_outbox(), b).last(), Some(&MATCH_STATUS_NONE));
    }

    #[test]
    fn team_score_counts_kills_as_they_happen_and_keeps_them() {
        let (mut server, a, b) = two_players();
        start_match(&mut server, a);
        let (alice, bob) = (server.players[&a].team, server.players[&b].team);
        assert_ne!(alice, bob, "TDM puts the two on opposite teams");
        server.players.get_mut(&a).unwrap().post_teleport_freeze_until = None;
        server.players.get_mut(&b).unwrap().post_teleport_freeze_until = None;
        server.drain_outbox();

        let kill = ClientMsg::Hit { victim_id: 2, damage: 500.0, weapon: Some("AK-47".into()), backstab: None };
        server.handle_packet(a, &kill.encode());
        server.tick();
        let expected = if alice == crate::player::Team::Red {
            game::TeamScore { red: 1, blue: 0 }
        } else {
            game::TeamScore { red: 0, blue: 1 }
        };
        assert_eq!(server.team_score, expected);
        let (red, blue) = (expected.red as u16, expected.blue as u16);
        let sent = ServerMsg::TeamScore { red, blue, to_win: game::MATCH_SCORE_TO_WIN as u16 };
        assert!(to(&server.drain_outbox(), b).contains(&sent));

        // alice leaving the match doesn't take her kill off her team
        server.handle_packet(a, &ClientMsg::WorldChange { target: WorldId::Lobby.as_u8() }.encode());
        server.tick();
        assert_eq!(server.team_score, expected);
    }

    #[test]
    fn team_kills_score_nothing() {
        let mut score = game::TeamScore::default();
        score.record_kill(crate::player::Team::Red, crate::player::Team::Red);
        score.record_kill(crate::player::Team::Spectator, crate::player::Team::Blue);
        assert_eq!(score, game::TeamScore::default());
        score.record_kill(crate::player::Team::Blue, crate::player::Team::Red);
        assert_eq!(score, game::TeamScore { red: 0, blue: 1 });
    }
}
