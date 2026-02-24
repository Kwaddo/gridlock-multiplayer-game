use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::Instant;

use crate::player::Player;
use crate::player::WorldId;
use crate::protocols::*;

use crate::server::broadcast;

/// Handler for movement updates sent by clients.
pub fn handle_position(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    src_addr: SocketAddr,
    payload: &[u8],
) {
    if payload.len() < 16 {
        return;
    }

    // parse client-provided snapshot first
    let x = f32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let y = f32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
    let z = f32::from_be_bytes([payload[8], payload[9], payload[10], payload[11]]);
    let yaw = f32::from_be_bytes([payload[12], payload[13], payload[14], payload[15]]);

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

    if payload.len() >= 17 {
        let state_flags = payload[16];
        if let Some(player) = players.get_mut(&src_addr) {
            player.is_sprinting = (state_flags & FLAG_SPRINTING) != 0;
            player.is_crouched = (state_flags & FLAG_CROUCHING) != 0;
            player.is_ads = (state_flags & FLAG_ADS) != 0;
            player.is_airborne = (state_flags & FLAG_AIRBORNE) != 0;
            player.is_prone = (state_flags & FLAG_PRONE) != 0;
        }
    }
    if payload.len() >= 18 {
        let event_flags = payload[17];
        if let Some(player) = players.get_mut(&src_addr) {
            player.evt_jump_start = (event_flags & EVT_JUMP_START) != 0;
            player.evt_landed = (event_flags & EVT_LANDED) != 0;
        }
    }

    broadcast::broadcast_state(socket, players);
}
