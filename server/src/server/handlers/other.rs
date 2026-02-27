use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use crate::bot::personality::BotArchetype;
use crate::maze::MazeData;
use crate::player::{Player, Team, WorldId};
use crate::protocols::*;
use crate::server::{broadcast, game};
use crate::server::game::MAZE_CELL_SIZE;

const BACKSTAB_CONE_DOT: f32 = 0.5;

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
            if p.is_bot {
                continue;
            }
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

/// Process a hit report from a client. This function parses payload and forwards
/// to shared damage pipeline so bots can reuse the same logic.
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
    snd_state: &mut Option<crate::server::snd::SndState>,
    bot_states: &mut HashMap<SocketAddr, crate::bot::ai::BotState>,
) {
    // the sender of the hit packet is the attacker
    let attacker = src_addr;
    if payload.len() < 8 {
        return;
    }

    let victim_id = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let damage_f = f32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
    let damage = damage_f.round().max(0.0) as u32;

    let mut idx = 8;
    let weapon_name = if idx < payload.len() {
        let name_len = payload[idx] as usize;
        idx += 1;
        if idx + name_len <= payload.len() {
            let name = String::from_utf8_lossy(&payload[idx..idx + name_len]).to_string();
            idx += name_len;
            name
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let is_backstab_claim = if idx < payload.len() {
        payload[idx] != 0
    } else {
        false
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

    let backstab_verified = if is_backstab_claim && weapon_name.eq_ignore_ascii_case("knife") {
        let atk_yaw = players.get(&src_addr).map(|a| a.yaw).unwrap_or(0.0);
        let vic_yaw = players.get(&victim_addr).map(|v| v.yaw).unwrap_or(0.0);
        let atk_fwd = (atk_yaw.cos(), atk_yaw.sin());
        let vic_fwd = (vic_yaw.cos(), vic_yaw.sin());
        let dot = atk_fwd.0 * vic_fwd.0 + atk_fwd.1 * vic_fwd.1;
        dot > BACKSTAB_CONE_DOT
    } else {
        false
    };
    let damage = if backstab_verified {
        players
            .get(&victim_addr)
            .map(|v| v.health.saturating_add(1))
            .unwrap_or(damage)
    } else {
        damage
    };

    if damage >= players.get(&victim_addr).map(|v| v.health).unwrap_or(0) {
        // fatal blow: mutate victim in a short scope and send death message
        let wbytes = weapon_name.as_bytes();

        // pick a new primary for the victim (team modes) — unused in Gun Game
        let _chosen_weapon_for_victim = if let Some(vref) = players.get(&victim_addr) {
            if vref.team != Team::Spectator {
                game::choose_team_weapon_for_player(players, vref)
            } else {
                "Knife".to_string()
            }
        } else {
            "Knife".to_string()
        };

        {
            if let Some(vmut) = players.get_mut(&victim_addr) {
                vmut.is_alive = false;
                vmut.deaths += 1;
            }
        }

        // S&D: notify the round state that a player died (drops bomb if they had it)
        if match_mode == MODE_SEARCH_AND_DESTROY {
            if let Some(ref mut snd) = snd_state {
                // Get victim position for bomb drop
                let victim_pos = players
                    .get(&victim_addr)
                    .map(|v| v.position)
                    .unwrap_or((0.0, 0.0, 0.0));
                snd.on_player_death(victim_id, victim_pos);
                // Also cancel any defuse/plant if the dead player was doing it
                snd.stop_plant(victim_addr);
                snd.stop_defuse(victim_addr);
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
                // gun_level 1..19 = guns from GUN_GAME_PROGRESSION, 20 = knife-only level
                let final_knife_level = (game::GUN_GAME_PROGRESSION.len() + 1) as u32;
                if is_knife {
                    if killer.gun_level == final_knife_level {
                        // Knife kill on final level = win the game
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
                        // Non-final knife kill: reset victim to first weapon
                        victim_reset_required = true;
                    }
                } else {
                    // Non-knife kill: advance killer along the progression
                    if (killer.gun_level as usize) < game::GUN_GAME_PROGRESSION.len() + 1 {
                        killer.gun_level = killer.gun_level.saturating_add(1);
                        // Update the killer's current_weapon to match new gun_level
                        let new_level = killer.gun_level as usize;
                        if new_level <= game::GUN_GAME_PROGRESSION.len() {
                            killer.current_weapon =
                                game::GUN_GAME_PROGRESSION[new_level - 1].to_string();
                        } else {
                            // Reached knife-only level
                            killer.current_weapon = "Knife".to_string();
                        }
                    }
                }
            }

            // send kill confirmation to killer
            let mut kill_msg = vec![MSG_KILL];
            kill_msg.extend_from_slice(&victim_id.to_be_bytes());
            kill_msg.push(wbytes.len() as u8);
            kill_msg.extend_from_slice(wbytes);
            let _ = socket.send_to(&kill_msg, attacker);
        }

        // If needed, reset victim's gun level now that the killer borrow has ended
        if victim_reset_required {
            if let Some(vmut) = players.get_mut(&victim_addr) {
                vmut.gun_level = 1;
                // Reset to the first Gun Game weapon
                vmut.current_weapon = game::GUN_GAME_PROGRESSION[0].to_string();
            }
        }

        // propagate score/gun-level changes (after victim reset so clients see updated gun_level)
        broadcast::broadcast_state(socket, players);
        broadcast::broadcast_player_list(socket, players);

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
     
    apply_damage(
        socket,
        players,
        src_addr,
        victim_id,
        damage,
        &weapon_name,
        is_backstab_claim,
        match_end_pending,
        match_mode,
        bot_states,
        _maze.as_ref(), // pass immutable reference for any maze queries during damage application
    );
}

/// Respond to a respawn request by moving the player to a spawn point and
/// returning their new coordinates.
pub fn handle_respawn_request(
    socket: &UdpSocket,
    maze: Option<&MazeData>,
    players: &mut HashMap<SocketAddr, Player>,
    src_addr: SocketAddr,
    match_mode: u8,
    snd_state: &Option<crate::server::snd::SndState>,
) {
    // in Search & Destroy mode, disallow manual respawn until a new round begins
    if match_mode == MODE_SEARCH_AND_DESTROY {
        if let Some(snd) = snd_state {
            if snd.phase != crate::server::snd::SndPhase::FreezeTime {
                // either active or round over, ignore request
                return;
            }
        }
    }
    // compute spawn first using only immutable access to `players`
    let spawn = if let Some(pref) = players.get(&src_addr) {
        if pref.world == WorldId::Maze {
            match pref.team {
                Team::Red => {
                    let (red, _blue) = maze.unwrap().team_spawn_positions(game::MAZE_CELL_SIZE);
                    (red.0, red.1, red.2)
                }
                Team::Blue => {
                    let (_red, blue) = maze.unwrap().team_spawn_positions(game::MAZE_CELL_SIZE);
                    (blue.0, blue.1, blue.2)
                }
                _ => {
                    let (sx, sy, sz, _fallback) =
                        game::random_spawn_far(maze.unwrap(), players, game::MAZE_CELL_SIZE * 3.0);
                    (sx, sy, sz)
                }
            }
        } else {
            (0.0_f32, 0.5_f32, 0.0_f32)
        }
    } else {
        (0.0_f32, 0.5_f32, 0.0_f32)
    };

    if let Some(player) = players.get_mut(&src_addr) {
        player.health = player.max_health;
        player.is_alive = true;
        player.position = spawn;

        let mut resp = Vec::with_capacity(16);
        resp.push(MSG_RESPAWN);
        resp.extend_from_slice(&spawn.0.to_be_bytes());
        resp.extend_from_slice(&spawn.1.to_be_bytes());
        resp.extend_from_slice(&spawn.2.to_be_bytes());
        let _ = socket.send_to(&resp, src_addr);
    }

    // Determine and assign a weapon without holding overlapping borrows.
    // In Gun Game, the weapon is determined by gun_level, not by team loadout.
    let chosen = if let Some(pref) = players.get(&src_addr) {
        let gl = pref.gun_level as usize;
        if gl >= 1 && gl <= game::GUN_GAME_PROGRESSION.len() {
            // Gun Game: assign weapon matching current progression level
            game::GUN_GAME_PROGRESSION[gl - 1].to_string()
        } else if gl > game::GUN_GAME_PROGRESSION.len() {
            "Knife".to_string()
        } else if pref.team != Team::Spectator {
            game::choose_team_weapon_for_player(players, pref)
        } else {
            "Knife".to_string()
        }
    } else {
        "Knife".to_string()
    };

    if let Some(player) = players.get_mut(&src_addr) {
        player.current_weapon = chosen;
    }
}

/// Handle a client-request to set their preferred team while in the lobby.
pub fn handle_set_team(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    src_addr: SocketAddr,
    payload: &[u8],
) {
    if payload.is_empty() {
        return;
    }
    let team_byte = payload[0];
    let team = Team::from_u8(team_byte).unwrap_or(Team::Spectator);

    if let Some(player) = players.get_mut(&src_addr) {
        if player.world == WorldId::Lobby {
            player.team = team;
            broadcast::broadcast_player_list(socket, players);
        }
    }
}

pub fn apply_damage(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    attacker_addr: SocketAddr,
    victim_id: u32,
    damage: u32,
    weapon_name: &str,
    is_backstab: bool,
    match_end_pending: &mut Option<(Instant, u32, String)>,
    match_mode: u8,
    bot_states: &mut HashMap<SocketAddr, crate::bot::ai::BotState>,
    maze: Option<&MazeData>,
) {
    let (attacker, attacker_world, attacker_is_bot) = match players.get(&attacker_addr) {
        Some(a) => {
            if a.post_teleport_freeze_until
                .map_or(false, |t| t > Instant::now())
                && a.world == WorldId::Maze
            {
                return;
            }
            (a.socket_addr, a.world, a.is_bot)
        }
        None => return,
    };

    let victim_addr = match players
        .values()
        .find(|p| p.id == victim_id)
        .map(|p| p.socket_addr)
    {
        Some(addr) => addr,
        None => return,
    };

    {
        let victim_ref = match players.get(&victim_addr) {
            Some(v) => v,
            None => return,
        };
        if victim_ref.world != attacker_world {
            return;
        }
        // In lobby, allow only player<->bot combat (no player<->player).
        if attacker_world == WorldId::Lobby && !(attacker_is_bot || victim_ref.is_bot) {
            return;
        }
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

    let backstab_verified = if is_backstab && weapon_name.eq_ignore_ascii_case("knife") {
        let atk_yaw = players.get(&attacker_addr).map(|a| a.yaw).unwrap_or(0.0);
        let vic_yaw = players.get(&victim_addr).map(|v| v.yaw).unwrap_or(0.0);
        let atk_fwd = (atk_yaw.cos(), atk_yaw.sin());
        let vic_fwd = (vic_yaw.cos(), vic_yaw.sin());
        let dot = atk_fwd.0 * vic_fwd.0 + atk_fwd.1 * vic_fwd.1;
        dot > BACKSTAB_CONE_DOT
    } else {
        false
    };

    let mut damage = if backstab_verified {
        players
            .get(&victim_addr)
            .map(|v| v.health.saturating_add(1))
            .unwrap_or(damage)
    } else {
        damage
    };

    // Lobby rule: players are invincible to death in lobby; clamp at 1 HP.
    if let Some(victim_ref) = players.get(&victim_addr) {
        if attacker_world == WorldId::Lobby && !victim_ref.is_bot {
            damage = damage.min(victim_ref.health.saturating_sub(1));
        }
    }

    if damage == 0 {
        return;
    }

    if damage >= players.get(&victim_addr).map(|v| v.health).unwrap_or(0) {
        let wbytes = weapon_name.as_bytes();

        let chosen_weapon_for_victim = if let Some(vref) = players.get(&victim_addr) {
            if vref.team != Team::Spectator {
                game::choose_team_weapon_for_player(players, vref)
            } else {
                "Knife".to_string()
            }
        } else {
            "Knife".to_string()
        };

        if let Some(vmut) = players.get_mut(&victim_addr) {
            vmut.is_alive = false;
            vmut.deaths += 1;
            vmut.death_time = Some(Instant::now());
        }

        let mut death_msg = vec![MSG_DEATH];
        if let Some(att) = players.get(&attacker) {
            death_msg.extend_from_slice(&att.id.to_be_bytes());
        } else {
            death_msg.extend_from_slice(&0u32.to_be_bytes());
        }
        death_msg.push(wbytes.len() as u8);
        death_msg.extend_from_slice(wbytes);
        if let Some(victim) = players.get(&victim_addr) {
            if !victim.is_bot {
                let _ = socket.send_to(&death_msg, victim_addr);
            }
        }

        let mut victim_reset_required = false;
        let mut pending_match_end: Option<(u32, String)> = None;
        if let Some(killer) = players.get_mut(&attacker) {
            killer.kills += 1;

            if match_mode == MODE_GUN_GAME {
                let is_knife = weapon_name.eq_ignore_ascii_case("knife");
                let final_knife_level = (game::GUN_GAME_PROGRESSION.len() + 1) as u32;

                if is_knife {
                    if killer.gun_level == final_knife_level {
                        pending_match_end = Some((killer.id, killer.username.clone()));
                    } else {
                        victim_reset_required = true;
                    }
                } else if (killer.gun_level as usize) < game::GUN_GAME_PROGRESSION.len() + 1 {
                    killer.gun_level = killer.gun_level.saturating_add(1);
                }
            }

            let mut kill_msg = vec![MSG_KILL];
            kill_msg.extend_from_slice(&victim_id.to_be_bytes());
            kill_msg.push(wbytes.len() as u8);
            kill_msg.extend_from_slice(wbytes);
            if !killer.is_bot {
                let _ = socket.send_to(&kill_msg, attacker);
            }
        }

        if let Some((winner_id, winner_name)) = pending_match_end {
            let mut end_msg = vec![MSG_MATCH_END];
            end_msg.extend_from_slice(&winner_id.to_be_bytes());
            let name_bytes = winner_name.as_bytes();
            end_msg.push(name_bytes.len() as u8);
            end_msg.extend_from_slice(name_bytes);
            end_msg.push(game::MATCH_END_COUNTDOWN_SECS);

            for p in players.values() {
                if p.is_bot {
                    continue;
                }
                let _ = socket.send_to(&end_msg, p.socket_addr);
            }

            *match_end_pending = Some((
                Instant::now() + Duration::from_secs(game::MATCH_END_COUNTDOWN_SECS as u64),
                winner_id,
                winner_name,
            ));
        }

        if victim_reset_required {
            if let Some(vmut) = players.get_mut(&victim_addr) {
                vmut.gun_level = 1;
                vmut.current_weapon = chosen_weapon_for_victim.clone();
            }
        }

        broadcast::broadcast_state(socket, players);
        broadcast::broadcast_player_list(socket, players);

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
            if p.is_bot {
                continue;
            }
            let _ = socket.send_to(&feed, p.socket_addr);
        }
    } else {
        // apply non-lethal damage and check for shadow teleport
        let mut shadow_teleport = false;
        if let Some(vmut) = players.get_mut(&victim_addr) {
            vmut.health -= damage;
            if vmut.world == WorldId::Lobby && !vmut.is_bot {
                vmut.lobby_regen_resume_at = Some(Instant::now() + Duration::from_millis(1500));
            }
            // shadow teleport check: must read is_bot before dropping borrow
            if vmut.is_bot {
                if let Some(bot_state) = bot_states.get_mut(&victim_addr) {
                    if bot_state.personality.archetype == BotArchetype::KnifeSneaker {
                        bot_state.hits_received_since_teleport += 1;
                        if bot_state.hits_received_since_teleport >= 1 {
                            shadow_teleport = true;
                            bot_state.hits_received_since_teleport = 0;
                        }
                    }
                }
            }
        }
        // teleport shadow far from all players (borrow on players is dropped now)
        if shadow_teleport {
            if let Some(m) = maze {
                let (x, _y, z, _) = game::random_spawn_far(m, players, MAZE_CELL_SIZE * 3.0);
                if let Some(vmut) = players.get_mut(&victim_addr) {
                    vmut.position = (x, 0.0, z);
                }
            }
        }

        let mut dmg_msg = vec![MSG_DAMAGE];
        dmg_msg.extend_from_slice(&(damage as f32).to_be_bytes());
        if let Some(att) = players.get(&attacker) {
            dmg_msg.extend_from_slice(&att.id.to_be_bytes());
        } else {
            dmg_msg.extend_from_slice(&0u32.to_be_bytes());
        }

        if let Some(victim) = players.get(&victim_addr) {
            if !victim.is_bot {
                let _ = socket.send_to(&dmg_msg, victim_addr);
            }
        }
    }
}
