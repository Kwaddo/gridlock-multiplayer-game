use std::collections::HashMap;
use crate::net::{Outbox, PeerId};
use platform::Instant;

use crate::player::Player;
use crate::player::WorldId;
use crate::protocols::*;

/// Handler for movement updates sent by clients.
pub fn handle_position(
    _socket: &Outbox,
    players: &mut HashMap<PeerId, Player>,
    src_addr: PeerId,
    (x, y, z): (f32, f32, f32),
    yaw: f32,
    state_flags: Option<u8>,
    event_flags: Option<u8>,
) {

    if let Some(player) = players.get_mut(&src_addr) {
        // If the *player* is currently in their per-player post-teleport freeze,
        // ignore movement updates from that player so they remain stationary.
        if player
            .post_teleport_freeze_until
            .map_or(false, |t| t > Instant::now())
            && player.world == WorldId::Maze
        {
            player.last_activity = Instant::now();
            return;
        }

        player.position = (x, y, z);
        player.yaw = yaw;
        player.last_activity = Instant::now();
    }

    if let Some(state_flags) = state_flags {
        if let Some(player) = players.get_mut(&src_addr) {
            player.is_sprinting = (state_flags & FLAG_SPRINTING) != 0;
            player.is_crouched = (state_flags & FLAG_CROUCHING) != 0;
            player.is_ads = (state_flags & FLAG_ADS) != 0;
            player.is_airborne = (state_flags & FLAG_AIRBORNE) != 0;
            player.is_prone = (state_flags & FLAG_PRONE) != 0;
            player.is_sliding = (state_flags & FLAG_SLIDING) != 0;
        }
    }
    if let Some(event_flags) = event_flags {
        if let Some(player) = players.get_mut(&src_addr) {
            // Kept until the next state broadcast sends them (see Server::tick).
            player.evt_jump_start |= (event_flags & EVT_JUMP_START) != 0;
            player.evt_landed |= (event_flags & EVT_LANDED) != 0;
        }
    }

}
