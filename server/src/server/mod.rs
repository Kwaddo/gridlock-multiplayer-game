use std::collections::HashMap;
use std::fs;
use std::net::{ SocketAddr, UdpSocket };
use std::time::{ Duration, Instant };

pub mod broadcast;
pub mod game;
pub mod handlers;
pub mod utils;

use crate::bot::ai::{ BotBehavior, BotState };
use crate::maze::MazeData;
use crate::player::Player;
use crate::player::WorldId;
use crate::protocols::*;
use crate::bot::manager::BotManager;

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
    let asset_manifest: Option<serde_json::Value> = fs
        ::read_to_string("assets/manifest.json")
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok());

    // Do not generate a full gameplay maze at server start — create per-match instead.
    let mut maze: Option<MazeData> = None; // Some(...) when a match is running

    // Basic match state for a future game loop
    let mut match_running: bool = false;
    let mut match_start_time: Option<Instant> = None;
    // warmup / countdown before match actually begins
    let mut match_warmup_end: Option<Instant> = None;
    // scheduled match end (winner, delayed return-to-lobby)
    let mut match_end_pending: Option<(Instant, u32, String)> = None;
    // last sent authoritative match timer (to avoid spamming identical values)
    let mut last_match_time_sent: Option<u8> = None;
    // last sent warmup countdown (MSG_MATCH_COUNTDOWN)
    let mut last_warmup_sent: Option<u8> = None;
    // last sent match-end countdown (MSG_MATCH_END countdown byte)
    let mut last_end_countdown_sent: Option<u8> = None;
    // current match mode (defaults to Team Deathmatch)
    let mut match_mode: u8 = MODE_TEAM_DEATHMATCH;
    // how many players entered the maze during this match (for last-man-standing detection)
    let mut match_total_maze_entries: usize = 0;

    loop {
        match socket.recv_from(&mut buffer) {
            Ok((bytes_rcv, src_addr)) => {
                let msg_type = buffer[0];
                let payload = &buffer[1..bytes_rcv];

                match msg_type {
                    MSG_JOIN =>
                        handlers::handle_join(
                            &socket,
                            &mut players,
                            maze.as_ref(),
                            match_running,
                            src_addr,
                            payload,
                            asset_manifest.as_ref()
                        ),
                    MSG_DISCONNECT => handlers::handle_disconnect(&socket, &mut players, src_addr),
                    MSG_POSITION => {
                        handlers::handle_position(&socket, &mut players, src_addr, payload);
                    }

                    // client-requested world change (teleporter)
                    MSG_WORLD_CHANGE => {
                        handlers::handle_world_change(
                            &socket,
                            maze.as_ref(),
                            &mut players,
                            src_addr,
                            payload
                        );
                    }

                    // client requests to start a match from the lobby
                    MSG_START_GAME => {
                        // read the requested game mode from the payload
                        if !payload.is_empty() {
                            let requested = payload[0];
                            if
                                requested == MODE_GUN_GAME ||
                                requested == MODE_TEAM_DEATHMATCH ||
                                requested == MODE_SEARCH_AND_DESTROY
                            {
                                match_mode = requested;
                            }
                        }
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
                            src_addr
                        );
                    }
                    MSG_WALL_BROKEN => {
                        if let Some(ref mut m) = maze {
                            handlers::handle_wall_broken(&socket, m, &players, src_addr, payload);
                        }
                    }
                    MSG_DISCOVER => handlers::handle_discover(&socket, &players, src_addr),
                    MSG_HIT =>
                        handlers::handle_hit(
                            &socket,
                            &mut players,
                            src_addr,
                            payload,
                            &mut maze,
                            &mut match_running,
                            &mut match_start_time,
                            &mut match_end_pending,
                            match_mode
                        ),
                    MSG_RESPAWN_REQUEST => {
                        if let Some(ref m) = maze {
                            handlers::handle_respawn_request(&socket, m, &mut players, src_addr);
                        }
                    }
                    MSG_SET_TEAM => {
                        handlers::handle_set_team(&socket, &mut players, src_addr, payload);
                    }
                    MSG_BOT_SPAWN => {
                        if payload.len() >= 2 {
                            let behavior_code = payload[0];
                            let level = payload[1];

                            let behavior: BotBehavior = match behavior_code {
                                0 => BotBehavior::Static,
                                1 =>
                                    BotBehavior::Patrol {
                                        waypoints: vec![
                                            (5.0, 0.0, 5.0),
                                            (5.0, 0.0, -5.0),
                                            (-5.0, 0.0, -5.0),
                                            (-5.0, 0.0, 5.0)
                                        ],
                                        current: 0,
                                        arrival_threshold: 1.0,
                                    },
                                2 =>
                                    BotBehavior::PatrolJump {
                                        waypoints: vec![
                                            (5.0, 0.0, 5.0),
                                            (5.0, 0.0, -5.0),
                                            (-5.0, 0.0, -5.0),
                                            (-5.0, 0.0, 5.0)
                                        ],
                                        current: 0,
                                        jump_interval: 2.0,
                                        jump_timer: 2.0,
                                        arrival_threshold: 1.0,
                                    },
                                _ => BotBehavior::Static,
                            };

                            let bot_count = players
                                .values()
                                .filter(|p| p.is_bot)
                                .count();
                            let spawn_points = [
                                (5.0_f32, 0.0_f32, 5.0_f32),
                                (5.0_f32, 0.0_f32, -5.0_f32),
                                (-5.0_f32, 0.0_f32, 5.0_f32),
                                (-5.0_f32, 0.0_f32, -5.0_f32),
                            ];
                            let spawn = spawn_points[bot_count % spawn_points.len()];

                            let bot_id = 9000u32 + (bot_count as u32);
                            let bot_name = format!("Bot{}", bot_count + 1);

                            let bot_player = Player {
                                id: bot_id,
                                username: bot_name,
                                position: spawn,
                                yaw: 0.0,
                                pitch: 0.0,
                                health: 100,
                                shield: 0,
                                max_health: 100,
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
                                team: crate::player::Team::Red,
                                current_weapon: "MP5".to_string(),
                                is_bot: true,
                            };

                            let (fire_cooldown, reaction_remaining) = match level {
                                1 => (2.0, 1.5),
                                2 => (1.0, 1.0),
                                3 => (0.5, 0.5),
                                _ => (2.0, 1.5),
                            };

                            let bot_state = BotState {
                                movement: crate::physics::BotMovementState {
                                    velocity: glam::Vec3::ZERO,
                                    is_grounded: true,
                                    height: 1.6,
                                },
                                behavior,
                                shooting: crate::bot::shooting::BotShootingState {
                                    fire_cooldown,
                                    has_target: false,
                                    reaction_remaining,
                                },
                                target_id: None,
                                reaction_timer: 0.0,
                            };

                            bot_manager.spawn_bot(&mut players, bot_player, bot_state);
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
            Err(err) if
                err.kind() == std::io::ErrorKind::WouldBlock ||
                err.kind() == std::io::ErrorKind::TimedOut ||
                err.kind() == std::io::ErrorKind::ConnectionReset
            => {}
            Err(err) => {
                return Err(err);
            }
        }

        // tick bots
        bot_manager.tick(&mut players);
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
                match_mode
            );
        }

        // Periodically rebroadcast warmup countdown so clients that missed
        // the original packet can catch up (helps offscreen/paused clients).
        if let Some(end) = match_warmup_end {
            let secs = if Instant::now() >= end {
                0u8
            } else {
                end.saturating_duration_since(Instant::now()).as_secs().min(255) as u8
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

        // If a match is running, check player counts for early end conditions
        if match_running && match_end_pending.is_none() {
            handlers::check_early_end_conditions(
                &socket,
                &mut players,
                &mut maze,
                &mut match_running,
                &mut match_end_pending,
                &mut match_total_maze_entries,
                &mut match_start_time,
                match_mode
            );
        }

        // Basic match timer: end match after MATCH_DURATION and send periodic authoritative timer updates
        if match_running {
            handlers::update_match_timer(
                &socket,
                &mut players,
                &mut maze,
                &mut match_running,
                &mut match_start_time,
                &mut last_match_time_sent,
                &mut match_total_maze_entries,
                &mut match_end_pending,
                match_mode
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
                    winner_name
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
                when.saturating_duration_since(Instant::now()).as_secs().min(255) as u8
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
