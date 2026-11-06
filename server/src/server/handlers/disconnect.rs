use std::collections::HashMap;
use crate::net::{Outbox, PeerId};

use crate::protocols::LEAVE_QUIT;
use crate::server::{broadcast, game, handlers};
use crate::player::Player;

/// Handler for MSG_DISCONNECT: remove player and inform others.
pub fn handle_disconnect(
    socket: &Outbox,
    players: &mut HashMap<PeerId, Player>,
    src_addr: PeerId,
) {
    if let Some(p) = players.remove(&src_addr) {
        log!(
            "Player '{}' disconnected. Total players: {}",
            p.username,
            players.len()
        );
        handlers::announce_left(socket, players, &p, LEAVE_QUIT);
        // re-balance lobby teams after player left
        game::auto_assign_lobby_teams(players);
        // broadcast state so other clients remove the character immediately
        broadcast::broadcast_state(socket, players);
        broadcast::broadcast_player_list(socket, players);
    }
}
