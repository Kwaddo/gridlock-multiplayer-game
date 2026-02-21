use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

use crate::server::broadcast;
use crate::player::Player;

/// Handler for MSG_DISCONNECT: remove player and inform others.
pub fn handle_disconnect(
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
        broadcast::broadcast_player_list(socket, players);
    }
}
