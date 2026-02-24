use std::collections::HashMap;
use std::net::UdpSocket;
use std::time::Instant;

use crate::player::{Player, WorldId};
use crate::protocols::*;

/// Send the full connected player list to a single destination
#[allow(dead_code)]
pub fn send_player_list(
    socket: &UdpSocket,
    players: &HashMap<std::net::SocketAddr, Player>,
    dest: std::net::SocketAddr,
) {
    // format: [MSG_PLAYER_LIST][count:1][entries...]
    // entry: [id:4][name_len:1][name bytes][kills:4][deaths:4][world:1][alive:1][gun_level:1]
    // entry: [id:4][name_len:1][name bytes][kills:4][deaths:4][world:1][alive:1][gun_level:1][team:1][weapon_len:1][weapon bytes]
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
    }
    let _ = socket.send_to(&msg, dest);
}

/// Broadcast updated player list to every connected client
pub fn broadcast_player_list(socket: &UdpSocket, players: &HashMap<std::net::SocketAddr, Player>) {
    // build a single buffer and reuse; send_player_list currently allocates each time,
    // inline the logic here for efficiency.
    let mut msg: Vec<u8> = Vec::with_capacity(1024);

    for p in players.values() {
        msg.clear();
        let count = players.len().min(255) as u8;
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
        }
        let _ = socket.send_to(&msg, p.socket_addr);
    }
}

/// Broadcast authoritative match-time remaining (seconds) to all connected clients
pub fn broadcast_match_timer(
    socket: &UdpSocket,
    players: &HashMap<std::net::SocketAddr, Player>,
    secs: u8,
) {
    // fixed-size array avoids allocation
    let msg: [u8; 2] = [MSG_MATCH_TIMER, secs];
    for p in players.values() {
        let _ = socket.send_to(&msg, p.socket_addr);
    }
}

/// Move all players to Lobby and optionally reset per-match scores/state
pub fn move_all_players_to_lobby(
    socket: &UdpSocket,
    players: &mut HashMap<std::net::SocketAddr, Player>,
    reset_scores: bool,
) {
    for p in players.values_mut() {
        p.world = WorldId::Lobby;
        p.position = (0.0_f32, 0.0_f32, 0.0_f32);
        if reset_scores {
            p.is_alive = true;
            p.health = p.max_health;
            p.shield = 0;
            p.kills = 0;
            p.gun_level = 1; // reset Gun Game progression
            p.death_time = None;
        }

        // notify client of world change
        // world change to lobby: fixed buffer
        let mut wc = [0u8; 14];
        wc[0] = MSG_WORLD_CHANGE;
        wc[1] = WorldId::Lobby.as_u8();
        wc[2..6].copy_from_slice(&0.0f32.to_be_bytes());
        wc[6..10].copy_from_slice(&0.0f32.to_be_bytes());
        wc[10..14].copy_from_slice(&0.0f32.to_be_bytes());
        let _ = socket.send_to(&wc, p.socket_addr);
    }

    // re-balance lobby teams now that all players are back in lobby
    crate::server::game::auto_assign_lobby_teams(players);
    // broadcast final state and player-list so clients update immediately
    broadcast_state(socket, players);
    broadcast_player_list(socket, players);
}

/// Send a full `MSG_STATE` to every receiver; the message is tailored so that each
/// client only receives updates about players in its own world.
pub fn broadcast_state(socket: &UdpSocket, players: &HashMap<std::net::SocketAddr, Player>) {
    // build a single reusable buffer to reduce heap churn
    let mut state_msg: Vec<u8> = Vec::with_capacity(1024);
    for receiver in players.values() {
        state_msg.clear();
        state_msg.push(MSG_STATE);

        // first pass: count matching players to write header
        // include players in the same world even if they're dead so clients
        // receive "alive" status updates for offscreen deaths.
        let mut count: u32 = 0;
        for p in players.values() {
            if p.world != receiver.world {
                continue;
            }
            count += 1;
        }
        state_msg.extend_from_slice(&count.to_be_bytes());

        let now = Instant::now();

        // second pass: append each player's data directly
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
            if player.is_sprinting {
                state_flags |= FLAG_SPRINTING;
            }
            if player.is_crouched {
                state_flags |= FLAG_CROUCHING;
            }
            if player.is_ads {
                state_flags |= FLAG_ADS;
            }
            if player.is_airborne {
                state_flags |= FLAG_AIRBORNE;
            }
            if player.is_prone {
                state_flags |= FLAG_PRONE;
            }
            state_msg.push(state_flags);

            let mut event_flags = 0u8;
            if player.evt_jump_start {
                event_flags |= EVT_JUMP_START;
            }
            if player.evt_landed {
                event_flags |= EVT_LANDED;
            }
            state_msg.push(event_flags);

            // include kills (score) so clients can render scoreboard
            state_msg.extend_from_slice(&player.kills.to_be_bytes());

            // include gun progression level for Gun Game (1 = first weapon)
            state_msg.push(player.gun_level as u8);

            // include health and max_health so other clients can render health bars
            state_msg.extend_from_slice(&(player.health as f32).to_be_bytes());
            state_msg.extend_from_slice(&(player.max_health as f32).to_be_bytes());

            let name_bytes = player.username.as_bytes();
            state_msg.push(name_bytes.len() as u8);
            state_msg.extend_from_slice(name_bytes);
            // include alive flag so clients learn of deaths even when offscreen
            state_msg.push(if player.is_alive { 1u8 } else { 0u8 });
            // include remaining post-teleport freeze seconds so clients can
            // recover authoritative freeze state if they missed the world-change
            // packet. Represent as a single byte (0 = no freeze).
            let freeze_secs: u8 = match player.post_teleport_freeze_until {
                Some(t) if t > now => t.saturating_duration_since(now).as_secs().min(255) as u8,
                _ => 0u8,
            };
            state_msg.push(freeze_secs);

            // include team and current primary weapon
            state_msg.push(player.team.as_u8());
            let wbytes = player.current_weapon.as_bytes();
            state_msg.push(wbytes.len() as u8);
            state_msg.extend_from_slice(wbytes);
        }

        let _ = socket.send_to(&state_msg, receiver.socket_addr);
    }
}
