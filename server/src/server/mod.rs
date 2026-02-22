use std::collections::HashMap;
use std::fs;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

pub mod broadcast;
pub mod game;
pub mod handlers;
pub mod utils;

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

    // cache assets manifest to avoid hitting the filesystem on every join
    let asset_manifest: Option<serde_json::Value> = fs::read_to_string("assets/manifest.json")
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
    // current match mode (defaults to Gun Game)
    let match_mode: u8 = MODE_GUN_GAME;
    // how many players entered the maze during this match (for last-man-standing detection)
    let mut match_total_maze_entries: usize = 0;

    loop {
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
                        handlers::handle_position(&socket, &mut players, src_addr, payload)
                    }

                    // client-requested world change (teleporter)
                    MSG_WORLD_CHANGE => {
                        handlers::handle_world_change(
                            &socket,
                            maze.as_ref(),
                            &mut players,
                            src_addr,
                            payload,
                        );
                    }

                    // client requests to start a match from the lobby
                    MSG_START_GAME => {
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
                            src_addr,
                        );
                    }
                    MSG_WALL_BROKEN => {
                        if let Some(ref mut m) = maze {
                            handlers::handle_wall_broken(&socket, m, &players, src_addr, payload)
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
                    ),
                    MSG_RESPAWN_REQUEST => {
                        if let Some(ref m) = maze {
                            handlers::handle_respawn_request(&socket, m, &mut players, src_addr)
                        }
                    }
                    _ => println!("Unknown message type: {}", msg_type),
                }
            }
            Err(err)
                if err.kind() == std::io::ErrorKind::WouldBlock
                    || err.kind() == std::io::ErrorKind::TimedOut => {}
            Err(err) => return Err(err),
        }

        if let Some(ref m) = maze {
            game::process_auto_respawns(&socket, m, &mut players);
        }
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
            );
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
                match_mode,
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
                    winner_name,
                );
            }
        }
    }
}
