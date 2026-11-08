use client::net::protocols::{MODE_SEARCH_AND_DESTROY, MODE_TEAM_DEATHMATCH};
use client::screens::PlayingState;
use client::types::world::WorldId;

use super::SyncResult;

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
