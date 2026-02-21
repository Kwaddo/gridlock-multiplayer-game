use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::Instant;

use crate::maze::MazeData;
use crate::player::{Player, WorldId};
use crate::protocols::*;

use crate::server::game;

/// Called when a client reports that they broke a wall at (x,y).
/// Update the maze and broadcast the event to all connected players.
pub fn handle_wall_broken(
    socket: &UdpSocket,
    maze: &mut MazeData,
    players: &HashMap<SocketAddr, Player>,
    _src_addr: SocketAddr,
    payload: &[u8],
) {
    if payload.len() < 2 {
        return;
    }
    let x = payload[0];
    let y = payload[1];

    if maze.break_wall(x, y) {
        let msg = [MSG_WALL_BROKEN, x, y];
        for p in players.values() {
            let _ = socket.send_to(&msg, p.socket_addr);
        }
    }
}

/// Respond to a LAN discovery packet by sending back a small summary of the server.
pub fn handle_discover(
    socket: &UdpSocket,
    players: &HashMap<SocketAddr, Player>,
    src_addr: SocketAddr,
) {
    let mut resp = Vec::with_capacity(64);
    resp.push(MSG_DISCOVER);
    resp.push(players.len() as u8);
    resp.push(game::MATCH_MAX_PLAYERS as u8);
    let name = "Multiplayer FPS";
    let name_bytes = name.as_bytes();
    resp.push(name_bytes.len() as u8);
    resp.extend_from_slice(name_bytes);
    let _ = socket.send_to(&resp, src_addr);
}

/// Process a hit report from a client.  This updates health/kill state and sends
/// appropriate MSG_DAMAGE/MSG_DEATH/MSG_KILL/MATCH_FEED messages.
pub fn handle_hit(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    src_addr: SocketAddr,
    payload: &[u8],
    _maze: &mut Option<MazeData>,
    _match_running: &mut bool,
    _match_start_time: &mut Option<Instant>,
    _match_end_pending: &mut Option<(Instant, u32, String)>,
    _match_mode: u8,
) {
    if payload.len() < 8 {
        return;
    }
    let victim_id = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let damage = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);

    let mut idx = 8;
    let weapon_name = if idx < payload.len() {
        let name_len = payload[idx] as usize;
        idx += 1;
        if idx + name_len <= payload.len() {
            String::from_utf8_lossy(&payload[idx..idx + name_len]).to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let attacker = match players.get(&src_addr) {
        Some(a) => a.socket_addr, // just capture addr to lookup later
        None => return,
    };

    // find victim by id (we iterate; could use map index if available)
    let victim_addr_opt = players
        .values()
        .find(|p| p.id == victim_id)
        .map(|p| p.socket_addr);

    if victim_addr_opt.is_none() {
        return;
    }
    let victim_addr = victim_addr_opt.unwrap();

    // borrow both attacker and victim separately to avoid double mutable borrow
    if let Some(victim) = players.get_mut(&victim_addr) {
        if !victim.is_alive {
            return;
        }

        if damage >= victim.health {
            // fatal blow
            victim.is_alive = false;
            victim.deaths += 1;

            // notify victim of death
            let mut death_msg = vec![MSG_DEATH];
            // attacker id
            if let Some(att) = players.get(&attacker) {
                death_msg.extend_from_slice(&att.id.to_be_bytes());
            } else {
                death_msg.extend_from_slice(&0u32.to_be_bytes());
            }
            let wbytes = weapon_name.as_bytes();
            death_msg.push(wbytes.len() as u8);
            death_msg.extend_from_slice(wbytes);
            let _ = socket.send_to(&death_msg, victim_addr);

            // reward killer
            if let Some(killer) = players.get_mut(&attacker) {
                killer.kills += 1;
                // send kill confirmation to killer
                let mut kill_msg = vec![MSG_KILL];
                kill_msg.extend_from_slice(&victim_id.to_be_bytes());
                kill_msg.push(wbytes.len() as u8);
                kill_msg.extend_from_slice(wbytes);
                let _ = socket.send_to(&kill_msg, attacker);
            }

            // broadcast kill feed
            let mut feed = vec![MSG_KILL_FEED];
            if let Some(killer) = players.get(&attacker) {
                feed.extend_from_slice(&killer.id.to_be_bytes());
            } else {
                feed.extend_from_slice(&0u32.to_be_bytes());
            }
            feed.extend_from_slice(&victim_id.to_be_bytes());
            for p in players.values() {
                let _ = socket.send_to(&feed, p.socket_addr);
            }
        } else {
            // non-lethal damage
            victim.health -= damage;
            let mut dmg_msg = vec![MSG_DAMAGE];
            dmg_msg.extend_from_slice(&damage.to_be_bytes());
            if let Some(att) = players.get(&attacker) {
                dmg_msg.extend_from_slice(&att.id.to_be_bytes());
            } else {
                dmg_msg.extend_from_slice(&0u32.to_be_bytes());
            }
            let _ = socket.send_to(&dmg_msg, victim_addr);
        }
    }
}

/// Respond to a respawn request by moving the player to a spawn point and
/// returning their new coordinates.
pub fn handle_respawn_request(
    socket: &UdpSocket,
    _maze: &MazeData,
    players: &mut HashMap<SocketAddr, Player>,
    src_addr: SocketAddr,
) {
    if let Some(player) = players.get_mut(&src_addr) {
        player.health = player.max_health;
        player.is_alive = true;

        // simple spawn logic from roadmap
        let spawn = match player.world {
            WorldId::Lobby => (0.0_f32, 0.5_f32, 0.0_f32),
            WorldId::Maze => {
                let spawn_x = 1.5 * game::MAZE_CELL_SIZE;
                let spawn_z = 1.5 * game::MAZE_CELL_SIZE;
                (spawn_x, 0.5, spawn_z)
            }
        };
        player.position = spawn;

        let mut resp = Vec::with_capacity(16);
        resp.push(MSG_RESPAWN);
        resp.extend_from_slice(&spawn.0.to_be_bytes());
        resp.extend_from_slice(&spawn.1.to_be_bytes());
        resp.extend_from_slice(&spawn.2.to_be_bytes());
        let _ = socket.send_to(&resp, src_addr);
    }
}
