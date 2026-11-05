use std::collections::HashMap;
use crate::net::{Outbox, PeerId};
use platform::Instant;

use crate::player::{Player, WorldId};
use crate::protocols::*;

/// Send the full connected player list to a single destination
#[allow(dead_code)]
pub fn send_player_list(
    socket: &Outbox,
    players: &HashMap<PeerId, Player>,
    dest: PeerId,
) {
    let _ = socket.send_to(&player_list_msg(players), dest);
}

fn player_list_msg(players: &HashMap<PeerId, Player>) -> Vec<u8> {
    let entries = players
        .values()
        .map(|p| PlayerListEntry {
            id: p.id,
            name: p.username.clone(),
            kills: p.kills,
            deaths: p.deaths,
            world: p.world.as_u8(),
            alive: p.is_alive,
            gun_level: p.gun_level as u8,
            team: p.team.as_u8(),
            weapon: p.current_weapon.clone(),
            is_bot: p.is_bot,
        })
        .collect();
    ServerMsg::PlayerList(entries).encode()
}

/// Broadcast updated player list to every connected client.
/// Also broadcasts the current lobby leader so clients can update the host indicator.
pub fn broadcast_player_list(socket: &Outbox, players: &HashMap<PeerId, Player>) {
    broadcast_lobby_leader(socket, players);

    // Build the message once; the content is identical for every receiver.
    let msg = player_list_msg(players);

    for (addr, p) in players.iter() {
        if p.is_bot {
            continue;
        }
        let _ = socket.send_to(&msg, *addr);
    }
}

/// Broadcast the current lobby leader's player ID to every connected client.
pub fn broadcast_lobby_leader(socket: &Outbox, players: &HashMap<PeerId, Player>) {
    let leader_id: u32 = crate::server::game::lobby_leader_addr(players)
        .and_then(|addr| players.get(&addr))
        .map(|p| p.id)
        .unwrap_or(0);
    let msg = ServerMsg::LobbyLeader { id: leader_id }.encode();
    for (addr, p) in players.iter() {
        if p.is_bot {
            continue;
        }
        let _ = socket.send_to(&msg, *addr);
    }
}

/// Broadcast authoritative match-time remaining (seconds) to all connected clients
pub fn broadcast_match_timer(
    socket: &Outbox,
    players: &HashMap<PeerId, Player>,
    secs: u16,
) {
    let msg = ServerMsg::MatchTimer { secs }.encode();
    for (addr, p) in players.iter() {
        if p.is_bot {
            continue;
        }
        let _ = socket.send_to(&msg, *addr);
    }
}

/// Move all players to Lobby and optionally reset per-match scores/state
pub fn move_all_players_to_lobby(
    socket: &Outbox,
    players: &mut HashMap<PeerId, Player>,
    reset_scores: bool,
) {
    // Build the world-change-to-lobby message once; coords are the same for everyone.
    let wc = ServerMsg::WorldChange {
        world: WorldId::Lobby.as_u8(),
        spawn: (0.0, 0.0, 0.0),
        mode: None,
        freeze_secs: None,
        match_timer: None,
    }
    .encode();

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
pub fn broadcast_state(socket: &Outbox, players: &HashMap<PeerId, Player>) {
    let now = Instant::now();

    for (recv_addr, receiver) in players.iter() {
        if receiver.is_bot {
            continue;
        }

        let mut entries = Vec::new();
        for player in players.values() {
            if player.world != receiver.world {
                continue;
            }
            let mut state_flags = 0u8;
            if player.is_sprinting { state_flags |= FLAG_SPRINTING; }
            if player.is_crouched  { state_flags |= FLAG_CROUCHING; }
            if player.is_ads       { state_flags |= FLAG_ADS; }
            if player.is_airborne  { state_flags |= FLAG_AIRBORNE; }
            if player.is_prone     { state_flags |= FLAG_PRONE; }
            if player.is_sliding   { state_flags |= FLAG_SLIDING; }
            if player.is_bot       { state_flags |= FLAG_IS_BOT; }
            if player.is_silent    { state_flags |= FLAG_SILENT; }

            let mut event_flags = 0u8;
            if player.evt_jump_start { event_flags |= EVT_JUMP_START; }
            if player.evt_landed     { event_flags |= EVT_LANDED; }

            let freeze_secs: u8 = match player.post_teleport_freeze_until {
                Some(t) if t > now => t.saturating_duration_since(now).as_secs().min(255) as u8,
                _ => 0u8,
            };

            entries.push(PlayerState {
                id: player.id,
                position: player.position,
                yaw: player.yaw,
                world: player.world.as_u8(),
                state_flags,
                event_flags,
                kills: player.kills,
                gun_level: player.gun_level as u8,
                health: player.health as f32,
                max_health: player.max_health as f32,
                name: player.username.clone(),
                alive: player.is_alive,
                freeze_secs,
                team: player.team.as_u8(),
                weapon: player.current_weapon.clone(),
            });
        }
        let state_msg = ServerMsg::State(entries).encode();

        let _ = socket.send_to(&state_msg, *recv_addr);
    }
}
