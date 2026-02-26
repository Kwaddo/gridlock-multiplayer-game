use rand::Rng;
use std::collections::HashMap;
use std::fs;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

pub mod broadcast;
pub mod game;
pub mod handlers;
pub mod snd;
pub mod utils;

use crate::bot::ai::BotState;
use crate::bot::manager::BotManager;
use crate::maze::MazeData;
use crate::player::Player;
use crate::player::WorldId;
use crate::protocols::*;

/// Entry point for the server logic; previously lived in `main.rs`.
/// This function handles socket setup and contains the main loop.
pub fn run() -> std::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:7878")?;
    socket.set_read_timeout(Some(Duration::from_millis(100)))?;
    println!("UDP server started on port 7878");

    let mut buffer: [u8; 512] = [0; 512];
    let mut players: HashMap<SocketAddr, Player> = HashMap::new();

    let mut bot_manager = BotManager::new();

    // cache assets manifest to avoid hitting the filesystem on every join
    let asset_manifest: Option<serde_json::Value> = fs::read_to_string("assets/manifest.json")
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok());

    // Do not generate a full gameplay maze at server start — create per-match instead.
    let mut maze: Option<MazeData> = None; // Some(...) when a match is running

    // Basic match state for a future game loop
    let mut match_running: bool = false;
    let mut match_start_time: Option<Instant> = None;
    // current running match length (None=infinite); initially use the default
    let mut match_duration: Option<Duration> = game::MATCH_DURATION;
    // warmup / countdown before match actually begins
    let mut match_warmup_end: Option<Instant> = None;
    // scheduled match end (winner, delayed return-to-lobby)
    let mut match_end_pending: Option<(Instant, u32, String)> = None;
    // last sent authoritative match timer (to avoid spamming identical values)
    let mut last_match_time_sent: Option<u16> = None;
    // last sent warmup countdown (MSG_MATCH_COUNTDOWN)
    let mut last_warmup_sent: Option<u8> = None;
    // last sent match-end countdown (MSG_MATCH_END countdown byte)
    let mut last_end_countdown_sent: Option<u8> = None;
    // current match mode (defaults to Team Deathmatch)
    let mut match_mode: u8 = MODE_TEAM_DEATHMATCH;
    // how many players entered the maze during this match (for last-man-standing detection)
    let mut match_total_maze_entries: usize = 0;
    // Search & Destroy state (only Some when S&D mode is active)
    let mut snd_state: Option<snd::SndState> = None;
    // Track last tick time for delta-time calculation
    let mut last_tick = Instant::now();

    loop {
        let now_tick = Instant::now();
        let dt = now_tick.duration_since(last_tick).as_secs_f32();
        last_tick = now_tick;
        match socket.recv_from(&mut buffer) {
            Ok((bytes_rcv, src_addr)) => {
                let msg_type = buffer[0];
                let payload = &buffer[1..bytes_rcv];

                match msg_type {
                    MSG_JOIN => handlers::handle_join(
                        &socket,
                        &mut players,
                        maze.as_ref(),
                        match_running,
                        src_addr,
                        payload,
                        asset_manifest.as_ref(),
                    ),
                    MSG_DISCONNECT => handlers::handle_disconnect(&socket, &mut players, src_addr),
                    MSG_POSITION => {
                        handlers::handle_position(&socket, &mut players, src_addr, payload);
                    },

                    // client-requested world change (teleporter)
                    MSG_WORLD_CHANGE => {
                        handlers::handle_world_change(
                            &socket,
                            maze.as_ref(),
                            &mut players,
                            src_addr,
                            payload,
                        );
                    },
                    MSG_WEAPON_CHANGE => {
                        // client tells us which weapon they're now holding; update
                        // player state and broadcast to everyone so world models
                        // stay in sync.
                        if let Some(p) = players.get_mut(&src_addr) {
                            if !payload.is_empty() {
                                let len = payload[0] as usize;
                                if payload.len() >= 1 + len {
                                    if let Ok(name) = std::str::from_utf8(&payload[1..1 + len]) {
                                        p.current_weapon = name.to_string();
                                        broadcast::broadcast_state(&socket, &players);
                                    }
                                }
                            }
                        }
                    },
                    // client requests to start a match from the lobby
                    MSG_START_GAME => {
                        // read the requested game mode from the payload
                        if !payload.is_empty() {
                            let requested = payload[0];
                            if requested == MODE_GUN_GAME
                                || requested == MODE_TEAM_DEATHMATCH
                                || requested == MODE_SEARCH_AND_DESTROY
                            {
                                match_mode = requested;
                            }
                        }
                        // Duration is fixed per mode: TDM = 15 min, S&D = None (round-based), Gun Game = default
                        let requested_duration: Option<Duration> = match match_mode {
                            MODE_TEAM_DEATHMATCH => Some(Duration::from_secs(900)),
                            MODE_SEARCH_AND_DESTROY => None,
                            _ => game::MATCH_DURATION,
                        };
                        handlers::handle_start_game(
                            &socket,
                            &mut players,
                            &mut maze,
                            match_running,
                            &mut match_start_time,
                            &mut match_warmup_end,
                            &mut match_end_pending,
                            &mut match_total_maze_entries,
                            match_mode,
                            &mut match_duration,
                            requested_duration,
                            src_addr,
                        );
                    }
                    MSG_WALL_BROKEN => {
                        if let Some(ref mut m) = maze {
                            handlers::handle_wall_broken(&socket, m, &players, src_addr, payload);
                        }
                    }
                    MSG_DISCOVER => handlers::handle_discover(&socket, &players, src_addr),
                    MSG_HIT => handlers::handle_hit(
                        &socket,
                        &mut players,
                        src_addr,
                        payload,
                        &mut maze,
                        &mut match_running,
                        &mut match_start_time,
                        &mut match_end_pending,
                        match_mode,
                        &mut snd_state,
                    ),
                    MSG_RESPAWN_REQUEST => {
                        handlers::handle_respawn_request(
                            &socket,
                            maze.as_ref(),
                            &mut players,
                            src_addr,
                            match_mode,
                            &snd_state,
                        );
                    }
                    MSG_SET_TEAM => {
                        handlers::handle_set_team(&socket, &mut players, src_addr, payload);
                    }
                    MSG_SND_BOMB_PICKUP => {
                        if let Some(ref mut snd) = snd_state {
                            snd.try_pickup_bomb(src_addr, &players);
                        }
                    }
                    MSG_SND_PLANT_START => {
                        if let Some(ref mut snd) = snd_state {
                            snd.try_start_plant(src_addr, &players);
                        }
                    }
                    MSG_SND_PLANT_STOP => {
                        if let Some(ref mut snd) = snd_state {
                            snd.stop_plant(src_addr);
                        }
                    }
                    MSG_SND_DEFUSE_START => {
                        if let Some(ref mut snd) = snd_state {
                            snd.try_start_defuse(src_addr, &players);
                        }
                    }
                    MSG_SND_DEFUSE_STOP => {
                        if let Some(ref mut snd) = snd_state {
                            snd.stop_defuse(src_addr);
                        }
                    }
                    MSG_BOT_SPAWN => {
                        if payload.len() >= 2 {
                            let mut rng = rand::thread_rng();
                            let behavior_code = payload[0];
                            let level = payload[1];
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
                            bot_manager.set_range_challenge_deadline(range_deadline);
                            let base_bot_count = players.values().filter(|p| p.is_bot).count();
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
                                    shield: 0,
                                    max_health: hp,
                                    max_shield: 100,
                                    is_alive: true,
                                    kills: 0,
                                    deaths: 0,
                                    gun_level: 1,
                                    socket_addr: BotManager::virtual_addr(0), // manager will replace with generated addr
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
                                    auto_respawn: false,
                                    lobby_regen_resume_at: None,
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
                                    },
                                    difficulty: crate::bot::personality::BotDifficulty::Normal,
                                    archetype: crate::bot::personality::BotArchetype::AllRounder,
                                };

                                bot_manager.spawn_bot(&mut players, bot_player, bot_state);
                            }
                            broadcast::broadcast_state(&socket, &players);
                            broadcast::broadcast_player_list(&socket, &players);
                        }
                    }

                    MSG_BOT_DESPAWN_ALL => {
                        bot_manager.despawn_all(&mut players);
                        broadcast::broadcast_state(&socket, &players);
                        broadcast::broadcast_player_list(&socket, &players);
                    }

                    _ => println!("Unknown message type: {}", msg_type),
                }
            }
            Err(err)
                if err.kind() == std::io::ErrorKind::WouldBlock
                    || err.kind() == std::io::ErrorKind::TimedOut
                    || err.kind() == std::io::ErrorKind::ConnectionReset => {}
            Err(err) => {
                return Err(err);
            }
        }

        // Auto-respawn dead players EXCEPT in Search & Destroy mode (one life per round)
        if match_mode != MODE_SEARCH_AND_DESTROY {
            game::process_auto_respawns(&socket, maze.as_ref(), &mut players);
        }

        // S&D tick: advance round state
        if match_running && match_mode == MODE_SEARCH_AND_DESTROY {
            if let Some(ref mut snd) = snd_state {
                if let Some((winner_name, winner_id)) = snd.tick(dt, &socket, &mut players) {
                    // S&D match is over
                    println!("S&D match over: {} wins", winner_name);
                    let mut end_msg = vec![MSG_MATCH_END];
                    end_msg.extend_from_slice(&winner_id.to_be_bytes());
                    let name_bytes = winner_name.as_bytes();
                    end_msg.push(name_bytes.len() as u8);
                    end_msg.extend_from_slice(name_bytes);
                    end_msg.push(game::MATCH_END_COUNTDOWN_SECS);
                    for p in players.values() {
                        let _ = socket.send_to(&end_msg, p.socket_addr);
                    }
                    match_end_pending = Some((
                        Instant::now() + Duration::from_secs(game::MATCH_END_COUNTDOWN_SECS as u64),
                        winner_id,
                        winner_name,
                    ));
                    match_running = false;
                    snd_state = None;
                }
            }
        }
        // tick bots
        bot_manager.tick(&socket, &mut players, &mut match_end_pending, match_mode);
        if bot_manager.handle_range_challenge_timeout(&mut players) {
            broadcast::broadcast_state(&socket, &players);
            broadcast::broadcast_player_list(&socket, &players);
        }
        game::process_lobby_regen(&mut players);
        broadcast::broadcast_state(&socket, &players);

        // Always attempt to auto-respawn dead players; maze may be None if no
        // match is currently generated but players still need respawn handling.
        game::process_auto_respawns(&socket, maze.as_ref(), &mut players);
        if game::cleanup_inactive_players(&mut players) {
            // players were removed — broadcast so other clients drop their characters
            broadcast::broadcast_state(&socket, &players);
            broadcast::broadcast_player_list(&socket, &players);
        }

        // If a match is running but there are no players left in the Maze, end it immediately
        if match_running {
            let maze_count = players
                .values()
                .filter(|p| p.world == WorldId::Maze)
                .count();
            if maze_count == 0 {
                println!("No players remain in match — ending match early");
                match_running = false;
                match_start_time = None;
                maze = None;
                match_total_maze_entries = 0;
            }
        }

        // Warmup expiry: actually start the match when the warmup countdown ends
        if match_warmup_end.is_some() && !match_running {
            handlers::maybe_start_match(
                &socket,
                &mut players,
                &mut maze,
                &mut match_running,
                &mut match_start_time,
                &mut match_warmup_end,
                &mut last_match_time_sent,
                &mut match_total_maze_entries,
                match_mode,
                &mut snd_state,
                match_duration,
            );
        }

        // Periodically rebroadcast warmup countdown so clients that missed
        // the original packet can catch up (helps offscreen/paused clients).
        if let Some(end) = match_warmup_end {
            let secs = if Instant::now() >= end {
                0u8
            } else {
                end.saturating_duration_since(Instant::now())
                    .as_secs()
                    .min(255) as u8
            };
            if last_warmup_sent != Some(secs) {
                let cd: [u8; 3] = [MSG_MATCH_COUNTDOWN, secs, match_mode];
                for p in players.values() {
                    let _ = socket.send_to(&cd, p.socket_addr);
                }
                last_warmup_sent = Some(secs);
            }
        } else {
            last_warmup_sent = None;
        }

        // If a match is running, check player counts for early end conditions (skip for S&D, it handles its own)
        if match_running && match_end_pending.is_none() && match_mode != MODE_SEARCH_AND_DESTROY {
            handlers::check_early_end_conditions(
                &socket,
                &mut players,
                &mut maze,
                &mut match_running,
                &mut match_end_pending,
                &mut match_total_maze_entries,
                &mut match_start_time,
                match_mode,
                &mut match_duration,
            );
        }

        // Basic match timer: end match after MATCH_DURATION (skip for S&D, rounds handle timing)
        if match_running && match_mode != MODE_SEARCH_AND_DESTROY {
            handlers::update_match_timer(
                &socket,
                &mut players,
                &mut maze,
                &mut match_running,
                &mut match_start_time,
                &mut last_match_time_sent,
                &mut match_total_maze_entries,
                &mut match_end_pending,
                match_mode,
                match_duration,
            );
        }

        // If a match-end was scheduled (winner reached score or last-man-standing), execute it when the countdown expires
        if let Some((when, _, winner_name)) = match_end_pending.clone() {
            if Instant::now() >= when {
                handlers::execute_scheduled_match_end(
                    &socket,
                    &mut players,
                    &mut maze,
                    &mut match_running,
                    &mut match_start_time,
                    &mut match_end_pending,
                    &mut last_match_time_sent,
                    &mut match_total_maze_entries,
                    &mut match_duration,
                    winner_name,
                    &mut snd_state,
                );
            }
        }

        // Rebroadcast match-end countdown periodically so offscreen clients
        // display the remaining end-of-match timer even if they missed the
        // original MSG_MATCH_END packet.
        if let Some((when, winner_id, winner_name)) = match_end_pending.clone() {
            let secs = if Instant::now() >= when {
                0u8
            } else {
                when.saturating_duration_since(Instant::now())
                    .as_secs()
                    .min(255) as u8
            };
            if last_end_countdown_sent != Some(secs) {
                let mut end_msg = vec![MSG_MATCH_END];
                end_msg.extend_from_slice(&winner_id.to_be_bytes());
                let name_bytes = winner_name.as_bytes();
                end_msg.push(name_bytes.len() as u8);
                end_msg.extend_from_slice(name_bytes);
                end_msg.push(secs);
                for p in players.values() {
                    let _ = socket.send_to(&end_msg, p.socket_addr);
                }
                last_end_countdown_sent = Some(secs);
            }
        } else {
            last_end_countdown_sent = None;
        }
    }
}
