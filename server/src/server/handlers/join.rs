use std::collections::HashMap;
use crate::net::{Outbox, PeerId};
use platform::Instant;

use crate::maze::MazeData;
use crate::player::Player;
use crate::protocols::*;

use crate::server::{broadcast, game, utils};

/// Handler for an incoming MSG_JOIN packet.
pub fn handle_join(
    socket: &Outbox,
    players: &mut HashMap<PeerId, Player>,
    maze: Option<&MazeData>,
    match_running: bool,
    src_addr: PeerId,
    requested_username: &str,
    asset_manifest: Option<&serde_json::Value>,
) {
    // Check if player already exists from this address
    if let Some(existing_player) = players.get_mut(&src_addr) {
        // Player is already connected, just update their activity and resend welcome
        existing_player.last_activity = Instant::now();
        let response = ServerMsg::Welcome {
            id: existing_player.id,
            username: existing_player.username.clone(),
        }
        .encode();
        let _ = socket.send_to(&response, src_addr);
        return;
    }

    let requested_username = requested_username.to_string();
    let assigned_username = utils::handle_username(players, &requested_username);
    let id = players.values().map(|p| p.id).max().unwrap_or(0) + 1;
    let new_player = Player::new(id, assigned_username.clone());

    log!(
        "Player '{}' connected (requested '{}'). Total players: {}",
        assigned_username, requested_username, id
    );
    players.insert(src_addr, new_player);

    let response = ServerMsg::Welcome { id, username: assigned_username.clone() }.encode();
    if socket.send_to(&response, src_addr).is_ok() {
        if match_running {
            if let Some(m) = maze {
                m.send(socket, src_addr);
            }
        }
        utils::send_assets(socket, src_addr, asset_manifest);

        // auto-assign balanced teams for all lobby players
        game::auto_assign_lobby_teams(players);
        // broadcast updated connected-player list to everyone
        broadcast::broadcast_player_list(socket, players);
    }
}
