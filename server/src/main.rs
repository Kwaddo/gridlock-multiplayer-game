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

    let mut maze = MazeData::new(35, 35);
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
            MSG_WORLD_CHANGE => {
                handle_world_change(&socket, &maze, &mut players, src_addr, payload);
            }
            MSG_WALL_BROKEN => handle_wall_broken(&socket, &mut maze, &players, src_addr, payload),
            MSG_DISCOVER => handle_discover(&socket, &players, src_addr),
            MSG_HIT => handle_hit(&socket, &mut players, src_addr, payload),
            MSG_RESPAWN_REQUEST => handle_respawn_request(&socket, &maze, &mut players, src_addr),
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
        send_assets(socket, src_addr);
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
    // updated this function to build a separate message per receiver filtered by world
    for receiver in players.values() {
        let same_world: Vec<&Player> = players
            .values()
            .filter(|p| p.world == receiver.world)
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
    maze: &MazeData,
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

    player.world = target;

    let (sx, sy, sz) = match target {
        WorldId::Lobby => (0.0_f32, 0.0_f32, 0.0_f32),
        WorldId::Maze => maze.teleporter_world(MAZE_CELL_SIZE),
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
    let max_players: u8 = 16;
    let hostname = fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "Unknown".to_string());
    let name = format!("{}'s Server", hostname);
    let name_bytes = name.as_bytes();
    let mut response = vec![
        MSG_DISCOVER,
        player_count,
        max_players,
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

    if !attacker.is_alive {
        return; // Dead players can't deal damage
    }

    let attacker_id = attacker.id;

    // Find victim by ID
    let Some(victim_addr) = players.iter().find(|(_, p)| p.id == victim_id).map(|(addr, _)| *addr) else {
        return;
    };

    let Some(victim) = players.get_mut(&victim_addr) else {
        return;
    };

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

        println!("Player {} killed player {}", attacker_id, victim_id);

        // Send MSG_DEATH to victim
        let mut death_msg = vec![MSG_DEATH];
        death_msg.extend_from_slice(&attacker_id.to_be_bytes());
        let _ = socket.send_to(&death_msg, victim_addr);

        // Send MSG_KILL to attacker
        if let Some(attacker) = players.get_mut(&src_addr) {
            attacker.kills += 1;
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

    // Choose spawn position based on world
    let (sx, sy, sz) = match player.world {
        WorldId::Lobby => (0.0_f32, 0.0_f32, 0.0_f32),
        WorldId::Maze => maze.teleporter_world(MAZE_CELL_SIZE),
    };
    player.position = (sx, sy, sz);

    println!("Player '{}' respawned in {:?}", player.username, player.world);

    // Send MSG_RESPAWN with spawn position
    let mut respawn_msg = vec![MSG_RESPAWN];
    respawn_msg.extend_from_slice(&sx.to_be_bytes());
    respawn_msg.extend_from_slice(&sy.to_be_bytes());
    respawn_msg.extend_from_slice(&sz.to_be_bytes());
    let _ = socket.send_to(&respawn_msg, src_addr);
}
