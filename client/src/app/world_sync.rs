use transport::Link;

use client::types::world::WorldId;
use macroquad::prelude::*;

use client::net::poll_updates;
use client::screens::PlayingState;
use client::types::maze::Maze;

mod combat;
mod gun_game;
mod merge;
mod players;
mod world_change;

use combat::{apply_damage, apply_death, apply_kill_feed, spawn_bot_shots};
use gun_game::apply_gun_level;
use merge::{apply_player_info, merge_players};
use players::{apply_player_list, apply_team_colors};
use world_change::enter_world;

pub struct SyncResult {
    pub server_seen: bool, // true if we received at least one packet during poll
    pub removed_by_server: bool, // true if server no longer lists our player id (timed-out/removed)
    pub returned_to_lobby: bool, // true when a world-change back to Lobby was applied this poll
}

#[inline(always)]
pub fn sync_world(
    link: Option<&mut dyn Link>,
    player_id: u32,
    username: &str,
    playing_state: &mut PlayingState,
    maze: &mut Option<Maze>,
) -> SyncResult {
    let mut result = SyncResult {
        server_seen: false,
        removed_by_server: false,
        returned_to_lobby: false,
    };

    if let Some(link) = link {
        let updates = poll_updates(link, player_id, &mut playing_state.player_info);

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
            merge_players(playing_state, players);
        }
        if updates.player_info_changed {
            apply_player_info(playing_state);
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
            apply_gun_level(playing_state, gl, updates.match_mode, &updates.world_change);
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
            playing_state.mirrors = client::world::mirror::mirrors_for_world(world_id);
            playing_state.position = spawn;
            playing_state.movement_state.velocity = Vec3::ZERO;
            world_change_applied = true;
            enter_world(playing_state, &mut result, world_id, freeze_secs, timer_secs, updates.match_mode);
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
        // The server sends whole seconds rounded down, so it only ever extends
        // the local timer: overwriting it would unlock up to a second early.
        if !world_change_applied {
            if let Some(secs) = updates.self_freeze_secs {
                if playing_state.current_world == WorldId::Maze
                    && secs as f32 > playing_state.post_teleport_freeze_timer
                {
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
        if let Some((damage, attacker_id, is_stun)) = updates.damage_taken {
            apply_damage(playing_state, damage, attacker_id, is_stun);
        }
        if let Some(bot_shots) = updates.bot_shots {
            spawn_bot_shots(playing_state, bot_shots, maze.as_ref());
        }
        if let Some(kills) = updates.kill_feed {
            apply_kill_feed(playing_state, kills, player_id, username);
        }
        if updates.death {
            apply_death(playing_state, updates.death_killer_id, &updates.death_weapon);
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

        for event in updates.chat {
            playing_state.chat.push(event);
        }

        // Match-join announcement broadcast: in the chat, with the other comings and goings
        if let Some(msg) = updates.match_join_announce {
            playing_state.chat.push(client::net::ChatEvent::Notice(msg));
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

        // Lobby leader update
        if let Some(status) = updates.match_status {
            playing_state.match_status = status;
        }
        if let Some(score) = updates.team_score {
            playing_state.team_score = Some(score);
        }
        if let Some(settings) = updates.lobby_settings {
            playing_state.host_settings = Some(settings);
        }
        if let Some(leader_id) = updates.lobby_leader_id {
            playing_state.lobby_leader_id = if leader_id == 0 { None } else { Some(leader_id) };
        }

        // Global connected player list (for Tab / scoreboard)
        if let Some(list) = updates.player_list {
            apply_player_list(playing_state, &mut result, list, player_id);
        }

        // ── Always apply team-based colors after all updates ────────────
        // Team colors must be reapplied every sync, not just when
        // MSG_PLAYER_LIST arrives. MSG_STATE creates/updates OtherPlayer
        // objects with default colors, so without this they would lose
        // their team tint until the next (infrequent) player list packet.
        apply_team_colors(playing_state);
    }

    result
}
