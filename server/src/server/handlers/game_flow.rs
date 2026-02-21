use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use crate::maze::MazeData;
use crate::player::{Player, WorldId};
use crate::protocols::*;

use crate::server::{broadcast, game};

/// Handle a `MSG_START_GAME` packet sent from a lobby player.  This routine does not
/// actually start the match, but instead updates state and may broadcast countdowns.
pub fn handle_start_game(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    maze: &mut Option<MazeData>,
    match_running: bool,
    match_start_time: &mut Option<Instant>,
    match_warmup_end: &mut Option<Instant>,
    match_end_pending: &mut Option<(Instant, u32, String)>,
    match_total_maze_entries: &mut usize,
    match_mode: u8,
    src_addr: SocketAddr,
) {
    // If a match is marked running but there are no players in the Maze,
    // treat it as stale and clear it so new starts are accepted.
    if match_running {
        let maze_count = players
            .values()
            .filter(|p| p.world == WorldId::Maze)
            .count();
        if maze_count == 0 {
            println!("Stale match detected (no players) — clearing match state");
            // caller will clear flags; we just early return
            *match_start_time = None;
        }
    }

    // only accept start requests when not already running or scheduled
    if !match_running && match_warmup_end.is_none() && match_end_pending.is_none() {
        // require minimal number of players in lobby
        let lobby_count = players
            .values()
            .filter(|p| p.world == WorldId::Lobby)
            .count();
        if lobby_count >= game::MIN_PLAYERS_TO_START {
            // schedule a warmup/countdown before the match starts so all clients
            // can show a synchronized countdown UI.
            *match_warmup_end =
                Some(Instant::now() + Duration::from_secs(game::MATCH_START_COUNTDOWN_SECS as u64));

            // broadcast countdown to all connected clients (include selected gamemode)
            let cd: [u8; 3] = [
                MSG_MATCH_COUNTDOWN,
                game::MATCH_START_COUNTDOWN_SECS,
                match_mode,
            ];
            for p in players.values() {
                let _ = socket.send_to(&cd, p.socket_addr);
            }

            println!(
                "Match start requested by {} — mode {} — countdown {}s",
                src_addr,
                match_mode,
                game::MATCH_START_COUNTDOWN_SECS
            );
        } else {
            println!(
                "Start-game request ignored: not enough players ({}).",
                lobby_count
            );
        }
    } else {
        // match is already running and contains players — allow the requester to
        // join if there's a vacant slot, otherwise notify them that the match is full.
        let maze_count = players
            .values()
            .filter(|p| p.world == WorldId::Maze)
            .count();
        if maze_count < game::MATCH_MAX_PLAYERS {
            if let Some(req_player) = players.get_mut(&src_addr) {
                if req_player.world == WorldId::Lobby {
                    let current_maze = match &*maze {
                        Some(m) => m,
                        None => {
                            // shouldn't happen: match_running implies a maze exists
                            let _ = socket.send_to(&[MSG_START_DENIED], src_addr);
                            return;
                        }
                    };
                    let (sx, sy, sz, _used_fallback) =
                        current_maze.random_spawn_world(game::MAZE_CELL_SIZE);
                    req_player.world = WorldId::Maze;
                    req_player.position = (sx, sy, sz);
                    // set per-player freeze for newly-teleported joiners
                    req_player.post_teleport_freeze_until = Some(
                        Instant::now() + Duration::from_secs(game::MATCH_POST_TELEPORT_FREEZE_SECS),
                    );

                    // build fixed-size buffer: type(1)+world(1)+3*4bytes pos + mode(1)
                    let mut response = [0u8; 14];
                    response[0] = MSG_WORLD_CHANGE;
                    response[1] = WorldId::Maze.as_u8();
                    response[2..6].copy_from_slice(&sx.to_be_bytes());
                    response[6..10].copy_from_slice(&sy.to_be_bytes());
                    response[10..14].copy_from_slice(&sz.to_be_bytes());
                    // last byte overwritten by mode
                    response[13] = match_mode;
                    let _ = socket.send_to(&response, req_player.socket_addr);

                    // send maze data to the joining client
                    current_maze.send(&socket, req_player.socket_addr);

                    *match_total_maze_entries += 1;
                    println!(
                        "Player '{}' joined running match (vacant slot)",
                        req_player.username
                    );

                    // Broadcast announcement to all connected players
                    let name_bytes = req_player.username.as_bytes();
                    let mut ann = vec![MSG_MATCH_JOINED];
                    ann.extend_from_slice(&req_player.id.to_be_bytes());
                    ann.push(name_bytes.len() as u8);
                    ann.extend_from_slice(name_bytes);
                    for p in players.values() {
                        let _ = socket.send_to(&ann, p.socket_addr);
                    }
                }
            }
        } else {
            // Inform the requester the match is running and full
            let _ = socket.send_to(&[MSG_START_DENIED], src_addr);
            println!("Start-game request rejected: match already running and full.");
        }
    }
}

/// The warmup countdown has expired; actually start the match by generating a new
/// maze and moving lobby players into it.
pub fn maybe_start_match(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    maze: &mut Option<MazeData>,
    match_running: &mut bool,
    match_start_time: &mut Option<Instant>,
    match_warmup_end: &mut Option<Instant>,
    last_match_time_sent: &mut Option<u8>,
    match_total_maze_entries: &mut usize,
    match_mode: u8,
) {
    if let Some(end) = *match_warmup_end {
        if Instant::now() >= end {
            // start the match now (generate maze, move lobby players into Maze)
            *maze = Some(MazeData::new(game::MAZE_WIDTH, game::MAZE_HEIGHT));
            let current_maze = maze.as_ref().unwrap();
            let lobby_count = players
                .values()
                .filter(|p| p.world == WorldId::Lobby)
                .count();
            println!(
                "Match warmup complete — starting match with seed {} (moving {} players)",
                current_maze.seed, lobby_count
            );

            *match_running = true;
            *match_start_time = Some(Instant::now());
            *match_warmup_end = None;
            *match_total_maze_entries = lobby_count;
            // send initial authoritative match timer to all clients
            let initial_secs = game::MATCH_DURATION.as_secs().min(255) as u8;
            broadcast::broadcast_match_timer(socket, players, initial_secs);
            *last_match_time_sent = Some(initial_secs);
            // set per-player post-teleport freeze for each player moved into the Maze

            for player in players.values_mut().filter(|p| p.world == WorldId::Lobby) {
                let (sx, sy, sz, _used_fallback) =
                    current_maze.random_spawn_world(game::MAZE_CELL_SIZE);
                player.world = WorldId::Maze;
                player.position = (sx, sy, sz);
                player.post_teleport_freeze_until = Some(
                    Instant::now() + Duration::from_secs(game::MATCH_POST_TELEPORT_FREEZE_SECS),
                );

                // reset per-player Gun Game progression at match start
                player.gun_level = 1;

                let mut response = vec![MSG_WORLD_CHANGE, WorldId::Maze.as_u8()];
                response.extend_from_slice(&sx.to_be_bytes());
                response.extend_from_slice(&sy.to_be_bytes());
                response.extend_from_slice(&sz.to_be_bytes());
                // attach match mode so clients know which mode is starting
                response.push(match_mode);
                let _ = socket.send_to(&response, player.socket_addr);

                current_maze.send(&socket, player.socket_addr);
            }

            // broadcast updated player list/state so clients update immediately
            broadcast::broadcast_state(socket, players);
            broadcast::broadcast_player_list(socket, players);
        }
    }
}

/// Check for early match end conditions such as last‑man standing.
pub fn check_early_end_conditions(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    maze: &mut Option<MazeData>,
    match_running: &mut bool,
    match_end_pending: &mut Option<(Instant, u32, String)>,
    match_total_maze_entries: &mut usize,
    match_start_time: &mut Option<Instant>,
) {
    // count maze players and alive players in a single pass
    let mut maze_count = 0;
    let mut alive_count = 0;
    let mut last_alive: Option<&Player> = None;
    for p in players.values() {
        if p.world == WorldId::Maze {
            maze_count += 1;
            if p.is_alive {
                alive_count += 1;
                last_alive = Some(p);
            }
        }
    }

    if maze_count == 0 {
        println!("No players remain in match — ending match early");
        *match_running = false;
        *match_start_time = None;
        *maze = None;
    } else if alive_count == 1 && *match_total_maze_entries >= 2 {
        // Last player standing wins — only trigger when at least 2 players
        // entered the maze during this match (not just a solo player)
        let winner = last_alive.unwrap();
        let winner_id = winner.id;
        let winner_name = winner.username.clone();
        println!("Last player standing: '{}' wins the match", winner_name);

        // Broadcast MSG_MATCH_END
        let mut end_msg = vec![MSG_MATCH_END];
        end_msg.extend_from_slice(&winner_id.to_be_bytes());
        let name_bytes = winner_name.as_bytes();
        end_msg.push(name_bytes.len() as u8);
        end_msg.extend_from_slice(name_bytes);
        for p in players.values() {
            let _ = socket.send_to(&end_msg, p.socket_addr);
        }

        *match_end_pending = Some((
            Instant::now() + Duration::from_secs(game::MATCH_END_COUNTDOWN_SECS as u64),
            winner_id,
            winner_name,
        ));
    }
}

/// Update the authoritative match timer and check for timeout-based end.
pub fn update_match_timer(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    maze: &mut Option<MazeData>,
    match_running: &mut bool,
    match_start_time: &mut Option<Instant>,
    last_match_time_sent: &mut Option<u8>,
    match_total_maze_entries: &mut usize,
) {
    if let Some(start) = match_start_time {
        let elapsed = Instant::now().duration_since(*start);

        // send authoritative remaining-time to clients (only when value changes)
        let remaining_secs = if elapsed >= game::MATCH_DURATION {
            0u8
        } else {
            (game::MATCH_DURATION - elapsed).as_secs().min(255) as u8
        };
        if *last_match_time_sent != Some(remaining_secs) {
            let _ = broadcast::broadcast_match_timer(socket, players, remaining_secs);
            *last_match_time_sent = Some(remaining_secs);
        }

        // timeout -> move players to lobby
        if elapsed >= game::MATCH_DURATION {
            println!("Match duration elapsed — returning players to Lobby");
            broadcast::move_all_players_to_lobby(socket, players, false);
            *match_running = false;
            *match_start_time = None;
            *maze = None;
            *last_match_time_sent = None;
            *match_total_maze_entries = 0;
        }
    }
}

/// Execute the previously scheduled match end (after countdown expires).
pub fn execute_scheduled_match_end(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    maze: &mut Option<MazeData>,
    match_running: &mut bool,
    match_start_time: &mut Option<Instant>,
    match_end_pending: &mut Option<(Instant, u32, String)>,
    last_match_time_sent: &mut Option<u8>,
    match_total_maze_entries: &mut usize,
    winner_name: String,
) {
    println!(
        "Match end countdown expired — returning players to Lobby (winner {})",
        winner_name
    );

    // Move all players back to Lobby, reset match state and clear maze
    broadcast::move_all_players_to_lobby(socket, players, true);

    // clear server-side maze and match flags
    *maze = None;
    *match_running = false;
    *match_start_time = None;

    // clear pending flag
    *match_end_pending = None;
    *last_match_time_sent = None;
    *match_total_maze_entries = 0;
}
