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
            // If the attacker is in their per-player post-teleport freeze, ignore hits
            if a.post_teleport_freeze_until
                .map_or(false, |t| t > Instant::now())
                && a.world == WorldId::Maze
            {
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

    // avoid holding multiple simultaneous mutable borrows of `players` by doing
    // short-lived immutable checks, then separate mutable borrows for victim and
    // killer. We'll record if the victim needs to be reset after killer handling.
    // First, perform immutable checks on the victim
    {
        let victim_ref = match players.get(&victim_addr) {
            Some(v) => v,
            None => return,
        };
        // If the victim is in their per-player post-teleport freeze, ignore hits
        if victim_ref
            .post_teleport_freeze_until
            .map_or(false, |t| t > Instant::now())
            && victim_ref.world == WorldId::Maze
        {
            return;
        }
        if !victim_ref.is_alive {
            return;
        }
    }

    if damage >= players.get(&victim_addr).map(|v| v.health).unwrap_or(0) {
        // fatal blow: mutate victim in a short scope and send death message
        let wbytes = weapon_name.as_bytes();

        {
            if let Some(vmut) = players.get_mut(&victim_addr) {
                vmut.is_alive = false;
                vmut.deaths += 1;
            }
        }

        // notify victim of death (include attacker id if known)
        let mut death_msg = vec![MSG_DEATH];
        if let Some(att) = players.get(&attacker) {
            death_msg.extend_from_slice(&att.id.to_be_bytes());
        } else {
            death_msg.extend_from_slice(&0u32.to_be_bytes());
        }
        death_msg.push(wbytes.len() as u8);
        death_msg.extend_from_slice(wbytes);
        let _ = socket.send_to(&death_msg, victim_addr);

        // reward killer — separate mutable borrow
        let mut victim_reset_required = false;
        if let Some(killer) = players.get_mut(&attacker) {
            killer.kills += 1;

            if match_mode == MODE_GUN_GAME {
                let is_knife = weapon_name.eq_ignore_ascii_case("knife");
                // Define the special knife level as one past the gun progression.
                let final_knife_level = (game::GUN_GAME_PROGRESSION.len() + 1) as u32;
                if is_knife {
                    // Knife only triggers an immediate win when the killer is on the
                    // dedicated knife level (final_knife_level). Otherwise a knife
                    // kill simply sets the victim back to the first gun.
                    if killer.gun_level == final_knife_level {
                        let winner_id = killer.id;
                        let winner_name = killer.username.clone();
                        println!("Gun Game winner (knife final): {}", winner_name);

                        let mut end_msg = vec![MSG_MATCH_END];
                        end_msg.extend_from_slice(&winner_id.to_be_bytes());
                        let name_bytes = winner_name.as_bytes();
                        end_msg.push(name_bytes.len() as u8);
                        end_msg.extend_from_slice(name_bytes);
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
                    } else {
                        // non-final knife: mark victim to be reset after we drop this mutable borrow
                        victim_reset_required = true;
                    }
                } else {
                    // Non-knife kills advance the killer along the progression.
                    // Allow progression to reach the special knife level (one past
                    // the last gun) so the player becomes 'knife-only'.
                    if (killer.gun_level as usize) < game::GUN_GAME_PROGRESSION.len() + 1 {
                        killer.gun_level = killer.gun_level.saturating_add(1);
                    }
                }
            }

            // send kill confirmation to killer
            let mut kill_msg = vec![MSG_KILL];
            kill_msg.extend_from_slice(&victim_id.to_be_bytes());
            kill_msg.push(wbytes.len() as u8);
            kill_msg.extend_from_slice(wbytes);
            let _ = socket.send_to(&kill_msg, attacker);

            // propagate score/gun-level changes
            broadcast::broadcast_state(socket, players);
            broadcast::broadcast_player_list(socket, players);
        }

        // If needed, reset victim's gun level now that the killer borrow has ended
        if victim_reset_required {
            if let Some(vmut) = players.get_mut(&victim_addr) {
                vmut.gun_level = 1;
            }
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
        // non-lethal damage: apply mutation in a short scope and send damage msg
        if let Some(vmut) = players.get_mut(&victim_addr) {
            vmut.health -= damage;
        }
        let mut dmg_msg = vec![MSG_DAMAGE];
        dmg_msg.extend_from_slice(&(damage as f32).to_be_bytes());
        if let Some(att) = players.get(&attacker) {
            dmg_msg.extend_from_slice(&att.id.to_be_bytes());
        } else {
            dmg_msg.extend_from_slice(&0u32.to_be_bytes());
        }
        let _ = socket.send_to(&dmg_msg, victim_addr);
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
