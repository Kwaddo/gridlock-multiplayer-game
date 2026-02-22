use std::net::UdpSocket;

use client::types::world::WorldId;
use client::ui::OtherPlayer;
use macroquad::prelude::*;

use client::net::poll_updates;
use client::net::protocols::MODE_GUN_GAME;
use client::screens::PlayingState;
use client::types::animation_state::AnimState;
use client::types::maze::Maze;

pub struct SyncResult {
    pub assets: Option<Vec<(String, String)>>,
    pub server_seen: bool, // true if we received at least one packet during poll
    pub removed_by_server: bool, // true if server no longer lists our player id (timed-out/removed)
}

#[inline(always)]
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

        // If the server sent a new maze, remember its spawn point as well.
        // The spawn coordinates are the teleporter location and are only used
        // when we don't already have an explicit world_change to Maze. This
        // covers the case where a world_change packet arrived but the accompanying
        // maze data was lost, leaving the client in "Maze" with no geometry.
        let mut maze_spawn: Option<Vec3> = None;
        if let Some((new_maze, spawn)) = updates.maze {
            *maze = Some(new_maze);
            maze_spawn = Some(spawn);
        }
        if let Some(players) = updates.players {
            // track ids we saw this frame so we can prune removed players later
            playing_state.seen_player_ids.clear();
            // reserve enough space in case it shrinks
            playing_state.seen_player_ids.reserve(players.len());

            for mut incoming in players {
                playing_state.seen_player_ids.insert(incoming.id);
                if let Some(&idx) = playing_state.player_index.get(&incoming.id) {
                    // update existing player by index
                    let current = &mut playing_state.other_players[idx];
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
                        if current.pending_move_dir == Some(candidate_dir) {
                            // pending already in progress; leave it
                        } else {
                            current.pending_move_dir = Some(candidate_dir);
                            current.pending_move_timer = OtherPlayer::MOVE_DIR_GRACE;
                        }
                        anim_state.move_dir = current.last_move_dir;
                    } else {
                        current.pending_move_dir = None;
                        current.pending_move_timer = 0.0;
                        current.last_move_dir = candidate_dir;
                        anim_state.move_dir = candidate_dir;
                    }

                    current.username = incoming.username;
                    current.world = incoming.world;
                    current.skin_type = AnimState::skin_from_state(anim_state, None);
                    if incoming.is_sprinting {
                        current.sprint_grace = OtherPlayer::SPRINT_GRACE;
                    }
                    current.is_sprinting = incoming.is_sprinting;
                    current.apply_snapshot(incoming.x, incoming.y, incoming.z, incoming.yaw);
                    current.kills = incoming.kills;
                    current.hp = incoming.hp;
                    current.max_hp = incoming.max_hp;
                } else {
                    // new player
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
                    incoming.last_move_dir = anim_state.move_dir;
                    incoming.pending_move_dir = None;
                    incoming.pending_move_timer = 0.0;
                    incoming.apply_snapshot(incoming.x, incoming.y, incoming.z, incoming.yaw);
                    let idx = playing_state.other_players.len();
                    playing_state.other_players.push(incoming);
                    playing_state
                        .player_index
                        .insert(playing_state.other_players[idx].id, idx);
                }
            }

            // remove any players the server no longer reports
            if !playing_state.seen_player_ids.is_empty() {
                playing_state
                    .other_players
                    .retain(|p| playing_state.seen_player_ids.contains(&p.id));
                // rebuild index after potential removals/shifts
                playing_state.rebuild_player_index();
            }
        }

        // Update local player's kill count when server provides it via MSG_STATE
        if let Some(k) = updates.self_kills {
            playing_state.local_kills = k;
        }

        // If server indicates match mode (sent with countdown or world change), store it locally
        if let Some(mode) = updates.match_mode {
            playing_state.match_mode = Some(mode);
        }

        // Sync authoritative Gun Game weapon slot for local player — only enforce when the
        // authoritative/current mode is Gun Game (prefer mode included in the same update).
        // IMPORTANT: apply the forced slot *after* the player actually teleports into Maze.
        if let Some(gl) = updates.self_gun_level {
            // determine the authoritative mode for this update: prefer updates.match_mode if present
            let current_mode = updates.match_mode.or(playing_state.match_mode);
            if current_mode == Some(MODE_GUN_GAME) {
                // server sends 1 = first slot; convert to zero-based slot index
                let slot = (gl as usize).saturating_sub(1);

                // If we're already in the Maze, apply immediately. If the update also contains a
                // pending world_change to Maze (same poll), apply immediately as well. Otherwise
                // defer application until the client receives the world change / teleport.
                let will_be_in_maze = updates
                    .world_change
                    .as_ref()
                    .map(|(w, _)| *w == WorldId::Maze)
                    .unwrap_or(false);

                if playing_state.current_world == WorldId::Maze || will_be_in_maze {
                    let _ = playing_state.weapons.switch_to(slot);
                    playing_state.weapons.set_locked_slot(Some(slot));
                    playing_state.pending_forced_weapon_slot = None;
                } else {
                    // store and wait until teleport completes
                    playing_state.pending_forced_weapon_slot = Some(slot);
                }
            } else {
                // do NOT force weapon changes from the server when not in Gun Game
                // just clear any previous lock so the player can freely switch in the lobby
                playing_state.weapons.set_locked_slot(None);
                playing_state.pending_forced_weapon_slot = None;
            }
        }

        // NOTE: previously we moved the client to the lobby as soon as a
        // MSG_MATCH_END packet arrived.  behaviour here should be limited to
        // queuing up the winner announcement; the server will issue a
        // MSG_WORLD_CHANGE later when the countdown expires.  leaving the old
        // teleport logic caused the final scoreboard to flash briefly and then
        // disappear, and it also wiped out any kill count the player had just
        // earned.

        // (no longer doing anything here)

        let mut world_change_applied = false;
        if let Some((world_id, spawn)) = updates.world_change {
            playing_state.current_world = world_id;
            playing_state.position = spawn;
            playing_state.movement_state.velocity = Vec3::ZERO;
            world_change_applied = true;

            // If we just entered the Maze, start the short post-teleport movement freeze
            if world_id == WorldId::Maze {
                playing_state.post_teleport_freeze_timer = 10.0; // start client-side match timer to mirror server MATCH_DURATION
                playing_state.match_time_left = 120.0; // seconds

                // if a forced slot was received earlier while we were still in the lobby,
                // apply it now that the teleport is complete
                if let Some(forced) = playing_state.pending_forced_weapon_slot.take() {
                    let _ = playing_state.weapons.switch_to(forced);
                    playing_state.weapons.set_locked_slot(Some(forced));
                }
            } else {
                // returning to lobby: clear match-end state and maze
                playing_state.match_time_left = 0.0;
                // leaving the Maze / returning to Lobby — clear any mode-specific restrictions
                playing_state.match_mode = None;
                playing_state.weapons.set_locked_slot(None);
                playing_state.pending_forced_weapon_slot = None;
            }
            // If the server attached a match_mode to the world-change packet, remember it.
            // We used to clear `*maze` here, but that meant the very maze we just received
            // would be discarded when a match started (word_change+match_mode arrive
            // together). the maze is already cleared when we explicitly return to the
            // lobby above, so keep it intact on entry.
            if let Some(mode) = updates.match_mode {
                playing_state.match_mode = Some(mode);
                playing_state.match_end_winner = None;
                playing_state.match_end_countdown_timer = 0.0;
                playing_state.local_kills = 0;
            }
        }

        // if we just received maze data but didn't have a world change this poll,
        // and we're already considered to be in the Maze, use the teleporter spawn so
        // we don't end up stranded in empty space. this covers lost-world-change
        // scenarios and ensures teleporting works even if the maze packet arrives
        // slightly after the world-change message.
        if let Some(spawn) = maze_spawn {
            if !world_change_applied && playing_state.current_world == WorldId::Maze {
                playing_state.position = spawn;
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
                // push a kill-feed entry for UI
                playing_state
                    .kill_feed
                    .push(client::screens::playing::KillFeedEntry {
                        killer_id,
                        victim_id,
                        weapon,
                        timer: 5.0,
                    });

                // if the victim is another player, remove them immediately so no
                // corpse lingers. the server will eventually stop sending a state
                // update for the dead player, but the kill-feed packet arrives
                // slightly earlier and gives us an opportunity to cull them right
                // away. note: we only remove non-local players here since the
                // local client handles its own death path separately.
                if victim_id != player_id {
                    playing_state.other_players.retain(|p| p.id != victim_id);
                    playing_state.rebuild_player_index();
                }
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
            // leave local_kills alone so final score is visible until we actually
            // teleport back to the lobby (MSG_WORLD_CHANGE will arrive later).
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
