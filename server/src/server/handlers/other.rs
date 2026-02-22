use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use crate::maze::MazeData;
use crate::player::{Player, WorldId};
use crate::protocols::*;

use crate::server::{broadcast, game};

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
    match_end_pending: &mut Option<(Instant, u32, String)>,
    match_mode: u8,
) {
    if payload.len() < 8 {
        return;
    }
    let victim_id = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
    // Client sends damage as f32 — decode the IEEE 754 bytes then convert to u32
    let damage_f = f32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
    let damage = damage_f.round().max(0.0) as u32;

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
        Some(a) => {
            // No damage allowed in the lobby
            if a.world == WorldId::Lobby {
                return;
            }
            a.socket_addr
        }
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

                // Gun Game progression: only advance when running in that mode
                if match_mode == MODE_GUN_GAME {
                    killer.gun_level = killer.gun_level.saturating_add(1);

                    // check for victory (level beyond progression length)
                    if (killer.gun_level as usize) > game::GUN_GAME_PROGRESSION.len() {
                        // announce winner and schedule end
                        let winner_id = killer.id;
                        let winner_name = killer.username.clone();
                        println!("Gun Game winner reached final level: {}", winner_name);

                        let mut end_msg = vec![MSG_MATCH_END];
                        end_msg.extend_from_slice(&winner_id.to_be_bytes());
                        let name_bytes = winner_name.as_bytes();
                        end_msg.push(name_bytes.len() as u8);
                        end_msg.extend_from_slice(name_bytes);
                        // include countdown byte so clients can show timer
                        end_msg.push(game::MATCH_END_COUNTDOWN_SECS);
                        for p in players.values() {
                            let _ = socket.send_to(&end_msg, p.socket_addr);
                        }

                        *match_end_pending = Some((
                            Instant::now()
                                + Duration::from_secs(game::MATCH_END_COUNTDOWN_SECS as u64),
                            winner_id,
                            winner_name,
                        ));
                    }
                }

                // send kill confirmation to killer
                let mut kill_msg = vec![MSG_KILL];
                kill_msg.extend_from_slice(&victim_id.to_be_bytes());
                kill_msg.push(wbytes.len() as u8);
                kill_msg.extend_from_slice(wbytes);
                let _ = socket.send_to(&kill_msg, attacker);

                // immediately propagate score/gun-level changes to everyone so the
                // scoreboard/weapon lock updates promptly instead of waiting for the
                // next position packet
                broadcast::broadcast_state(socket, players);
                broadcast::broadcast_player_list(socket, players);
            }

            // broadcast kill feed (include weapon name so client can display it)
            let mut feed = vec![MSG_KILL_FEED];
            if let Some(killer) = players.get(&attacker) {
                feed.extend_from_slice(&killer.id.to_be_bytes());
            } else {
                feed.extend_from_slice(&0u32.to_be_bytes());
            }
            feed.extend_from_slice(&victim_id.to_be_bytes());
            feed.push(wbytes.len() as u8);
            feed.extend_from_slice(wbytes);
            for p in players.values() {
                let _ = socket.send_to(&feed, p.socket_addr);
            }
        } else {
            // non-lethal damage
            victim.health -= damage;
            let mut dmg_msg = vec![MSG_DAMAGE];
            // Send damage as f32 to match client's f32::from_be_bytes() decoding
            dmg_msg.extend_from_slice(&(damage as f32).to_be_bytes());
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
    maze: &MazeData,
    players: &mut HashMap<SocketAddr, Player>,
    src_addr: SocketAddr,
) {
    if let Some(player) = players.get_mut(&src_addr) {
        player.health = player.max_health;
        player.is_alive = true;

        // choose a spawn position based on current world. the maze argument
        // is provided so we can pick a random safe location instead of always
        // teleporting to the same corner.
        let spawn = if player.world == WorldId::Maze {
            let (sx, sy, sz, _fallback) = maze.random_spawn_world(game::MAZE_CELL_SIZE);
            (sx, sy, sz)
        } else {
            // lobby players respawn at a fixed lobby coordinate
            (0.0_f32, 0.5_f32, 0.0_f32)
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
