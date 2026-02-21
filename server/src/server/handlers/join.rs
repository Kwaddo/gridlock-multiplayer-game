use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::Instant;

use crate::maze::MazeData;
use crate::player::Player;
use crate::protocols::*;

use crate::server::{broadcast, utils};

/// Handler for an incoming MSG_JOIN packet.
pub fn handle_join(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    maze: Option<&MazeData>,
    match_running: bool,
    src_addr: SocketAddr,
    payload: &[u8],
    asset_manifest: Option<&serde_json::Value>,
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
    let assigned_username = utils::handle_username(players, &requested_username);
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
        utils::send_assets(socket, src_addr, asset_manifest);

        // broadcast updated connected-player list to everyone
        broadcast::broadcast_player_list(socket, players);
    }
}
