use std::collections::HashMap;
use crate::net::{Outbox, PeerId};
use std::time::Duration;
use platform::Instant;

use crate::maze::MazeData;
use crate::player::{Player, Team, WorldId};
use crate::protocols::*;
use crate::server::{broadcast, game};

const BACKSTAB_CONE_DOT: f32 = 0.5;

/// Called when a client reports that they broke a wall at (x,y).
/// Update the maze and broadcast the event to all connected players.
pub fn handle_wall_broken(
    socket: &Outbox,
    maze: &mut MazeData,
    players: &HashMap<PeerId, Player>,
    _src_addr: PeerId,
    x: u8,
    y: u8,
) {
    if maze.break_wall(x, y) {
        let msg = ServerMsg::WallBroken { x, y }.encode();
        for (addr, p) in players.iter() {
            if p.is_bot {
                continue;
            }
            let _ = socket.send_to(&msg, *addr);
        }
    }
}

/// Respond to a LAN discovery packet by sending back a small summary of the server.
pub fn handle_discover(
    socket: &Outbox,
    players: &HashMap<PeerId, Player>,
    name: &str,
    src_addr: PeerId,
) {
    let resp = ServerMsg::Discover {
        players: players.len().min(255) as u8,
        max_players: game::MATCH_MAX_PLAYERS as u8,
        name: name.to_string(),
    }
    .encode();
    let _ = socket.send_to(&resp, src_addr);
}

/// Process a hit report from a client. This function parses payload and forwards
/// to shared damage pipeline so bots can reuse the same logic.
pub fn handle_hit(
    socket: &Outbox,
    players: &mut HashMap<PeerId, Player>,
    src_addr: PeerId,
    victim_id: u32,
    damage_f: f32,
    weapon_name: Option<String>,
    is_backstab_claim: Option<bool>,
    _maze: &mut Option<MazeData>,
    _match_running: &mut bool,
    _match_start_time: &mut Option<Instant>,
    match_end_pending: &mut Option<(Instant, u32, String)>,
    match_mode: u8,
    _snd_state: &mut Option<crate::server::snd::SndState>,
    bot_states: &mut HashMap<PeerId, crate::bot::ai::BotState>,
    team_score: &mut game::TeamScore,
) {
    // the sender of the hit packet is the attacker
    let _attacker = src_addr;
    let damage = damage_f.round().max(0.0) as u32;
    let weapon_name = weapon_name.unwrap_or_default();
    let is_backstab_claim = is_backstab_claim.unwrap_or(false);

    let victim_addr_opt = players
        .iter()
        .find(|(_, p)| p.id == victim_id)
        .map(|(addr, _)| *addr);

    let Some(victim_addr) = victim_addr_opt else { return; };

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
        team_score,
    );
}

/// Respond to a respawn request by moving the player to a spawn point and
/// returning their new coordinates.
pub fn handle_respawn_request(
    socket: &Outbox,
    maze: Option<&MazeData>,
    players: &mut HashMap<PeerId, Player>,
    src_addr: PeerId,
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
            let maze_data = match maze {
                Some(m) => m,
                None => return,
            };
            match pref.team {
                Team::Red => {
                    let (red, _blue) = maze_data.team_spawn_positions(game::MAZE_CELL_SIZE);
                    (red.0, red.1, red.2)
                }
                Team::Blue => {
                    let (_red, blue) = maze_data.team_spawn_positions(game::MAZE_CELL_SIZE);
                    (blue.0, blue.1, blue.2)
                }
                _ => {
                    let (sx, sy, sz, _fallback) =
                        game::random_spawn_far(maze_data, players, game::MAZE_CELL_SIZE * 3.0);
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

        let resp = ServerMsg::Respawn { position: spawn, kind: None }.encode();
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
            "Punch".to_string()
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
    socket: &Outbox,
    players: &mut HashMap<PeerId, Player>,
    src_addr: PeerId,
    team_byte: u8,
) {
    // Teams are kept balanced by the server; only the host may swap sides,
    // and only when the teams stay within one of each other.
    let team = match Team::from_u8(team_byte) {
        Some(t @ (Team::Red | Team::Blue)) => t,
        _ => return,
    };
    if game::host_addr(players) != Some(src_addr) {
        return;
    }
    let Some(current) = players.get(&src_addr).filter(|p| p.world == WorldId::Lobby).map(|p| p.team) else {
        return;
    };
    if current == team {
        return;
    }
    let (mut red, mut blue) = game::lobby_team_counts(players);
    match current {
        Team::Red => red -= 1,
        Team::Blue => blue -= 1,
        Team::Spectator => {}
    }
    if team == Team::Red { red += 1 } else { blue += 1 }
    if red.abs_diff(blue) > 1 {
        // refused: resend the list so the requester's roster is right again
        broadcast::broadcast_player_list(socket, players);
        return;
    }
    if let Some(player) = players.get_mut(&src_addr) {
        player.team = team;
    }
    broadcast::broadcast_player_list(socket, players);
}

pub fn apply_damage(
    socket: &Outbox,
    players: &mut HashMap<PeerId, Player>,
    attacker_addr: PeerId,
    victim_id: u32,
    damage: u32,
    weapon_name: &str,
    is_backstab: bool,
    match_end_pending: &mut Option<(Instant, u32, String)>,
    match_mode: u8,
    bot_states: &mut HashMap<PeerId, crate::bot::ai::BotState>,
    _maze: Option<&MazeData>,
    team_score: &mut game::TeamScore,
) {
    let (attacker, attacker_world, attacker_is_bot) = match players.get(&attacker_addr) {
        Some(a) => {
            if a.post_teleport_freeze_until
                .map_or(false, |t| t > Instant::now())
                && a.world == WorldId::Maze
            {
                return;
            }
            (attacker_addr, a.world, a.is_bot)
        }
        None => return,
    };

    let Some(victim_addr) = players
        .iter()
        .find(|(_, p)| p.id == victim_id)
        .map(|(addr, _)| *addr)
    else { return; };

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

    let backstab_verified = if is_backstab && (weapon_name.eq_ignore_ascii_case("knife") || weapon_name.eq_ignore_ascii_case("punch")) {
        let atk_pos = players.get(&attacker_addr).map(|a| a.position).unwrap_or((0.0, 0.0, 0.0));
        let vic_pos = players.get(&victim_addr).map(|v| v.position).unwrap_or((0.0, 0.0, 0.0));
        let vic_yaw = players.get(&victim_addr).map(|v| v.yaw).unwrap_or(0.0);
        let vic_fwd = (vic_yaw.cos(), vic_yaw.sin());
        let dx = atk_pos.0 - vic_pos.0;
        let dz = atk_pos.2 - vic_pos.2;
        let len = (dx * dx + dz * dz).sqrt().max(f32::EPSILON);
        let vic_to_atk = (dx / len, dz / len);
        let dot = vic_fwd.0 * vic_to_atk.0 + vic_fwd.1 * vic_to_atk.1;
        dot < -BACKSTAB_CONE_DOT
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

    // a match bot that gets shot by an enemy turns toward them, seen or not
    if let (Some(state), Some(shooter), Some(victim)) = (
        bot_states.get_mut(&victim_addr),
        players.get(&attacker_addr),
        players.get(&victim_addr),
    ) {
        if state.personality_id.is_some() && victim.is_hostile_to(shooter) {
            state.alert(shooter.id, shooter.position);
        }
    }

    if damage >= players.get(&victim_addr).map(|v| v.health).unwrap_or(0) {

        let _chosen_weapon_for_victim = if let Some(vref) = players.get(&victim_addr) {
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

        let death_msg = ServerMsg::Death {
            killer_id: players.get(&attacker).map_or(0, |att| att.id),
            weapon: weapon_name.to_string(),
        }
        .encode();
        if let Some(victim) = players.get(&victim_addr) {
            if !victim.is_bot {
                let _ = socket.send_to(&death_msg, victim_addr);
            }
        }

        let mut victim_reset_required = false;
        let mut pending_match_end: Option<(u32, String)> = None;
        let victim_team = players.get(&victim_addr).map(|v| v.team);
        if let Some(killer) = players.get_mut(&attacker) {
            killer.kills += 1;
            if attacker_world == WorldId::Maze {
                if let Some(victim_team) = victim_team {
                    team_score.record_kill(killer.team, victim_team);
                }
            }

            if match_mode == MODE_GUN_GAME {
                let is_punch = weapon_name.eq_ignore_ascii_case("punch");
                let final_punch_level = (game::GUN_GAME_PROGRESSION.len() + 1) as u32;

                if is_punch {
                    if killer.gun_level == final_punch_level {
                        pending_match_end = Some((killer.id, killer.username.clone()));
                    } else {
                        victim_reset_required = true;
                    }
                } else if (killer.gun_level as usize) < game::GUN_GAME_PROGRESSION.len() + 1 {
                    killer.gun_level = killer.gun_level.saturating_add(1);
                    let new_level = killer.gun_level as usize;
                    if new_level <= game::GUN_GAME_PROGRESSION.len() {
                        killer.current_weapon = game::GUN_GAME_PROGRESSION[new_level - 1].to_string();
                    } else {
                        killer.current_weapon = "Punch".to_string();
                    }
                }
            }

            let kill_msg =
                ServerMsg::Kill { victim_id, weapon: weapon_name.to_string() }.encode();
            if !killer.is_bot {
                let _ = socket.send_to(&kill_msg, attacker);
            }
        }

        if let Some((winner_id, winner_name)) = pending_match_end {
            let end_msg = ServerMsg::MatchEnd {
                winner_id: winner_id,
                winner_name: winner_name.to_string(),
                countdown: Some(game::MATCH_END_COUNTDOWN_SECS),
            }
            .encode();

            for (addr, p) in players.iter() {
                if p.is_bot {
                    continue;
                }
                let _ = socket.send_to(&end_msg, *addr);
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
                vmut.current_weapon = game::GUN_GAME_PROGRESSION[0].to_string();
            }
        }

        broadcast::broadcast_state(socket, players);
        broadcast::broadcast_player_list(socket, players);

        let feed = ServerMsg::KillFeed {
            killer_id: players.get(&attacker).map_or(0, |killer| killer.id),
            victim_id,
            weapon: weapon_name.to_string(),
        }
        .encode();

        for (addr, p) in players.iter() {
            if p.is_bot {
                continue;
            }
            let _ = socket.send_to(&feed, *addr);
        }
    } else {
        // apply non-lethal damage
        if let Some(vmut) = players.get_mut(&victim_addr) {
            vmut.health -= damage;
            if !vmut.is_bot {
                vmut.regen_resume_at = Some(Instant::now() + Duration::from_secs(7));
            }
        }

        // stun when punched
        let is_punch = weapon_name.eq_ignore_ascii_case("punch") || weapon_name.eq_ignore_ascii_case("knife"); 
        let dmg_msg = ServerMsg::Damage {
            amount: damage as f32,
            attacker_id: players.get(&attacker).map_or(0, |att| att.id),
            stun: is_punch,
        }
        .encode();

        if let Some(victim) = players.get(&victim_addr) {
            if !victim.is_bot {
                let _ = socket.send_to(&dmg_msg, victim_addr);
            }
        }
    }
}
