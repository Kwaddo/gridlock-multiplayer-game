use std::net::UdpSocket;

use client::types::world::WorldId;
use client::ui::OtherPlayer;
use macroquad::prelude::*;

use client::net::poll_updates;
use client::screens::PlayingState;
use client::types::animation_state::AnimState;
use client::types::maze::Maze;

pub struct SyncResult {
    pub assets: Option<Vec<(String, String)>>,
    pub server_seen: bool, // true if we received at least one packet during poll
    pub removed_by_server: bool, // true if server no longer lists our player id (timed-out/removed)
}

pub fn sync_world(
    socket: &Option<UdpSocket>,
    player_id: u32,
    playing_state: &mut PlayingState,
    maze: &mut Option<Maze>,
) -> SyncResult {
    let mut result = SyncResult {
        assets: None,
        server_seen: false,
        removed_by_server: false,
    };

    if let Some(ref sock) = socket {
        let updates = poll_updates(sock, player_id);

        // mark that we saw traffic from server during this poll
        result.server_seen = updates.received_any;

        if let Some((new_maze, _spawn)) = updates.maze {
            *maze = Some(new_maze);
        }
        if let Some(players) = updates.players {
            let mut existing = std::mem::take(&mut playing_state.other_players);
            let mut merged = Vec::with_capacity(players.len());

            for mut incoming in players {
                if let Some(index) = existing.iter().position(|p| p.id == incoming.id) {
                    let mut current = existing.swap_remove(index);
                    // treat sprinting with a short grace window to avoid walk/sprint flicker
                    let effective_sprinting =
                        incoming.is_sprinting || current.is_sprinting || current.sprint_grace > 0.0;
                    let mut anim_state = AnimState::from_movement(
                        current.x,
                        current.z,
                        incoming.x,
                        incoming.z,
                        incoming.yaw,
                        effective_sprinting,
                        incoming.is_ads,
                        incoming.is_crouching,
                        false,
                        false,
                        incoming.evt_jump_start,
                    );

                    // Hysteresis: avoid rapid forward/side animation flips by requiring a short
                    // confirmation window before accepting a new MoveDir.
                    let candidate_dir = anim_state.move_dir;
                    if candidate_dir != current.last_move_dir {
                        // start or refresh a pending candidate — don't switch visuals yet
                        if current.pending_move_dir == Some(candidate_dir) {
                            // pending already in progress; leave it
                        } else {
                            current.pending_move_dir = Some(candidate_dir);
                            current.pending_move_timer = OtherPlayer::MOVE_DIR_GRACE;
                        }
                        anim_state.move_dir = current.last_move_dir;
                    } else {
                        // candidate matches current direction — accept immediately and clear pending
                        current.pending_move_dir = None;
                        current.pending_move_timer = 0.0;
                        current.last_move_dir = candidate_dir;
                        anim_state.move_dir = candidate_dir;
                    }

                    current.username = incoming.username;
                    current.world = incoming.world;
                    current.skin_type = AnimState::skin_from_state(anim_state, None);
                    // refresh sprint grace when server reports sprinting
                    if incoming.is_sprinting {
                        current.sprint_grace = OtherPlayer::SPRINT_GRACE;
                    }
                    // update authoritative flags from server
                    current.is_sprinting = incoming.is_sprinting;
                    current.apply_snapshot(incoming.x, incoming.y, incoming.z, incoming.yaw);
                    // preserve updated kills
                    current.kills = incoming.kills;
                    merged.push(current);
                } else {
                    // for newly-seen players treat sprint flag as fresh and prime the grace timer
                    if incoming.is_sprinting {
                        incoming.sprint_grace = OtherPlayer::SPRINT_GRACE;
                    }
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
                    // initialize move-direction hysteresis state for new players
                    incoming.last_move_dir = anim_state.move_dir;
                    incoming.pending_move_dir = None;
                    incoming.pending_move_timer = 0.0;
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

        if let Some((world_id, spawn)) = updates.world_change {
            playing_state.current_world = world_id;
            playing_state.position = spawn;
            playing_state.movement_state.velocity = Vec3::ZERO;
            // If we just entered the Maze, start the short post-teleport movement freeze
            if world_id == WorldId::Maze {
                playing_state.post_teleport_freeze_timer = 10.0; // start client-side match timer to mirror server MATCH_DURATION
                playing_state.match_time_left = 120.0; // seconds
            } else {
                // returning to lobby: clear match-end state and maze
                playing_state.match_time_left = 0.0;
                playing_state.match_end_winner = None;
                playing_state.match_end_countdown_timer = 0.0;
                playing_state.local_kills = 0;
                playing_state.destroyed_lights.clear();
                *maze = None;
            }
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
            // also show a short announcement so hits appear in the feed
            let ann = if let Some(aid) = attacker_id {
                playing_state
                    .connected_players
                    .iter()
                    .find(|(id, _n, _)| *id == aid)
                    .map(|(_, name, _)| format!("Hit by {}", name))
                    .unwrap_or_else(|| "Hit!".to_string())
            } else {
                "Hit!".to_string()
            };
            playing_state.announcements.push((ann, 2.0));
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

        // Authoritative match timer (server-sent)
        if let Some(secs) = updates.match_time_left {
            playing_state.match_time_left = secs as f32;
        }

        // Match-join announcement broadcast (show bottom-left)
        if let Some(msg) = updates.match_join_announce {
            playing_state.announcements.push((msg, 4.0));
        }

        // Match-end winner + countdown (server-sent). Do NOT change world locally; server will send WorldChange when it executes the return.
        if let Some(winner) = updates.match_end_winner {
            playing_state.match_end_winner = Some(winner.clone());
            // clear client-side match timer immediately when match end is announced
            playing_state.match_time_left = 0.0;
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
            // detect whether the server still knows about *us* — if not, server removed us (timed out)
            let still_present = list
                .iter()
                .any(|(id, _name, _kills, _world, _alive)| *id == player_id);
            if !still_present {
                result.removed_by_server = true;
            }

            playing_state.connected_players = list
                .into_iter()
                .map(|(id, name, kills, _world, _alive)| (id, name, kills))
                .collect();
        }

        result.assets = updates.assets;
    }

    result
}
