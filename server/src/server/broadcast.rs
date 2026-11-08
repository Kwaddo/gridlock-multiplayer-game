use std::collections::HashMap;
use crate::net::{Outbox, PeerId};
use platform::Instant;

use crate::player::{Player, WorldId};
use crate::protocols::*;

/// Send one packet to every human player.
pub fn to_humans(socket: &Outbox, players: &HashMap<PeerId, Player>, msg: &[u8]) {
    for (addr, p) in players.iter() {
        if !p.is_bot {
            let _ = socket.send_to(msg, *addr);
        }
    }
}

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
/// Also broadcasts the host so clients can update the host indicator.
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

/// Broadcast the host's player ID to every connected client.
pub fn broadcast_lobby_leader(socket: &Outbox, players: &HashMap<PeerId, Player>) {
    let leader_id: u32 = crate::server::game::host_addr(players)
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
///
/// One-shot movement events are left out: they go out once, with the periodic
/// broadcast in `Server::tick` ([`broadcast_state_with_events`]), which then
/// clears them. Sending them here too would replay them on the next tick.
pub fn broadcast_state(socket: &Outbox, players: &HashMap<PeerId, Player>) {
    send_state(socket, players, false);
}

/// [`broadcast_state`] including pending movement events; the caller clears them.
pub fn broadcast_state_with_events(socket: &Outbox, players: &HashMap<PeerId, Player>) {
    send_state(socket, players, true);
}

fn send_state(socket: &Outbox, players: &HashMap<PeerId, Player>, include_events: bool) {
    let now = Instant::now();
    // The message only depends on the receiver's world: encode it once per
    // world (at most a couple) and send the same bytes to everyone in it.
    let mut by_world: Vec<(WorldId, Vec<u8>)> = Vec::with_capacity(2);

    for (recv_addr, receiver) in players.iter() {
        if receiver.is_bot {
            continue;
        }
        let at = match by_world.iter().position(|(w, _)| *w == receiver.world) {
            Some(at) => at,
            None => {
                by_world.push((receiver.world, state_msg(players, receiver.world, include_events, now)));
                by_world.len() - 1
            }
        };
        let _ = socket.send_to(&by_world[at].1, *recv_addr);
    }
}

/// The encoded `MSG_STATE` for a receiver in `world`: every player in that
/// world, in the map's iteration order.
pub(crate) fn state_msg(
    players: &HashMap<PeerId, Player>,
    world: WorldId,
    include_events: bool,
    now: Instant,
) -> Vec<u8> {
    let entries = players
        .values()
        .filter(|p| p.world == world)
        .map(|p| player_state(p, include_events, now))
        .collect();
    ServerMsg::State { world: world.as_u8(), players: entries }.encode()
}

/// One player's `MSG_STATE` entry. What rarely changes goes in `MSG_PLAYER_INFO`.
pub(crate) fn player_state(player: &Player, include_events: bool, now: Instant) -> PlayerState {
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
    if include_events {
        if player.evt_jump_start { event_flags |= EVT_JUMP_START; }
        if player.evt_landed     { event_flags |= EVT_LANDED; }
    }

    let freeze_secs: u8 = match player.post_teleport_freeze_until {
        Some(t) if t > now => t.saturating_duration_since(now).as_secs().min(255) as u8,
        _ => 0u8,
    };

    PlayerState {
        id: player.id,
        position: player.position,
        yaw: player.yaw,
        state_flags,
        event_flags,
        health: player.health.min(u16::MAX as u32) as u16,
        alive: player.is_alive,
        freeze_secs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::BOT_PEER_BASE;
    use crate::player::Team;
    use std::time::Duration;

    /// The previous `send_state`: one message built and encoded per receiver.
    fn per_receiver(players: &HashMap<PeerId, Player>, include_events: bool) -> Vec<(PeerId, Vec<u8>)> {
        let now = Instant::now();
        let mut out = Vec::new();
        for (recv_addr, receiver) in players.iter() {
            if receiver.is_bot {
                continue;
            }
            let entries = players
                .values()
                .filter(|p| p.world == receiver.world)
                .map(|p| player_state(p, include_events, now))
                .collect();
            let world = receiver.world.as_u8();
            out.push((*recv_addr, ServerMsg::State { world, players: entries }.encode()));
        }
        out
    }

    #[test]
    fn shared_per_world_state_equals_per_receiver_state() {
        let mut players = HashMap::new();
        for i in 0..12u32 {
            let bot = i % 4 == 3;
            let addr = if bot { PeerId(BOT_PEER_BASE + i) } else { PeerId(i + 1) };
            let mut p = Player::new(i + 1, format!("player{}", i));
            p.is_bot = bot;
            p.world = if i % 3 == 0 { WorldId::Lobby } else { WorldId::Maze };
            p.team = if i % 2 == 0 { Team::Red } else { Team::Blue };
            p.position = (i as f32 * 1.5, 0.25, -(i as f32));
            p.yaw = i as f32 * 0.3;
            p.kills = i;
            p.is_sprinting = i % 2 == 1;
            p.is_crouched = i % 5 == 0;
            p.is_alive = i != 7;
            p.evt_jump_start = i % 3 == 1;
            p.evt_landed = i % 4 == 2;
            p.current_weapon = if i % 2 == 0 { "M4A1".into() } else { "Shotgun".into() };
            // far enough off that both versions round it the same
            p.post_teleport_freeze_until = (i % 4 == 1).then(|| Instant::now() + Duration::from_millis(5500));
            players.insert(addr, p);
        }
        for include_events in [false, true] {
            let outbox = Outbox::default();
            send_state(&outbox, &players, include_events);
            let shared = outbox.drain();
            assert_eq!(shared, per_receiver(&players, include_events));
            assert_eq!(shared.len(), players.values().filter(|p| !p.is_bot).count());
        }
    }
}
