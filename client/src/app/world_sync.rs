use std::net::UdpSocket;

use client::types::world::WorldId;
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
                    // preserve updated kills
                    current.kills = incoming.kills;
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

        // Update local player's kill count when server provides it via MSG_STATE
        if let Some(k) = updates.self_kills {
            playing_state.local_kills = k;
        }

        // Handle match end broadcast
        let match_end_winner_cloned = updates.match_end_winner.clone();
        if let Some(winner_name) = match_end_winner_cloned {
            // move client back to lobby view and clear local maze
            playing_state.announcements.push((
                format!("{} won the match! Returning to lobby.", winner_name),
                6.0,
            ));
            playing_state.current_world = WorldId::Lobby;
            *maze = None;
            playing_state.local_kills = 0;
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

        // Match start countdown (server-sent) — show large center countdown
        if let Some(secs) = updates.match_start_countdown {
            playing_state.match_start_countdown_timer = secs as f32;
        }

        // Match-join announcement broadcast (show bottom-left)
        if let Some(msg) = updates.match_join_announce {
            playing_state.announcements.push((msg, 4.0));
        }

        // Match-end winner + countdown (server-sent). Do NOT change world locally; server will send WorldChange when it executes the return.
        if let Some(winner) = updates.match_end_winner {
            playing_state.match_end_winner = Some(winner.clone());
            if let Some(cd) = updates.match_end_countdown {
                playing_state.match_end_countdown_timer = cd as f32;
                playing_state.announcements.push((
                    format!("{} won the match! Returning to lobby in {}s", winner, cd),
                    playing_state.match_end_countdown_timer.max(4.0),
                ));
            } else {
                playing_state.match_end_countdown_timer = 4.0;
                playing_state.announcements.push((
                    format!("{} won the match! Returning to lobby.", winner),
                    4.0,
                ));
            }
            playing_state.local_kills = 0;
        }

        // Global connected player list (for Tab / scoreboard)
        if let Some(list) = updates.player_list {
            playing_state.connected_players = list
                .into_iter()
                .map(|(id, name, kills, _world, _alive)| (id, name, kills))
                .collect();
        }

        return updates.assets;
    }
    None
}
