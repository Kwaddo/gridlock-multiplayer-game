use macroquad::prelude::*;

use client::screens::PlayingState;
use client::types::world::WorldId;

use super::SyncResult;

/// Resets match state on a world change: entering the Maze or returning
/// to the lobby.
pub(super) fn enter_world(
    playing_state: &mut PlayingState,
    result: &mut SyncResult,
    world_id: WorldId,
    freeze_secs: Option<u8>,
    timer_secs: Option<u16>,
    match_mode: Option<u8>,
) {
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
        // (the server sends the new match's score straight away)
        playing_state.team_score = None;
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
        // a match the host ended early leaves no victory screen up
        playing_state.match_end_countdown_timer = 0.0;
        playing_state.team_score = None;
        // leaving the Maze / returning to Lobby — clear any mode-specific restrictions
        playing_state.match_mode = None;
        playing_state.weapons.set_locked_slot(None);
        playing_state.weapons.knife_locked = false;
        playing_state.pending_forced_weapon_slot = None;
        result.returned_to_lobby = true;

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
    if let Some(mode) = match_mode {
        playing_state.match_mode = Some(mode);
        playing_state.match_end_winner = None;
        playing_state.match_end_countdown_timer = 0.0;
        playing_state.local_kills = 0;
        playing_state.local_deaths = 0;
    }
}
