use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::Instant;

use crate::player::{Player, WorldId};
use crate::protocols::*;

/// Send the full connected player list to a single destination
#[allow(dead_code)]
pub fn send_player_list(
    socket: &UdpSocket,
    players: &HashMap<SocketAddr, Player>,
    dest: SocketAddr,
) {
    let count = players.len().min(255) as u8;
    let mut msg = vec![MSG_PLAYER_LIST, count];
    for p in players.values() {
        msg.extend_from_slice(&p.id.to_be_bytes());
        let name_bytes = p.username.as_bytes();
        msg.push(name_bytes.len() as u8);
        msg.extend_from_slice(name_bytes);
        msg.extend_from_slice(&p.kills.to_be_bytes());
        msg.extend_from_slice(&p.deaths.to_be_bytes());
        msg.push(p.world.as_u8());
        msg.push(if p.is_alive { 1u8 } else { 0u8 });
        msg.push(p.gun_level as u8);
        msg.push(p.team.as_u8());
        let wbytes = p.current_weapon.as_bytes();
        msg.push(wbytes.len() as u8);
        msg.extend_from_slice(wbytes);
        msg.push(if p.is_bot { 1u8 } else { 0u8 });
    }
    let _ = socket.send_to(&msg, dest);
}

/// Broadcast updated player list to every connected client.
/// Also broadcasts the current lobby leader so clients can update the host indicator.
pub fn broadcast_player_list(socket: &UdpSocket, players: &HashMap<SocketAddr, Player>) {
    broadcast_lobby_leader(socket, players);

    // Build the message once; the content is identical for every receiver.
    let count = players.len().min(255) as u8;
    let mut msg: Vec<u8> = Vec::with_capacity(1024);
    msg.push(MSG_PLAYER_LIST);
    msg.push(count);
    for player in players.values() {
        msg.extend_from_slice(&player.id.to_be_bytes());
        let name_bytes = player.username.as_bytes();
        msg.push(name_bytes.len() as u8);
        msg.extend_from_slice(name_bytes);
        msg.extend_from_slice(&player.kills.to_be_bytes());
        msg.extend_from_slice(&player.deaths.to_be_bytes());
        msg.push(player.world.as_u8());
        msg.push(if player.is_alive { 1u8 } else { 0u8 });
        msg.push(player.gun_level as u8);
        msg.push(player.team.as_u8());
        let wbytes = player.current_weapon.as_bytes();
        msg.push(wbytes.len() as u8);
        msg.extend_from_slice(wbytes);
        msg.push(if player.is_bot { 1u8 } else { 0u8 });
    }

    for (addr, p) in players.iter() {
        if p.is_bot {
            continue;
        }
        let _ = socket.send_to(&msg, *addr);
    }
}

/// Broadcast the current lobby leader's player ID to every connected client.
pub fn broadcast_lobby_leader(socket: &UdpSocket, players: &HashMap<SocketAddr, Player>) {
    let leader_id: u32 = crate::server::game::lobby_leader_addr(players)
        .and_then(|addr| players.get(&addr))
        .map(|p| p.id)
        .unwrap_or(0);
    let mut msg = vec![MSG_LOBBY_LEADER];
    msg.extend_from_slice(&leader_id.to_be_bytes());
    for (addr, p) in players.iter() {
        if p.is_bot {
            continue;
        }
        let _ = socket.send_to(&msg, *addr);
    }
}

/// Broadcast authoritative match-time remaining (seconds) to all connected clients
pub fn broadcast_match_timer(
    socket: &UdpSocket,
    players: &HashMap<SocketAddr, Player>,
    secs: u16,
) {
    let hi = (secs >> 8) as u8;
    let lo = (secs & 0xFF) as u8;
    let msg: [u8; 3] = [MSG_MATCH_TIMER, hi, lo];
    for (addr, p) in players.iter() {
        if p.is_bot {
            continue;
        }
        let _ = socket.send_to(&msg, *addr);
    }
}

/// Move all players to Lobby and optionally reset per-match scores/state
pub fn move_all_players_to_lobby(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    reset_scores: bool,
) {
    // Build the world-change-to-lobby message once; coords are the same for everyone.
    let mut wc = [0u8; 14];
    wc[0] = MSG_WORLD_CHANGE;
    wc[1] = WorldId::Lobby.as_u8();
    wc[2..6].copy_from_slice(&0.0f32.to_be_bytes());
    wc[6..10].copy_from_slice(&0.0f32.to_be_bytes());
    wc[10..14].copy_from_slice(&0.0f32.to_be_bytes());

    for (addr, p) in players.iter_mut() {
        if p.is_bot {
            continue;
        }
        p.world = WorldId::Lobby;
        p.position = (0.0_f32, 0.0_f32, 0.0_f32);
        if reset_scores {
            p.is_alive = true;
            p.health = p.max_health;
            p.kills = 0;
            p.gun_level = 1;
            p.death_time = None;
        }
        let _ = socket.send_to(&wc, *addr);
    }

    crate::server::game::auto_assign_lobby_teams(players);
    broadcast_state(socket, players);
    broadcast_player_list(socket, players);
}

/// Send a full `MSG_STATE` to every receiver; the message is tailored so that each
/// client only receives updates about players in its own world.
pub fn broadcast_state(socket: &UdpSocket, players: &HashMap<SocketAddr, Player>) {
    let mut state_msg: Vec<u8> = Vec::with_capacity(1024);
    let now = Instant::now();

    for (recv_addr, receiver) in players.iter() {
        if receiver.is_bot {
            continue;
        }
        state_msg.clear();
        state_msg.push(MSG_STATE);

        let count = players.values().filter(|p| p.world == receiver.world).count() as u32;
        state_msg.extend_from_slice(&count.to_be_bytes());

        for player in players.values() {
            if player.world != receiver.world {
                continue;
            }
            state_msg.extend_from_slice(&player.id.to_be_bytes());
            state_msg.extend_from_slice(&player.position.0.to_be_bytes());
            state_msg.extend_from_slice(&player.position.1.to_be_bytes());
            state_msg.extend_from_slice(&player.position.2.to_be_bytes());
            state_msg.extend_from_slice(&player.yaw.to_be_bytes());
            state_msg.push(player.world.as_u8());
            let mut state_flags = 0u8;
            if player.is_sprinting { state_flags |= FLAG_SPRINTING; }
            if player.is_crouched  { state_flags |= FLAG_CROUCHING; }
            if player.is_ads       { state_flags |= FLAG_ADS; }
            if player.is_airborne  { state_flags |= FLAG_AIRBORNE; }
            if player.is_prone     { state_flags |= FLAG_PRONE; }
            if player.is_sliding   { state_flags |= FLAG_SLIDING; }
            if player.is_bot       { state_flags |= FLAG_IS_BOT; }
            if player.is_silent    { state_flags |= FLAG_SILENT; }
            state_msg.push(state_flags);

            let mut event_flags = 0u8;
            if player.evt_jump_start { event_flags |= EVT_JUMP_START; }
            if player.evt_landed     { event_flags |= EVT_LANDED; }
            state_msg.push(event_flags);

            state_msg.extend_from_slice(&player.kills.to_be_bytes());
            state_msg.push(player.gun_level as u8);
            state_msg.extend_from_slice(&(player.health as f32).to_be_bytes());
            state_msg.extend_from_slice(&(player.max_health as f32).to_be_bytes());

            let name_bytes = player.username.as_bytes();
            state_msg.push(name_bytes.len() as u8);
            state_msg.extend_from_slice(name_bytes);
            state_msg.push(if player.is_alive { 1u8 } else { 0u8 });

            let freeze_secs: u8 = match player.post_teleport_freeze_until {
                Some(t) if t > now => t.saturating_duration_since(now).as_secs().min(255) as u8,
                _ => 0u8,
            };
            state_msg.push(freeze_secs);

            state_msg.push(player.team.as_u8());
            let wbytes = player.current_weapon.as_bytes();
            state_msg.push(wbytes.len() as u8);
            state_msg.extend_from_slice(wbytes);
        }

        let _ = socket.send_to(&state_msg, *recv_addr);
    }
}
