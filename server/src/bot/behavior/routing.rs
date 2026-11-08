//  Routing and enemy lookup helpers

use std::collections::HashMap;
use crate::net::PeerId;
use crate::player::{Player, WorldId};
use crate::maze::MazeData;
use crate::maze::pathfinding::{ astar, world_to_cell };
use crate::server::game::MAZE_CELL_SIZE;

/// `p` is someone `bot` fights: alive, in the maze, on the other team.
pub(super) fn is_enemy(bot: &Player, p: &Player) -> bool {
    p.is_alive && p.world == WorldId::Maze && bot.is_hostile_to(p)
}

pub(super) fn find_enemy<'a>(players: &'a HashMap<PeerId, Player>, bot: &Player, id: u32) -> Option<&'a Player> {
    players.values().find(|p| p.id == id && is_enemy(bot, p))
}

/// A* from `from` to `to`, without `from` itself: the bot is already in that
/// cell, and walking back to its centre first makes it stutter on every repath.
pub(super) fn route(nav_cells: &[u8], maze: &MazeData, from: (u8, u8), to: (u8, u8)) -> Option<Vec<(u8, u8)>> {
    let mut path = astar(nav_cells, maze.width, maze.height, from, to)?;
    if path.len() > 1 {
        path.remove(0);
    }
    Some(path)
}

pub(super) fn route_to_pos(
    nav_cells: &[u8],
    maze: &MazeData,
    from: (u8, u8),
    pos: (f32, f32, f32),
) -> Option<Vec<(u8, u8)>> {
    let goal = world_to_cell(pos.0, pos.2, MAZE_CELL_SIZE, &maze.cells, maze.width, maze.height)?;
    route(nav_cells, maze, from, goal)
}

pub(super) fn route_to_player(
    nav_cells: &[u8],
    maze: &MazeData,
    from: (u8, u8),
    player: &Player,
) -> Option<Vec<(u8, u8)>> {
    route_to_pos(nav_cells, maze, from, player.position)
}

pub(super) fn dist2(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dz = a.2 - b.2;
    dx * dx + dz * dz
}
