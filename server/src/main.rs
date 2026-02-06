use std::net::UdpSocket;
use std::collections::HashMap;
use std::net::SocketAddr;
mod player;
mod protocols;

use player::Player;
use protocols::*;

fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:7878").expect("Failed to bind socket");
    println!("UDP server started on port 7878");

    let mut buffer: [u8; 512] = [0; 512];
    let mut players: HashMap<SocketAddr, Player> = HashMap::new();

    loop {
        let (bytes_rcv, src_addr) = socket.recv_from(&mut buffer)?;
        let msg_type = buffer[0];
        let payload = &buffer[1..bytes_rcv];

        match msg_type {
            MSG_JOIN => {
                let username = String::from_utf8_lossy(payload).trim().to_string();
                let id = (players.len() as u32) + 1;
                let new_player = Player::new(id, username.clone(), src_addr);

                println!("Player '{}' connected. Total players: {}", username, id);
                players.insert(src_addr, new_player);

                let mut response = vec![MSG_WELCOME];
                response.extend_from_slice(&id.to_be_bytes());
                socket.send_to(&response, src_addr)?;
            }
            MSG_DISCONNECT => {
                if let Some(p) = players.remove(&src_addr) {
                    println!("Player '{}' disconnected. Total players: {}", p.username, players.len());
                }
            }
            MSG_POSITION => {
                if let Some(player) = players.get_mut(&src_addr) { // get player if exists
                    // parse position update
                    // payload is 16 bytes: x(4) + y(4) + z(4) + yaw(4)
                    let x = f32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
                    let y = f32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
                    let z = f32::from_be_bytes([payload[8], payload[9], payload[10], payload[11]]);
                    let yaw = f32::from_be_bytes([payload[12], payload[13], payload[14], payload[15]]);
                    player.position = (x, y, z);
                    player.yaw = yaw;
                }

                // broadcast all player positions to everyone
                // format: [MSG_STATE][player_count(4)][for each: id(4) + x(4) + y(4) + z(4) + yaw(4) + name_len(1) + name]
                let mut state_msg = vec![MSG_STATE];
                state_msg.extend_from_slice(&(players.len() as u32).to_be_bytes());

                for player in players.values() {
                    state_msg.extend_from_slice(&player.id.to_be_bytes());
                    state_msg.extend_from_slice(&player.position.0.to_be_bytes()); // x
                    state_msg.extend_from_slice(&player.position.1.to_be_bytes()); // y
                    state_msg.extend_from_slice(&player.position.2.to_be_bytes()); // z
                    state_msg.extend_from_slice(&player.yaw.to_be_bytes());
                    let name_bytes = player.username.as_bytes();
                    state_msg.push(name_bytes.len() as u8);
                    state_msg.extend_from_slice(name_bytes);
                }

                for player in players.values() {
                    let _ = socket.send_to(&state_msg, player.socket_addr);
                }
            }
            _ => {
                println!("Unknown message type: {}", msg_type);
            }
        }
    }
}
