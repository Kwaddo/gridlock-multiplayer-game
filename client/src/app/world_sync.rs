use std::net::UdpSocket;

use macroquad::prelude::*;

use client::net::poll_updates;
use client::screens::PlayingState;
use client::types::animation_state::AnimState;
use client::types::maze::Maze;

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
            let mut existing = std::mem::take(&mut playing_state.other_players);
            let mut merged = Vec::with_capacity(players.len());

            for mut incoming in players {
                if let Some(index) = existing.iter().position(|p| p.id == incoming.id) {
                    let mut current = existing.swap_remove(index);
                    let anim_state = AnimState::from_movement(
                        current.x,
                        current.z,
                        incoming.x,
                        incoming.z,
                        incoming.yaw,
                        incoming.is_sprinting,
                        incoming.is_ads,
                        incoming.is_crouching,
                        false,
                        false,
                        incoming.evt_jump_start,
                    );
                    current.username = incoming.username;
                    current.world = incoming.world;
                    current.skin_type = AnimState::skin_from_state(anim_state, None);
                    current.apply_snapshot(incoming.x, incoming.y, incoming.z, incoming.yaw);
                    merged.push(current);
                } else {
                    let anim_state = AnimState::from_movement(
                        incoming.x,
                        incoming.z,
                        incoming.x,
                        incoming.z,
                        incoming.yaw,
                        incoming.is_sprinting,
                        incoming.is_ads,
                        incoming.is_crouching,
                        false,
                        false,
                        incoming.evt_jump_start,
                    );
                    incoming.skin_type = AnimState::skin_from_state(anim_state, None);
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
            if let Some(fallback) = updates.respawn_fallback {
                if fallback {
                    playing_state.spawn_feedback_message =
                        Some("Respawned at teleporter (fallback)".to_string());
                    playing_state.spawn_feedback_timer = 3.0;
                }
            }
        }

        // Lobby console feedback from server (e.g. start rejected because match full)
        if let Some(msg) = updates.lobby_console_message {
            playing_state.lobby_console_message = Some(msg);
            playing_state.lobby_console_message_timer = 3.0;
        }

        // Match-join announcement broadcast (show bottom-left)
        if let Some(msg) = updates.match_join_announce {
            playing_state.announcements.push((msg, 4.0));
        }
        return updates.assets;
    }
    None
}
