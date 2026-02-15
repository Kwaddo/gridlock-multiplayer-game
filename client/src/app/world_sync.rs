use std::net::UdpSocket;

use macroquad::prelude::*;

use client::net::poll_updates;
use client::screens::PlayingState;
use client::types::maze::Maze;
use client::types::player::PlayerSkin;

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
        if let Some(mut players) = updates.players {
            for p in &mut players {
                p.skin_type = PlayerSkin::RifleIdle;
            }

            let mut existing = std::mem::take(&mut playing_state.other_players);
            let mut merged = Vec::with_capacity(players.len());

            for mut incoming in players {
                if let Some(index) = existing.iter().position(|p| p.id == incoming.id) {
                    let mut current = existing.swap_remove(index);
                    current.username = incoming.username;
                    current.world = incoming.world;
                    current.skin_type = incoming.skin_type;
                    current.apply_snapshot(incoming.x, incoming.y, incoming.z, incoming.yaw);
                    merged.push(current);
                } else {
                    incoming.apply_snapshot(incoming.x, incoming.y, incoming.z, incoming.yaw);
                    merged.push(incoming);
                }
            }

            playing_state.other_players = merged;
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
                playing_state
                    .kill_feed
                    .push(client::screens::playing::KillFeedEntry {
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
