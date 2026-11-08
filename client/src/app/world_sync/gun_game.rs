use macroquad::prelude::*;

use client::net::protocols::MODE_GUN_GAME;
use client::screens::PlayingState;
use client::types::world::WorldId;

/// Syncs the authoritative Gun Game weapon for the local player.
pub(super) fn apply_gun_level(
    playing_state: &mut PlayingState,
    gl: u8,
    match_mode: Option<u8>,
    world_change: &Option<(WorldId, Vec3, Option<u8>, Option<u16>)>,
) {
    let current_mode = match_mode.or(playing_state.match_mode);
    if current_mode == Some(MODE_GUN_GAME) {
        // server sends 1 = first slot; convert to zero-based slot index
        let _slot = (gl as usize).saturating_sub(1);

        // If we're already in the Maze, apply immediately. If the update also contains a
        // pending world_change to Maze (same poll), apply immediately as well. Otherwise
        // defer application until the client receives the world change / teleport.
        let will_be_in_maze = world_change
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
