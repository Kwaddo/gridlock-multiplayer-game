use std::collections::HashMap;
use std::fs;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

mod maze;
mod player;
mod protocols;

use maze::MazeData;
use player::Player;
use protocols::*;

use player::WorldId;

const PLAYER_TIMEOUT: Duration = Duration::from_secs(30);
const MAZE_CELL_SIZE: f32 = 1.8;
const MAZE_WIDTH: u8 = 35;
const MAZE_HEIGHT: u8 = 35;
const MAZE_AUTO_RESPAWN_DELAY: Duration = Duration::from_secs(3);

// Match / game loop settings (basic foundation)
const MATCH_DURATION: Duration = Duration::from_secs(120); // match length (seconds)
const MIN_PLAYERS_TO_START: usize = 1; // minimal players required to start (adjust later)
const MATCH_MAX_PLAYERS: usize = 16; // maximum concurrent players in a running match
const MATCH_SCORE_TO_WIN: u32 = 20; // kills required to win the match
const MATCH_START_COUNTDOWN_SECS: u8 = 5; // seconds shown to players before a match actually starts
const MATCH_END_COUNTDOWN_SECS: u8 = 6; // seconds to wait after a win before returning players to lobby
const MATCH_POST_TELEPORT_FREEZE_SECS: u64 = 10; // seconds players cannot move right after teleport into Maze

fn handle_username(players: &HashMap<SocketAddr, Player>, requested: &str) -> String {
    let mut candidate = if requested.is_empty() || requested.len() > 20 {
        println!("Invalid username '{}'. Assigning default.", requested);
        "Player".to_string()
    } else {
        requested.to_string()
    };

    if !players.values().any(|p| p.username == candidate) {
        return candidate;
    }

    let mut i = 0;
    loop {
        candidate = format!("quad{}", i);
        if !players.values().any(|p| p.username == candidate) {
            return candidate;
        }
        i += 1;
    }
}

fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:7878")?;
    socket.set_read_timeout(Some(Duration::from_millis(100)))?;
    println!("UDP server started on port 7878");

    let mut buffer: [u8; 512] = [0; 512];
    let mut players: HashMap<SocketAddr, Player> = HashMap::new();

    // Do not generate a full gameplay maze at server start — create per-match instead.
    let mut maze: Option<MazeData> = None; // Some(...) when a match is running

    // Basic match state for a future game loop
    let mut match_running: bool = false;
    let mut match_start_time: Option<Instant> = None;
    // warmup / countdown before match actually begins
    let mut match_warmup_end: Option<Instant> = None;
    // scheduled match end (winner, delayed return-to-lobby)
    let mut match_end_pending: Option<(Instant, u32, String)> = None;
    // short post-teleport freeze so players remain stationary for a few seconds after entering the Maze
    let mut match_post_teleport_freeze_end: Option<Instant> = None;

    loop {
        match socket.recv_from(&mut buffer) {
            Ok((bytes_rcv, src_addr)) => {
                let msg_type = buffer[0];
                let payload = &buffer[1..bytes_rcv];

                match msg_type {
                    MSG_JOIN => handle_join(
                        &socket,
                        &mut players,
                        maze.as_ref(),
                        match_running,
                        src_addr,
                        payload,
                    ),
                    MSG_DISCONNECT => handle_disconnect(&socket, &mut players, src_addr),
                    MSG_POSITION => handle_position(
                        &socket,
                        &mut players,
                        src_addr,
                        payload,
                        match_post_teleport_freeze_end.map_or(false, |t| t > Instant::now()),
                    ),

                    // client-requested world change (teleporter)
                    MSG_WORLD_CHANGE => {
                        handle_world_change(
                            &socket,
                            maze.as_ref(),
                            &mut players,
                            src_addr,
                            payload,
                        );
                    }

                    // client requests to start a match from the lobby
                    MSG_START_GAME => {
                        // If a match is marked running but there are no players in the Maze,
                        // treat it as stale and clear it so new starts are accepted.
                        if match_running {
                            let maze_count = players
                                .values()
                                .filter(|p| p.world == WorldId::Maze)
                                .count();
                            if maze_count == 0 {
                                println!(
                                    "Stale match detected (no players) — clearing match state"
                                );
                                match_running = false;
                                match_start_time = None;
                            }
                        }

                        // only accept start requests when not already running or scheduled
                        if !match_running
                            && match_warmup_end.is_none()
                            && match_end_pending.is_none()
                        {
                            // require minimal number of players in lobby
                            let lobby_count = players
                                .values()
                                .filter(|p| p.world == WorldId::Lobby)
                                .count();
                            if lobby_count >= MIN_PLAYERS_TO_START {
                                // schedule a warmup/countdown before the match starts so all clients
                                // can show a synchronized countdown UI.
                                match_warmup_end = Some(
                                    Instant::now()
                                        + Duration::from_secs(MATCH_START_COUNTDOWN_SECS as u64),
                                );

                                // broadcast countdown to all connected clients
                                let cd = vec![MSG_MATCH_COUNTDOWN, MATCH_START_COUNTDOWN_SECS];
                                for p in players.values() {
                                    let _ = socket.send_to(&cd, p.socket_addr);
                                }

                                println!(
                                    "Match start requested by {} — countdown {}s",
                                    src_addr, MATCH_START_COUNTDOWN_SECS
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
                            if maze_count < MATCH_MAX_PLAYERS {
                                if let Some(req_player) = players.get_mut(&src_addr) {
                                    if req_player.world == WorldId::Lobby {
                                        let current_maze = match &maze {
                                            Some(m) => m,
                                            None => {
                                                // shouldn't happen: match_running implies a maze exists
                                                let _ =
                                                    socket.send_to(&[MSG_START_DENIED], src_addr);
                                                continue;
                                            }
                                        };
                                        let (sx, sy, sz, _used_fallback) =
                                            current_maze.random_spawn_world(MAZE_CELL_SIZE);
                                        req_player.world = WorldId::Maze;
                                        req_player.position = (sx, sy, sz);

                                        let mut response =
                                            vec![MSG_WORLD_CHANGE, WorldId::Maze.as_u8()];
                                        response.extend_from_slice(&sx.to_be_bytes());
                                        response.extend_from_slice(&sy.to_be_bytes());
                                        response.extend_from_slice(&sz.to_be_bytes());
                                        let _ = socket.send_to(&response, req_player.socket_addr);

                                        // send maze data to the joining client
                                        current_maze.send(&socket, req_player.socket_addr);

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
                                println!(
                                    "Start-game request rejected: match already running and full."
                                );
                            }
                        }
                    }

                    MSG_WALL_BROKEN => {
                        if let Some(ref mut m) = maze {
                            handle_wall_broken(&socket, m, &players, src_addr, payload)
                        }
                    }
                    MSG_DISCOVER => handle_discover(&socket, &players, src_addr),
                    MSG_HIT => handle_hit(
                        &socket,
                        &mut players,
                        src_addr,
                        payload,
                        &mut maze,
                        &mut match_running,
                        &mut match_start_time,
                        &mut match_end_pending,
                    ),
                    MSG_RESPAWN_REQUEST => {
                        if let Some(ref m) = maze {
                            handle_respawn_request(&socket, m, &mut players, src_addr)
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
            process_auto_respawns(&socket, m, &mut players);
        }
        cleanup_inactive_players(&mut players);

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
            }
        }

        // Warmup expiry: actually start the match when the warmup countdown ends
        if match_warmup_end.is_some() && !match_running {
            if let Some(end) = match_warmup_end {
                if Instant::now() >= end {
                    // start the match now (generate maze, move lobby players into Maze)
                    maze = Some(MazeData::new(MAZE_WIDTH, MAZE_HEIGHT));
                    let current_maze = maze.as_ref().unwrap();
                    let lobby_count = players
                        .values()
                        .filter(|p| p.world == WorldId::Lobby)
                        .count();
                    println!(
                        "Match warmup complete — starting match with seed {} (moving {} players)",
                        current_maze.seed, lobby_count
                    );

                    match_running = true;
                    match_start_time = Some(Instant::now());
                    match_warmup_end = None;
                    // Start a short post-teleport freeze so players cannot move for a moment
                    match_post_teleport_freeze_end =
                        Some(Instant::now() + Duration::from_secs(MATCH_POST_TELEPORT_FREEZE_SECS));

                    for player in players.values_mut().filter(|p| p.world == WorldId::Lobby) {
                        let (sx, sy, sz, _used_fallback) =
                            current_maze.random_spawn_world(MAZE_CELL_SIZE);
                        player.world = WorldId::Maze;
                        player.position = (sx, sy, sz);

                        let mut response = vec![MSG_WORLD_CHANGE, WorldId::Maze.as_u8()];
                        response.extend_from_slice(&sx.to_be_bytes());
                        response.extend_from_slice(&sy.to_be_bytes());
                        response.extend_from_slice(&sz.to_be_bytes());
                        let _ = socket.send_to(&response, player.socket_addr);

                        current_maze.send(&socket, player.socket_addr);
                    }

                    // broadcast updated player list/state so clients update immediately
                    broadcast_state(&socket, &players);
                    broadcast_player_list(&socket, &players);
                }
            }
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
            }
        }

        // Basic match timer: end match after MATCH_DURATION
        if match_running {
            if let Some(start) = match_start_time {
                if Instant::now().duration_since(start) >= MATCH_DURATION {
                    println!("Match duration elapsed — returning players to Lobby");
                    // move all players back to lobby
                    for player in players.values_mut().filter(|p| p.world == WorldId::Maze) {
                        player.world = WorldId::Lobby;
                        player.position = (0.0, 0.0, 0.0);
                        let mut response = vec![MSG_WORLD_CHANGE, WorldId::Lobby.as_u8()];
                        response.extend_from_slice(&0.0f32.to_be_bytes());
                        response.extend_from_slice(&0.0f32.to_be_bytes());
                        response.extend_from_slice(&0.0f32.to_be_bytes());
                        let _ = socket.send_to(&response, player.socket_addr);
                    }

                    match_running = false;
                    match_start_time = None;
                    maze = None;

                    // broadcast final state and player-list so clients update immediately
                    broadcast_state(&socket, &players);
                    broadcast_player_list(&socket, &players);
                }
            }
        }

        // If a match-end was scheduled (winner reached score), execute it when the countdown expires
        if let Some((when, _, winner_name)) = match_end_pending.clone() {
            if Instant::now() >= when {
                println!(
                    "Match end countdown expired — returning players to Lobby (winner {})",
                    winner_name
                );

                // Move all players back to Lobby, reset match state and clear maze
                for p in players.values_mut() {
                    p.world = WorldId::Lobby;
                    p.position = (0.0, 0.0, 0.0);
                    p.is_alive = true;
                    p.health = p.max_health;
                    p.shield = 0;
                    p.kills = 0; // reset match score
                    p.death_time = None;

                    // notify client of world change
                    let mut wc = vec![MSG_WORLD_CHANGE, WorldId::Lobby.as_u8()];
                    wc.extend_from_slice(&0.0f32.to_be_bytes());
                    wc.extend_from_slice(&0.0f32.to_be_bytes());
                    wc.extend_from_slice(&0.0f32.to_be_bytes());
                    let _ = socket.send_to(&wc, p.socket_addr);
                }

                // clear server-side maze and match flags
                maze = None;
                match_running = false;
                match_start_time = None;

                // broadcast final state and player-list so clients update immediately
                broadcast_state(&socket, &players);
                broadcast_player_list(&socket, &players);

                // clear pending flag
                match_end_pending = None;
            }
        }
    }
}

fn process_auto_respawns(
    socket: &UdpSocket,
    maze: &MazeData,
    players: &mut HashMap<SocketAddr, Player>,
) {
    let now = Instant::now();
    // include a bool indicating whether the teleporter fallback was used
    let mut to_respawn: Vec<(SocketAddr, f32, f32, f32, bool, String)> = Vec::new();

    for player in players.values_mut() {
        if player.is_alive || player.world != WorldId::Maze {
            continue;
        }

        let Some(death_time) = player.death_time else {
            continue;
        };

        if now.duration_since(death_time) >= MAZE_AUTO_RESPAWN_DELAY {
            player.is_alive = true;
            player.health = player.max_health;
            player.shield = 0;
            player.death_time = None;

            // choose a random spawn inside the maze (may fallback to teleporter)
            let (sx, sy, sz, used_fallback) = maze.random_spawn_world(MAZE_CELL_SIZE);
            player.position = (sx, sy, sz);
            to_respawn.push((
                player.socket_addr,
                sx,
                sy,
                sz,
                used_fallback,
                player.username.clone(),
            ));
        }
    }

    for (socket_addr, sx, sy, sz, used_fallback, username) in to_respawn {
        println!(
            "Player '{}' auto-respawned in {:?}",
            username,
            WorldId::Maze
        );
        let mut respawn_msg = vec![MSG_RESPAWN];
        respawn_msg.extend_from_slice(&sx.to_be_bytes());
        respawn_msg.extend_from_slice(&sy.to_be_bytes());
        respawn_msg.extend_from_slice(&sz.to_be_bytes());
        // extra byte: 0 = random spawn, 1 = teleporter fallback
        respawn_msg.push(if used_fallback { 1u8 } else { 0u8 });
        let _ = socket.send_to(&respawn_msg, socket_addr);
    }
}

fn handle_join(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    maze: Option<&MazeData>,
    match_running: bool,
    src_addr: SocketAddr,
    payload: &[u8],
) {
    // Check if player already exists from this address
    if let Some(existing_player) = players.get_mut(&src_addr) {
        // Player is already connected, just update their activity and resend welcome
        existing_player.last_activity = Instant::now();
        let mut response = vec![MSG_WELCOME];
        response.extend_from_slice(&existing_player.id.to_be_bytes());
        let name_bytes = existing_player.username.as_bytes();
        response.push(name_bytes.len() as u8);
        response.extend_from_slice(name_bytes);
        let _ = socket.send_to(&response, src_addr);
        return;
    }

    let requested_username = String::from_utf8_lossy(payload).trim().to_string();
    let assigned_username = handle_username(players, &requested_username);
    let id = (players.len() as u32) + 1;
    let new_player = Player::new(id, assigned_username.clone(), src_addr);

    println!(
        "Player '{}' connected (requested '{}'). Total players: {}",
        assigned_username, requested_username, id
    );
    players.insert(src_addr, new_player);

    let mut response = vec![MSG_WELCOME];
    response.extend_from_slice(&id.to_be_bytes());
    let name_bytes = assigned_username.as_bytes();
    response.push(name_bytes.len() as u8);
    response.extend_from_slice(name_bytes);
    if socket.send_to(&response, src_addr).is_ok() {
        if match_running {
            if let Some(m) = maze {
                m.send(socket, src_addr);
            }
        }
        send_assets(socket, src_addr);

        // broadcast updated connected-player list to everyone
        broadcast_player_list(socket, players);
    }
}

fn handle_disconnect(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    src_addr: SocketAddr,
) {
    if let Some(p) = players.remove(&src_addr) {
        println!(
            "Player '{}' disconnected. Total players: {}",
            p.username,
            players.len()
        );
        // notify remaining clients about updated player list
        broadcast_player_list(socket, players);
    }
}

/// Send the full connected player list to a single destination
fn send_player_list(socket: &UdpSocket, players: &HashMap<SocketAddr, Player>, dest: SocketAddr) {
    // format: [MSG_PLAYER_LIST][count:1][entries...]
    // entry: [id:4][name_len:1][name bytes][kills:4][world:1][alive:1]
    let count = players.len().min(255) as u8;
    let mut msg = vec![MSG_PLAYER_LIST, count];
    for p in players.values() {
        msg.extend_from_slice(&p.id.to_be_bytes());
        let name_bytes = p.username.as_bytes();
        msg.push(name_bytes.len() as u8);
        msg.extend_from_slice(name_bytes);
        msg.extend_from_slice(&p.kills.to_be_bytes());
        msg.push(p.world.as_u8());
        msg.push(if p.is_alive { 1u8 } else { 0u8 });
    }
    let _ = socket.send_to(&msg, dest);
}

/// Broadcast updated player list to every connected client
fn broadcast_player_list(socket: &UdpSocket, players: &HashMap<SocketAddr, Player>) {
    for p in players.values() {
        send_player_list(socket, players, p.socket_addr);
    }
}

fn handle_position(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    src_addr: SocketAddr,
    payload: &[u8],
    freeze_active: bool,
) {
    if payload.len() < 16 {
        return;
    }

    // parse client-provided snapshot first
    let x = f32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let y = f32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
    let z = f32::from_be_bytes([payload[8], payload[9], payload[10], payload[11]]);
    let yaw = f32::from_be_bytes([payload[12], payload[13], payload[14], payload[15]]);

    if let Some(player) = players.get_mut(&src_addr) {
        // If a global post-teleport freeze is active, ignore movement updates from
        // clients that are in the Maze so they remain stationary (server-authoritative).
        if freeze_active && player.world == WorldId::Maze {
            // refresh activity so they don't time out, but do not accept the position update
            player.last_activity = Instant::now();
            return;
        }

        player.position = (x, y, z);
        player.yaw = yaw;
        player.last_activity = Instant::now();
    }

    if payload.len() >= 17 {
        let state_flags = payload[16];
        if let Some(player) = players.get_mut(&src_addr) {
            player.is_sprinting = (state_flags & FLAG_SPRINTING) != 0;
            player.is_crouched = (state_flags & FLAG_CROUCHING) != 0;
            player.is_ads = (state_flags & FLAG_ADS) != 0;
            player.is_airborne = (state_flags & FLAG_AIRBORNE) != 0;
        }
    }
    if payload.len() >= 18 {
        let event_flags = payload[17];
        if let Some(player) = players.get_mut(&src_addr) {
            player.evt_jump_start = (event_flags & EVT_JUMP_START) != 0;
            player.evt_landed = (event_flags & EVT_LANDED) != 0;
        }
    }

    broadcast_state(socket, players);
}

fn broadcast_state(socket: &UdpSocket, players: &HashMap<SocketAddr, Player>) {
    // updated this function to build a separate message per receiver filtered by world
    for receiver in players.values() {
        // Only include *alive* players in the per-receiver state snapshot so dead bodies
        // do not linger client-side as visible, shootable models.
        let same_world: Vec<&Player> = players
            .values()
            .filter(|p| p.world == receiver.world && p.is_alive)
            .collect();

        let mut state_msg = vec![MSG_STATE];
        state_msg.extend_from_slice(&(same_world.len() as u32).to_be_bytes());

        for player in &same_world {
            state_msg.extend_from_slice(&player.id.to_be_bytes());
            state_msg.extend_from_slice(&player.position.0.to_be_bytes());
            state_msg.extend_from_slice(&player.position.1.to_be_bytes());
            state_msg.extend_from_slice(&player.position.2.to_be_bytes());
            state_msg.extend_from_slice(&player.yaw.to_be_bytes());
            state_msg.push(player.world.as_u8());
            let mut state_flags = 0u8;
            if player.is_sprinting {
                state_flags |= FLAG_SPRINTING;
            }
            if player.is_crouched {
                state_flags |= FLAG_CROUCHING;
            }
            if player.is_ads {
                state_flags |= FLAG_ADS;
            }
            if player.is_airborne {
                state_flags |= FLAG_AIRBORNE;
            }
            state_msg.push(state_flags);

            let mut event_flags = 0u8;
            if player.evt_jump_start {
                event_flags |= EVT_JUMP_START;
            }
            if player.evt_landed {
                event_flags |= EVT_LANDED;
            }
            state_msg.push(event_flags);

            // include kills (score) so clients can render scoreboard
            state_msg.extend_from_slice(&player.kills.to_be_bytes());

            let name_bytes = player.username.as_bytes();
            state_msg.push(name_bytes.len() as u8);
            state_msg.extend_from_slice(name_bytes);
        }

        let _ = socket.send_to(&state_msg, receiver.socket_addr);
    }
}

fn cleanup_inactive_players(players: &mut HashMap<SocketAddr, Player>) {
    let now = Instant::now();
    let mut to_remove = Vec::new();

    for (addr, player) in players.iter() {
        if now.duration_since(player.last_activity) > PLAYER_TIMEOUT {
            to_remove.push(*addr);
        }
    }

    for addr in to_remove {
        if let Some(p) = players.remove(&addr) {
            println!(
                "Player '{}' timed out. Total players: {}",
                p.username,
                players.len()
            );
        }
    }
}

fn handle_world_change(
    socket: &UdpSocket,
    maze: Option<&MazeData>,
    players: &mut HashMap<SocketAddr, Player>,
    src_addr: SocketAddr,
    payload: &[u8],
) {
    if payload.is_empty() {
        return;
    }
    let Some(target) = WorldId::from_u8(payload[0]) else {
        return;
    };
    let Some(player) = players.get_mut(&src_addr) else {
        return;
    };

    // If the player requests to move into the Maze but no maze exists, ignore the request.
    if target == WorldId::Maze && maze.is_none() {
        return;
    }

    player.world = target;

    let (sx, sy, sz) = match target {
        WorldId::Lobby => (0.0_f32, 0.0_f32, 0.0_f32),
        WorldId::Maze => maze.unwrap().teleporter_world(MAZE_CELL_SIZE),
    };
    player.position = (sx, sy, sz);

    println!("Player '{}' moved to {:?}", player.username, target);

    let mut response = vec![MSG_WORLD_CHANGE, target.as_u8()];
    response.extend_from_slice(&sx.to_be_bytes());
    response.extend_from_slice(&sy.to_be_bytes());
    response.extend_from_slice(&sz.to_be_bytes());
    let _ = socket.send_to(&response, src_addr);
}

fn handle_discover(
    socket: &UdpSocket,
    players: &HashMap<SocketAddr, Player>,
    src_addr: SocketAddr,
) {
    let player_count = players.len().min(255) as u8;
    let hostname = fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "Unknown".to_string());
    let name = format!("{}'s Server", hostname);
    let name_bytes = name.as_bytes();
    let mut response = vec![
        MSG_DISCOVER,
        player_count,
        MATCH_MAX_PLAYERS as u8,
        name_bytes.len() as u8,
    ];
    response.extend_from_slice(name_bytes);
    let _ = socket.send_to(&response, src_addr);
}

fn send_assets(socket: &UdpSocket, addr: SocketAddr) {
    let Ok(data) = fs::read_to_string("assets/manifest.json") else {
        println!("Warning: could not read assets/manifest.json");
        return;
    };
    let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&data) else {
        println!("Warning: invalid assets/manifest.json");
        return;
    };

    let worlds = [("global", 255u8), ("lobby", 0u8), ("maze", 1u8)];

    for (world_name, world_id) in &worlds {
        if let Some(assets) = manifest.get(world_name).and_then(|v| v.as_object()) {
            if assets.is_empty() {
                continue;
            }
            let mut msg = vec![MSG_ASSETS, *world_id, assets.len() as u8];
            for (key, path) in assets {
                if let Some(path_str) = path.as_str() {
                    let key_bytes = key.as_bytes();
                    let path_bytes = path_str.as_bytes();
                    msg.push(key_bytes.len() as u8);
                    msg.extend_from_slice(key_bytes);
                    msg.push(path_bytes.len() as u8);
                    msg.extend_from_slice(path_bytes);
                }
            }
            println!(
                "Sending {} asset(s) for world '{}'",
                assets.len(),
                world_name
            );
            let _ = socket.send_to(&msg, addr);
        }
    }
}

fn handle_wall_broken(
    socket: &UdpSocket,
    maze: &mut MazeData,
    players: &HashMap<SocketAddr, Player>,
    src_addr: SocketAddr,
    payload: &[u8],
) {
    if payload.len() < 2 {
        return;
    }

    let x = payload[0];
    let y = payload[1];

    if x >= maze.width || y >= maze.height {
        return;
    }

    if !maze.break_wall(x, y) {
        return;
    }

    let msg = vec![MSG_WALL_BROKEN, x, y];
    for player in players.values() {
        if player.socket_addr != src_addr {
            let _ = socket.send_to(&msg, player.socket_addr);
        }
    }
}

fn handle_hit(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    src_addr: SocketAddr,
    payload: &[u8],
    _maze: &mut Option<MazeData>,
    _match_running: &mut bool,
    _match_start_time: &mut Option<Instant>,
    match_end_pending: &mut Option<(Instant, u32, String)>,
) {
    // Payload: victim_id (4 bytes), damage (4 bytes)
    if payload.len() < 8 {
        return;
    }

    let victim_id = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let damage = f32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);

    // Get attacker
    let Some(attacker) = players.get(&src_addr) else {
        return;
    };

    // Ignore hits originating outside the Maze (no PvP in lobby)
    if attacker.world != WorldId::Maze {
        return;
    }

    if !attacker.is_alive {
        return; // Dead players can't deal damage
    }

    let attacker_id = attacker.id;

    // Find victim by ID
    let Some(victim_addr) = players
        .iter()
        .find(|(_, p)| p.id == victim_id)
        .map(|(addr, _)| *addr)
    else {
        return;
    };

    let Some(victim) = players.get_mut(&victim_addr) else {
        return;
    };

    // Only allow damage inside the Maze world
    if victim.world != WorldId::Maze {
        return;
    }

    if !victim.is_alive {
        return; // Already dead
    }

    // Apply damage (shield first, then health)
    let damage_u32 = damage as u32;
    let shield_damage = damage_u32.min(victim.shield);
    victim.shield -= shield_damage;
    let remaining_damage = damage_u32 - shield_damage;

    if remaining_damage > 0 {
        victim.health = victim.health.saturating_sub(remaining_damage);
    }

    println!(
        "Player {} hit player {} for {} damage (HP: {}, Shield: {})",
        attacker_id, victim_id, damage, victim.health, victim.shield
    );

    // Send MSG_DAMAGE to victim
    let mut damage_msg = vec![MSG_DAMAGE];
    damage_msg.extend_from_slice(&damage.to_be_bytes());
    damage_msg.extend_from_slice(&attacker_id.to_be_bytes());
    let _ = socket.send_to(&damage_msg, victim_addr);

    // Check if victim died
    if victim.health == 0 {
        victim.is_alive = false;
        victim.deaths += 1;
        victim.death_time = Some(Instant::now());

        println!("Player {} killed player {}", attacker_id, victim_id);

        // Send MSG_DEATH to victim
        let mut death_msg = vec![MSG_DEATH];
        death_msg.extend_from_slice(&attacker_id.to_be_bytes());
        let _ = socket.send_to(&death_msg, victim_addr);

        // Send MSG_KILL to attacker and increment their kill count
        let mut attacker_won_match = false;
        if let Some(attacker) = players.get_mut(&src_addr) {
            attacker.kills += 1;
            // Check for match victory
            if attacker.kills >= MATCH_SCORE_TO_WIN && *_match_running {
                attacker_won_match = true;
            }
            let mut kill_msg = vec![MSG_KILL];
            kill_msg.extend_from_slice(&victim_id.to_be_bytes());
            let _ = socket.send_to(&kill_msg, src_addr);
        }

        // Broadcast kill feed to all players
        let weapon_name = "Rifle"; // TODO: Get actual weapon from payload
        let weapon_bytes = weapon_name.as_bytes();
        let mut kill_feed_msg = vec![MSG_KILL_FEED];
        kill_feed_msg.extend_from_slice(&attacker_id.to_be_bytes());
        kill_feed_msg.extend_from_slice(&victim_id.to_be_bytes());
        kill_feed_msg.push(weapon_bytes.len() as u8);
        kill_feed_msg.extend_from_slice(weapon_bytes);
        for player in players.values() {
            let _ = socket.send_to(&kill_feed_msg, player.socket_addr);
        }

        // Immediately broadcast an updated state so the killed player's model is removed
        // from other clients without waiting for the next position update.
        broadcast_state(socket, players);

        // also broadcast player-list so scoreboard UI updates immediately
        broadcast_player_list(socket, players);

        // If the attacker reached the score to win, end the match now.
        if attacker_won_match {
            // find winner (attacker) to announce
            let winner_name = players
                .get(&src_addr)
                .map(|p| p.username.clone())
                .unwrap_or_else(|| "Unknown".to_string());
            println!(
                "Match won by '{}' ({} kills) — ending match",
                winner_name, MATCH_SCORE_TO_WIN
            );

            // Broadcast MSG_MATCH_END (winner_id, name)
            let mut end_msg = vec![MSG_MATCH_END];
            end_msg.extend_from_slice(&attacker_id.to_be_bytes());
            let name_bytes = winner_name.as_bytes();
            end_msg.push(name_bytes.len() as u8);
            end_msg.extend_from_slice(name_bytes);
            for p in players.values() {
                let _ = socket.send_to(&end_msg, p.socket_addr);
            }

            // schedule server-side delayed match end (clients received MSG_MATCH_END with countdown)
            *match_end_pending = Some((
                Instant::now() + Duration::from_secs(MATCH_END_COUNTDOWN_SECS as u64),
                attacker_id,
                winner_name.clone(),
            ));

            // NOTE: actual move-to-lobby will be executed by the main loop when the countdown expires
            // (below). We still broadcast current state so clients immediately see final scores.
        }
    }
}

fn handle_respawn_request(
    socket: &UdpSocket,
    maze: &MazeData,
    players: &mut HashMap<SocketAddr, Player>,
    src_addr: SocketAddr,
) {
    let Some(player) = players.get_mut(&src_addr) else {
        return;
    };

    // Reset player state
    player.is_alive = true;
    player.health = player.max_health;
    player.shield = 0;
    player.death_time = None;

    // Choose spawn position based on world. For Maze, prefer random spawn.
    let (sx, sy, sz, used_fallback) = match player.world {
        WorldId::Lobby => (0.0_f32, 0.0_f32, 0.0_f32, false),
        WorldId::Maze => maze.random_spawn_world(MAZE_CELL_SIZE),
    };
    player.position = (sx, sy, sz);

    println!(
        "Player '{}' respawned in {:?}",
        player.username, player.world
    );

    // Send MSG_RESPAWN with spawn position + spawn-kind flag
    let mut respawn_msg = vec![MSG_RESPAWN];
    respawn_msg.extend_from_slice(&sx.to_be_bytes());
    respawn_msg.extend_from_slice(&sy.to_be_bytes());
    respawn_msg.extend_from_slice(&sz.to_be_bytes());
    respawn_msg.push(if used_fallback { 1u8 } else { 0u8 });
    let _ = socket.send_to(&respawn_msg, src_addr);

    // Immediately notify other clients so the player reappears for others right away.
    broadcast_state(socket, players);
}
