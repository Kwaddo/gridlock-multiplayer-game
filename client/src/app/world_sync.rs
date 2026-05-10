use std::net::UdpSocket;

use client::types::world::WorldId;
use client::ui::OtherPlayer;
use macroquad::prelude::*;

use client::net::poll_updates;
use client::net::protocols::{MODE_GUN_GAME, MODE_SEARCH_AND_DESTROY, MODE_TEAM_DEATHMATCH};
use client::screens::PlayingState;
use client::types::animation_state::{AnimState, MoveDir};
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
    username: &str,
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
                    let incoming_dead = !incoming.is_alive;

                    // skip updating dying players — they're playing a death animation
                    if current.death_timer.is_some() {
                        continue;
                    }

                    // Check if this player's position actually changed.
                    // The server broadcasts state on every position packet from
                    // ANY player, so most broadcasts carry the same position for
                    // players who didn't move. Only recompute animations when
                    // the position genuinely changed.
                    let dx = incoming.x - current.x;
                    let dz = incoming.z - current.z;
                    let pos_changed = dx * dx + dz * dz > 1.0e-4;
                    let state_changed = incoming.is_sprinting != current.is_sprinting
                        || incoming.is_ads != current.is_ads
                        || incoming.is_crouching != current.is_crouching
                        || incoming.is_prone != current.is_prone
                        || incoming.is_sliding != current.is_sliding
                        || incoming.evt_jump_start;

                    if pos_changed || state_changed {
                        let effective_sprinting = incoming.is_sprinting
                            || current.is_sprinting
                            || current.sprint_grace > 0.0;
                        let anim_state = AnimState::from_movement(
                            current.x,
                            current.z,
                            incoming.x,
                            incoming.z,
                            incoming.yaw,
                            effective_sprinting,
                            incoming.is_ads,
                            incoming.is_crouching,
                            incoming.is_prone,
                            incoming.is_sliding,
                            incoming.evt_jump_start,
                        );

                        let candidate_dir = anim_state.move_dir;
                        if candidate_dir != MoveDir::None {
                            current.last_move_dir = candidate_dir;
                        }
                        current.skin_type = AnimState::skin_from_state(anim_state, None);
                        current.last_move_time = macroquad::time::get_time();
                    }
                    // When position is unchanged we do nothing — the per-frame
                    // update() on OtherPlayer will transition to idle after a
                    // short grace period using last_move_time.

                    current.username = incoming.username;
                    current.world = incoming.world;
                    if incoming.is_sprinting {
                        current.sprint_grace = OtherPlayer::SPRINT_GRACE;
                    }
                    current.is_sprinting = incoming.is_sprinting;
                    current.is_ads = incoming.is_ads;
                    current.is_crouching = incoming.is_crouching;
                    current.is_prone = incoming.is_prone;
                    current.is_sliding = incoming.is_sliding;
                    current.apply_snapshot(incoming.x, incoming.y, incoming.z, incoming.yaw);
                    current.kills = incoming.kills;
                    current.team = incoming.team;
                    // copy weapon string verbatim; server will send a non-empty
                    // string when the player has a weapon. clearing it when the
                    // player drops/changes items is valid and ensures we don't
                    // render stale guns.
                    current.current_weapon = incoming.current_weapon;
                    current.hp = incoming.hp;
                    current.max_hp = incoming.max_hp;
                    current.is_alive = incoming.is_alive;

                    if incoming_dead && current.death_timer.is_none() {
                        current.skin_type = if current.is_prone {
                            client::types::player::PlayerSkin::ProneDeath
                        } else if current.is_crouching {
                            client::types::player::PlayerSkin::CrouchDeath
                        } else {
                            client::types::player::PlayerSkin::StandDeath
                        };
                        current.death_timer = Some(2.0);
                        if let Some(model_key) = current.skin_type.model_key() {
                            if let Some(anim) = playing_state.animated_models.get_mut(model_key) {
                                anim.play_from_start(false);
                            }
                        }
                    }
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
                        incoming.is_prone,
                        incoming.is_sliding,
                        incoming.evt_jump_start,
                    );
                    incoming.skin_type = AnimState::skin_from_state(anim_state, None);
                    incoming.last_move_dir = anim_state.move_dir;
                    incoming.pending_move_dir = None;
                    incoming.pending_move_timer = 0.0;
                    incoming.apply_snapshot(incoming.x, incoming.y, incoming.z, incoming.yaw);
                    if !incoming.is_alive {
                        incoming.skin_type = if incoming.is_prone {
                            client::types::player::PlayerSkin::ProneDeath
                        } else if incoming.is_crouching {
                            client::types::player::PlayerSkin::CrouchDeath
                        } else {
                            client::types::player::PlayerSkin::StandDeath
                        };
                        incoming.death_timer = Some(2.0);
                    }
                    let idx = playing_state.other_players.len();
                    playing_state.other_players.push(incoming);
                    playing_state
                        .player_index
                        .insert(playing_state.other_players[idx].id, idx);
                }
            }

            // remove any players the server no longer reports
            // (keep dying players until their death animation timer expires)
            if !playing_state.seen_player_ids.is_empty() {
                playing_state
                    .other_players
                    .retain(|p| playing_state.seen_player_ids.contains(&p.id) || p.death_timer.is_some());
                // rebuild index after potential removals/shifts
                playing_state.rebuild_player_index();
            }
        }

        // Update local player's kill count when server provides it via MSG_STATE
        if let Some(k) = updates.self_kills {
            playing_state.local_kills = k;
        }
        if let Some(hp) = updates.self_hp {
            playing_state.hp = hp;
        }
        if let Some(max_hp) = updates.self_max_hp {
            playing_state.max_hp = max_hp;
        }

        // If server indicates match mode (sent with countdown or world change), store it locally
        if let Some(mode) = updates.match_mode {
            playing_state.match_mode = Some(mode);
        }

        // Sync authoritative Gun Game weapon for local player — only enforce when the
        // authoritative/current mode is Gun Game (prefer mode included in the same update).
        // IMPORTANT: apply the forced weapon *after* the player actually teleports into Maze.
        if let Some(gl) = updates.self_gun_level {
            let current_mode = updates.match_mode.or(playing_state.match_mode);
            if current_mode == Some(MODE_GUN_GAME) {
                // server sends 1 = first slot; convert to zero-based slot index
                let _slot = (gl as usize).saturating_sub(1);

                // If we're already in the Maze, apply immediately. If the update also contains a
                // pending world_change to Maze (same poll), apply immediately as well. Otherwise
                // defer application until the client receives the world change / teleport.
                let will_be_in_maze = updates
                    .world_change
                    .as_ref()
                    .map(|(w, _, _, _)| *w == WorldId::Maze)
                    .unwrap_or(false);

                if playing_state.current_world == WorldId::Maze || will_be_in_maze {
                    // Gun Game: replace the weapon in slot 0 with the progression weapon
                    let level = gl as usize;
                    let is_knife_level = level > client::weapons::GUN_GAME_PROGRESSION.len();
                    if is_knife_level {
                        // Final level: knife only — force knife active
                        if !playing_state.weapons.knife_active {
                            playing_state.weapons.toggle_knife();
                        }
                        // Lock to slot 0 but keep knife active
                        playing_state.weapons.set_locked_slot(Some(0));
                        // prevent the player from putting knife away
                        playing_state.weapons.knife_locked = true;
                    } else if level >= 1 {
                        // leaving knife-only level: unlock knife
                        playing_state.weapons.knife_locked = false;
                        let weapon_id = client::weapons::GUN_GAME_PROGRESSION[level - 1];
                        // Only replace if the weapon actually changed
                        if playing_state.weapons.weapons[0].id != weapon_id {
                            playing_state.weapons.replace_weapon(0, weapon_id);
                        }
                        // Ensure we're on slot 0 (not knife) unless player toggled knife
                        if playing_state.weapons.knife_active {
                            // Let player keep knife out; lock still applies
                        } else {
                            playing_state.weapons.active_slot = 0;
                        }
                        playing_state.weapons.set_locked_slot(Some(0));
                    }
                    playing_state.pending_forced_weapon_slot = None;
                } else {
                    // Store gun_level and apply when teleport completes
                    playing_state.pending_forced_weapon_slot = Some(gl as usize);
                }
            } else {
                // Not Gun Game: clear any lock so player can freely switch
                playing_state.weapons.set_locked_slot(None);
                playing_state.pending_forced_weapon_slot = None;
                playing_state.weapons.knife_locked = false;
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
        if let Some((world_id, spawn, freeze_secs, timer_secs)) = updates.world_change {
            playing_state.current_world = world_id;
            playing_state.position = spawn;
            playing_state.movement_state.velocity = Vec3::ZERO;
            world_change_applied = true;

            // If we just entered the Maze, start the post-teleport movement freeze
            // using the server-provided duration so all clients are synchronized.
            if world_id == WorldId::Maze {
                let freeze = freeze_secs.map(|s| s as f32).unwrap_or(10.0);
                playing_state.post_teleport_freeze_timer = freeze;
                // if the server included an authoritative timer in the world-change packet
                // use it; otherwise leave whatever value we already had (it will be updated
                // by MSG_MATCH_TIMER soon thereafter).
                if let Some(ts) = timer_secs {
                    playing_state.match_time_left = ts as f32;
                }

                // Reset local player state for a fresh match
                playing_state.is_dead = false;
                playing_state.hp = playing_state.max_hp;
                playing_state.death_timer = 0.0;
                playing_state.local_kills = 0;
                playing_state.local_deaths = 0;
                playing_state.movement_state.velocity = Vec3::ZERO;

                // Clear stale other-player data so animations don't carry over
                playing_state.other_players.clear();
                playing_state.player_index.clear();
                playing_state.seen_player_ids.clear();

                // if a forced gun_level was received earlier while we were still in the lobby,
                // apply it now that the teleport is complete
                if let Some(level) = playing_state.pending_forced_weapon_slot.take() {
                    let is_knife_level = level > client::weapons::GUN_GAME_PROGRESSION.len();
                    if is_knife_level {
                        if !playing_state.weapons.knife_active {
                            playing_state.weapons.toggle_knife();
                        }
                        playing_state.weapons.set_locked_slot(Some(0));
                    } else if level >= 1 {
                        let weapon_id = client::weapons::GUN_GAME_PROGRESSION[level - 1];
                        playing_state.weapons.replace_weapon(0, weapon_id);
                        playing_state.weapons.active_slot = 0;
                        playing_state.weapons.set_locked_slot(Some(0));
                    }
                }
            } else {
                // returning to lobby: clear match-end state and maze
                playing_state.match_time_left = 0.0;
                // leaving the Maze / returning to Lobby — clear any mode-specific restrictions
                playing_state.match_mode = None;
                playing_state.weapons.set_locked_slot(None);
                playing_state.weapons.knife_locked = false;
                playing_state.pending_forced_weapon_slot = None;

                // Reset local player state when returning to lobby
                playing_state.is_dead = false;
                playing_state.hp = playing_state.max_hp;
                playing_state.death_timer = 0.0;
                playing_state.local_kills = 0;
                playing_state.local_deaths = 0;

                // Clear stale other-player data
                playing_state.other_players.clear();
                playing_state.player_index.clear();
                playing_state.seen_player_ids.clear();
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
                playing_state.local_deaths = 0;
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

        // If we did not apply a world_change this poll but the server included
        // an authoritative per-player freeze in MSG_STATE, use it as a fallback
        // to enforce movement/weapon locks when the MSG_WORLD_CHANGE packet
        // was lost while entering the Maze.
        if !world_change_applied {
            if let Some(secs) = updates.self_freeze_secs {
                if playing_state.current_world == WorldId::Maze {
                    playing_state.post_teleport_freeze_timer = secs as f32;
                }
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
                    .find(|(id, ..)| *id == aid)
                    .map(|(_, name, ..)| format!("Hit by {}", name))
                    .unwrap_or_else(|| "Hit!".to_string())
            } else {
                "Hit!".to_string()
            };
            playing_state.announcements.push((ann, 2.0));
        }
        if let Some(bot_shots) = updates.bot_shots {
            for (start, end) in bot_shots {
                playing_state
                    .tracers
                    .push(client::weapons::effects::BulletTracer {
                        start,
                        end,
                        lifetime: 0.09,
                        speed: 120.0,
                        progress: 0.0,
                        tracer_length: 0.5,
                        tracer_color: [255, 170, 90, 255],
                        tracer_thickness: 0.03,
                    });
                playing_state.world_muzzle_flashes.push(
                    client::weapons::effects::WorldMuzzleFlash {
                        position: start,
                        lifetime: client::weapons::effects::WorldMuzzleFlash::DURATION,
                    },
                );
            }
        }
        if let Some(kills) = updates.kill_feed {
            for (killer_id, victim_id, weapon) in kills {
                // resolve names now so they persist even after player models are removed
                let killer_name = if killer_id == player_id {
                    username.to_string()
                } else {
                    playing_state
                        .connected_players
                        .iter()
                        .find(|(id, ..)| *id == killer_id)
                        .map(|(_, name, ..)| name.clone())
                        .or_else(|| {
                            playing_state
                                .other_players
                                .iter()
                                .find(|p| p.id == killer_id)
                                .map(|p| p.username.clone())
                        })
                        .unwrap_or_else(|| "Player".to_string())
                };
                let victim_name = if victim_id == player_id {
                    username.to_string()
                } else {
                    playing_state
                        .connected_players
                        .iter()
                        .find(|(id, ..)| *id == victim_id)
                        .map(|(_, name, ..)| name.clone())
                        .or_else(|| {
                            playing_state
                                .other_players
                                .iter()
                                .find(|p| p.id == victim_id)
                                .map(|p| p.username.clone())
                        })
                        .unwrap_or_else(|| "Player".to_string())
                };

                // resolve teams from connected_players
                let killer_team = playing_state
                    .connected_players
                    .iter()
                    .find(|(id, ..)| *id == killer_id)
                    .map(|(_, _, _, _, _, team, _, _)| *team)
                    .unwrap_or(0);
                let victim_team = playing_state
                    .connected_players
                    .iter()
                    .find(|(id, ..)| *id == victim_id)
                    .map(|(_, _, _, _, _, team, _, _)| *team)
                    .unwrap_or(0);

                // push a kill-feed entry for UI
                playing_state
                    .kill_feed
                    .push(client::screens::playing::KillFeedEntry {
                        killer_id,
                        victim_id,
                        killer_name,
                        victim_name,
                        killer_team,
                        victim_team,
                        weapon,
                        timer: 5.0,
                    });

                // kill confirmation effect for local player
                if killer_id == player_id {
                    playing_state.kill_confirm_timer = 0.3;
                    // TDM: resupply ammo on kill
                    if playing_state.match_mode == Some(client::net::protocols::MODE_TEAM_DEATHMATCH) {
                        playing_state.weapons.resupply_active(15);
                    }
                }

                // mark victim as dying so they play a death animation before removal.
                // the server will eventually stop sending state updates for the dead
                // player, but the kill-feed packet arrives earlier and gives us an
                // opportunity to trigger the animation right away.
                if victim_id != player_id {
                    if let Some(victim) = playing_state.other_players.iter_mut().find(|p| p.id == victim_id) {
                        // pick death animation based on current stance
                        victim.skin_type = if victim.is_prone {
                            client::types::player::PlayerSkin::ProneDeath
                        } else if victim.is_crouching {
                            client::types::player::PlayerSkin::CrouchDeath
                        } else {
                            client::types::player::PlayerSkin::StandDeath
                        };
                        victim.death_timer = Some(2.0);
                        // reset animation playback so it starts from the beginning
                        if let Some(model_key) = victim.skin_type.model_key() {
                            if let Some(anim) = playing_state.animated_models.get_mut(model_key) {
                                anim.play_from_start(false);
                            }
                        }
                    }
                }
            }
        }
        if updates.death {
            if playing_state.current_world == WorldId::Lobby {
                // Lobby safety: never enter death state in lobby; server-side also clamps.
                playing_state.is_dead = false;
                playing_state.hp = playing_state.hp.max(1.0);
            } else {
                // Only trigger death logic when transitioning from alive -> dead.
                // Servers send periodic MSG_STATE entries marking a player as
                // dead until they respawn; calling `die()` every poll would keep
                // resetting the death timer and prevent auto-respawn.
                if !playing_state.is_dead {
                    // resolve killer name from the connected_players list
                    if let Some(killer_id) = updates.death_killer_id {
                        playing_state.death_killer_name = playing_state
                            .connected_players
                            .iter()
                            .find(|(id, ..)| *id == killer_id)
                            .map(|(_, name, ..)| name.clone());
                    } else {
                        playing_state.death_killer_name = None;
                    }
                    playing_state.death_weapon_name = updates.death_weapon.clone();
                    playing_state.die(None);
                }
            }
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

        // S&D round state
        if let Some(snd) = updates.snd_round_state {
            playing_state.snd_round = snd.round;
            playing_state.snd_max_rounds = snd.max_rounds;
            playing_state.snd_red_wins = snd.red_wins;
            playing_state.snd_blue_wins = snd.blue_wins;
            playing_state.snd_phase = snd.phase;
            playing_state.snd_attacking_team = snd.attacking_team;
            playing_state.snd_bomb_carrier_id = snd.bomb_carrier_id;
            playing_state.snd_bomb_dropped = snd.bomb_dropped;
            playing_state.snd_bomb_site = snd.bomb_site;
            playing_state.snd_bomb_planted = snd.bomb_planted;
            playing_state.snd_plant_progress = snd.plant_progress;
            playing_state.snd_defuse_progress = snd.defuse_progress;
            playing_state.snd_phase_timer = snd.phase_timer;
        }

        // Global connected player list (for Tab / scoreboard)
        if let Some(list) = updates.player_list {
            // detect whether the server still knows about *us* — if not, server removed us (timed out)
            let still_present = list.iter().any(
                |(id, _name, _kills, _deaths, _world, _alive, _gl, _team, _weapon, _is_bot)| {
                    *id == player_id
                },
            );
            if !still_present {
                result.removed_by_server = true;
            }

            // track local deaths from the player list
            if let Some(me) = list.iter().find(|(id, ..)| *id == player_id) {
                playing_state.local_deaths = me.3;
            }

            // remove 3D models of players who are no longer in the server's player list
            // (disconnected or timed out) — this ensures their model disappears instantly
            let server_ids: std::collections::HashSet<u32> =
                list.iter().map(|(id, ..)| *id).collect();
            let before = playing_state.other_players.len();
            playing_state
                .other_players
                .retain(|p| server_ids.contains(&p.id) || p.death_timer.is_some());
            if playing_state.other_players.len() != before {
                playing_state.rebuild_player_index();
            }

            // `list` now has entries: (id, name, kills, deaths, world, alive, gun_level, team, weapon, is_bot)
            playing_state.connected_players = list
                .into_iter()
                .map(
                    |(id, name, kills, deaths, _world, _alive, gun_level, team, weapon, is_bot)| {
                        (id, name, kills, deaths, gun_level, team, weapon, is_bot)
                    },
                )
                .collect();
        }

        // ── Always apply team-based colors after all updates ────────────
        // Team colors must be reapplied every sync, not just when
        // MSG_PLAYER_LIST arrives. MSG_STATE creates/updates OtherPlayer
        // objects with default colors, so without this they would lose
        // their team tint until the next (infrequent) player list packet.
        let is_team_mode = matches!(
            playing_state.match_mode,
            Some(m) if m == MODE_TEAM_DEATHMATCH || m == MODE_SEARCH_AND_DESTROY
        );
        if is_team_mode {
            for p in playing_state.other_players.iter_mut() {
                let team = p.team;
                let (br, bg, bb) = match team {
                    1 => (200u8, 60u8, 60u8),   // Red team
                    2 => (60u8, 120u8, 200u8),  // Blue team
                    _ => (120u8, 120u8, 120u8), // default/neutral
                };
                let hue_variation = ((p.id.wrapping_mul(97).wrapping_add(53) % 32) as i32) - 16;
                let adjust = |c: u8| -> u8 {
                    let v = (c as i32) + hue_variation;
                    v.clamp(0, 255) as u8
                };
                p.color =
                    macroquad::prelude::Color::from_rgba(adjust(br), adjust(bg), adjust(bb), 255);
            }
        } else {
            for p in playing_state.other_players.iter_mut() {
                p.color = client::types::player::OtherPlayer::color_for_id(p.id);
            }
        }

        result.assets = updates.assets;
    }

    result
}
