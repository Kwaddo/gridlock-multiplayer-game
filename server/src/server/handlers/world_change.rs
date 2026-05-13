use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

use crate::maze::MazeData;
use crate::player::Player;
use crate::player::WorldId;
use crate::protocols::*;

use crate::server::game;

/// Handler for world-change requests (lobby <-> maze).
pub fn handle_world_change(
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
        WorldId::Maze => {
            let Some(m) = maze else { return; };
            m.teleporter_world(game::MAZE_CELL_SIZE)
        }
    };
    player.position = (sx, sy, sz);

    log!("Player '{}' moved to {:?}", player.username, target);

    // send world-change reply using stack buffer
    let mut response = [0u8; 14];
    response[0] = MSG_WORLD_CHANGE;
    response[1] = target.as_u8();
    response[2..6].copy_from_slice(&sx.to_be_bytes());
    response[6..10].copy_from_slice(&sy.to_be_bytes());
    response[10..14].copy_from_slice(&sz.to_be_bytes());
    let _ = socket.send_to(&response, src_addr);

    // ensure the client has the maze data when transitioning into the Maze.
    if target == WorldId::Maze {
        if let Some(m) = maze {
            m.send(socket, src_addr);
        }
    }
}
