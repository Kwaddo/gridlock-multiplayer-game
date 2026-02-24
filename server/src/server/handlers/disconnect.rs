use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

use crate::server::{broadcast, game};
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
        // re-balance lobby teams after player left
        game::auto_assign_lobby_teams(players);
        // broadcast state so other clients remove the character immediately
        broadcast::broadcast_state(socket, players);
        broadcast::broadcast_player_list(socket, players);
    }
}
