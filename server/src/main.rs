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
            _ => {
                println!("Unknown message type: {}", msg_type);
            }
        }
    }
}
