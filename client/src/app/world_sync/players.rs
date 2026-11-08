use client::net::protocols::{MODE_SEARCH_AND_DESTROY, MODE_TEAM_DEATHMATCH};
use client::screens::PlayingState;
use client::types::animation_state::{AnimState, MoveDir};
use client::types::world::WorldId;
use client::ui::OtherPlayer;

use super::SyncResult;

/// Merges the server's player snapshots into the other players we draw.
pub(super) fn merge_players(playing_state: &mut PlayingState, players: Vec<OtherPlayer>) {
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

/// Applies the global connected player list (for Tab / scoreboard).
pub(super) fn apply_player_list(
    playing_state: &mut PlayingState,
    result: &mut SyncResult,
    list: Vec<(u32, String, u32, u32, WorldId, bool, u8, u8, String, bool)>,
    player_id: u32,
) {
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
        playing_state.local_team = me.7;
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

    playing_state.match_player_ids = list
        .iter()
        .filter(|(_, _, _, _, world, ..)| *world == WorldId::Maze)
        .map(|(id, ..)| *id)
        .collect();
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

/// Tints the other players by team in team modes, or by id otherwise.
pub(super) fn apply_team_colors(playing_state: &mut PlayingState) {
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
}
