use std::collections::HashMap;
use crate::net::{Outbox, PeerId};

use crate::maze::MazeData;
use crate::player::Player;
use crate::player::WorldId;
use crate::protocols::*;

use crate::server::game;

/// Handler for world-change requests (lobby <-> maze).
pub fn handle_world_change(
    socket: &Outbox,
    maze: Option<&MazeData>,
    players: &mut HashMap<PeerId, Player>,
    src_addr: PeerId,
    target: u8,
) {
    let Some(target) = WorldId::from_u8(target) else {
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

    let response = ServerMsg::WorldChange {
        world: target.as_u8(),
        spawn: (sx, sy, sz),
        mode: None,
        freeze_secs: None,
        match_timer: None,
    }
    .encode();
    let _ = socket.send_to(&response, src_addr);

    // ensure the client has the maze data when transitioning into the Maze.
    if target == WorldId::Maze {
        if let Some(m) = maze {
            m.send(socket, src_addr);
        }
    }
}
