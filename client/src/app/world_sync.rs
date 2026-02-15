use std::net::UdpSocket;

use client::net::poll_updates;
use client::screens::playing::KillFeedEntry;
use client::screens::PlayingState;
use client::types::maze::Maze;
use macroquad::prelude::*;

pub fn sync_world(
    socket: &Option<UdpSocket>,
    player_id: u32,
    playing_state: &mut PlayingState,
    maze: &mut Option<Maze>,
) -> Option<Vec<(String, String)>> {
    if let Some(ref sock) = socket {
        let updates = poll_updates(sock, player_id);
        if let Some((new_maze, _spawn)) = updates.maze {
            *maze = Some(new_maze);
        }
        if let Some(players) = updates.players {
            playing_state.merge_other_players(players);
        }

        if let Some((world_id, spawn)) = updates.world_change {
            playing_state.current_world = world_id;
            playing_state.position = spawn;
            playing_state.movement_state.velocity = Vec3::ZERO;
        }
        if let Some(broken) = updates.broken_walls {
            if let Some(m) = maze.as_mut() {
                for (x, y) in broken {
                    m.force_break_wall(x, y);
                }
            }
        }
        if let Some((damage, attacker_id)) = updates.damage_taken {
            playing_state.take_damage(damage, attacker_id);
        }
        if let Some(kills) = updates.kill_feed {
            for (killer_id, victim_id, weapon) in kills {
                playing_state.kill_feed.push(KillFeedEntry {
                    killer_id,
                    victim_id,
                    weapon,
                    timer: 5.0,
                });
            }
        }
        if updates.death {
            playing_state.die(None);
        }
        if let Some(spawn_pos) = updates.respawn_pos {
            playing_state.respawn(spawn_pos);
        }
        return updates.assets;
    }
    None
}
