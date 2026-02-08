use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

mod maze;
mod player;
mod protocols;

use maze::MazeData;
use player::Player;
use protocols::*;

const PLAYER_TIMEOUT: Duration = Duration::from_secs(30);

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
    println!("UDP server started on port 7878");

    let mut buffer: [u8; 512] = [0; 512];
    let mut players: HashMap<SocketAddr, Player> = HashMap::new();

    let maze = MazeData::new(21, 21);
    println!(
        "Maze generated: {}x{} with seed {}",
        maze.width, maze.height, maze.seed
    );

    loop {
        let (bytes_rcv, src_addr) = socket.recv_from(&mut buffer)?;
        let msg_type = buffer[0];
        let payload = &buffer[1..bytes_rcv];

        match msg_type {
            MSG_JOIN => handle_join(&socket, &mut players, &maze, src_addr, payload),
            MSG_DISCONNECT => handle_disconnect(&mut players, src_addr),
            MSG_POSITION => handle_position(&socket, &mut players, src_addr, payload),
            _ => println!("Unknown message type: {}", msg_type),
        }

        cleanup_inactive_players(&mut players);
    }
}

fn handle_join(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    maze: &MazeData,
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
        maze.send(socket, src_addr);
    }
}

fn handle_disconnect(players: &mut HashMap<SocketAddr, Player>, src_addr: SocketAddr) {
    if let Some(p) = players.remove(&src_addr) {
        println!(
            "Player '{}' disconnected. Total players: {}",
            p.username,
            players.len()
        );
    }
}

fn handle_position(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    src_addr: SocketAddr,
    payload: &[u8],
) {
    if payload.len() < 16 {
        return;
    }

    if let Some(player) = players.get_mut(&src_addr) {
        let x = f32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
        let y = f32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
        let z = f32::from_be_bytes([payload[8], payload[9], payload[10], payload[11]]);
        let yaw = f32::from_be_bytes([payload[12], payload[13], payload[14], payload[15]]);
        player.position = (x, y, z);
        player.yaw = yaw;
        player.last_activity = Instant::now();
    }

    broadcast_state(socket, players);
}

fn broadcast_state(socket: &UdpSocket, players: &HashMap<SocketAddr, Player>) {
    let mut state_msg = vec![MSG_STATE];
    state_msg.extend_from_slice(&(players.len() as u32).to_be_bytes());

    for player in players.values() {
        state_msg.extend_from_slice(&player.id.to_be_bytes());
        state_msg.extend_from_slice(&player.position.0.to_be_bytes());
        state_msg.extend_from_slice(&player.position.1.to_be_bytes());
        state_msg.extend_from_slice(&player.position.2.to_be_bytes());
        state_msg.extend_from_slice(&player.yaw.to_be_bytes());
        let name_bytes = player.username.as_bytes();
        state_msg.push(name_bytes.len() as u8);
        state_msg.extend_from_slice(name_bytes);
    }

    for player in players.values() {
        let _ = socket.send_to(&state_msg, player.socket_addr);
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
