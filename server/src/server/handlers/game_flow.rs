use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use crate::bot::manager::BotManager;
use crate::bot::personality::BotPersonalityId;
use crate::maze::MazeData;
use crate::player::Team;
use crate::player::{Player, WorldId};
use crate::protocols::*;

use crate::server::{
    broadcast, game,
    game::{
        MATCH_END_COUNTDOWN_SECS, MATCH_MAX_PLAYERS,
        MATCH_POST_TELEPORT_FREEZE_SECS, MATCH_SCORE_TO_WIN, MATCH_START_COUNTDOWN_SECS,
        MAZE_CELL_SIZE, MAZE_HEIGHT, MAZE_WIDTH, MIN_PLAYERS_TO_START, TEAM_LOADOUT_BLUE,
        TEAM_LOADOUT_RED,
    },
};

/// Handle a `MSG_START_GAME` packet sent from a lobby player.  This routine does not
/// actually start the match, but instead updates state and may broadcast countdowns.
pub fn handle_start_game(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    maze: &mut Option<MazeData>,
    match_running: &mut bool,
    match_start_time: &mut Option<Instant>,
    match_warmup_end: &mut Option<Instant>,
    match_end_pending: &mut Option<(Instant, u32, String)>,
    match_total_maze_entries: &mut usize,
    match_mode: u8,
    // pointer to variable that will hold the duration for the upcoming match
    match_duration: &mut Option<Duration>,
    // value requested by the client (may be None = default)
    requested_duration: Option<Duration>,
    src_addr: SocketAddr,
) {
    // If a match is marked running but there are no players in the Maze,
    // treat it as stale and clear it so new starts are accepted.
    if *match_running {
        let maze_count = players
            .values()
            .filter(|p| p.world == WorldId::Maze)
            .count();
        if maze_count == 0 {
            println!("Stale match detected (no players) — clearing match state");
            *match_running = false;
            *match_start_time = None;
            *match_duration = game::MATCH_DURATION;
            *match_end_pending = None;
        }
    }

    // only accept start requests when not already running or scheduled
    if !*match_running && match_warmup_end.is_none() && match_end_pending.is_none() {
        // Only the lobby leader (first-joined human lobby player) may start a new match.
        let leader_addr = game::lobby_leader_addr(players);
        if leader_addr != Some(src_addr) {
            let _ = socket.send_to(&[MSG_START_DENIED, 1], src_addr);
            println!("Start-game request from {} rejected: not the lobby leader.", src_addr);
            return;
        }

        // honour requested duration when scheduling new match
        *match_duration = requested_duration;
        // require minimal number of players in lobby
        let lobby_count = players
            .values()
            .filter(|p| p.world == WorldId::Lobby)
            .count();
        if lobby_count >= MIN_PLAYERS_TO_START {
            // schedule a warmup/countdown before the match starts so all clients
            // can show a synchronized countdown UI.
            *match_warmup_end =
                Some(Instant::now() + Duration::from_secs(MATCH_START_COUNTDOWN_SECS as u64));

            // broadcast countdown to all connected clients (include selected gamemode)
            let cd: [u8; 3] = [MSG_MATCH_COUNTDOWN, MATCH_START_COUNTDOWN_SECS, match_mode];
            for p in players.values() {
                let _ = socket.send_to(&cd, p.socket_addr);
            }

            let dur_desc = if requested_duration == game::MATCH_DURATION {
                "default".to_string()
            } else if requested_duration.is_none() {
                "infinite".to_string()
            } else {
                format!("{}s", requested_duration.unwrap().as_secs())
            };
            println!(
                "Match start requested by {} — mode {} — duration {} — countdown {}s",
                src_addr, match_mode, dur_desc, MATCH_START_COUNTDOWN_SECS
            );
        } else {
            println!(
                "Start-game request ignored: not enough players ({}).",
                lobby_count
            );
        }
    } else {
        // match is already running and contains players — allow the requester to
        // join if there's a vacant slot, otherwise notify them that the match is full.
        let maze_count = players
            .values()
            .filter(|p| p.world == WorldId::Maze)
            .count();
        if maze_count < MATCH_MAX_PLAYERS {
            // compute team counts before taking a mutable borrow of the
            // joining requester so we don't hold mutable and immutable
            // borrows at the same time
            let red_count = players
                .values()
                .filter(|p| p.world == WorldId::Maze && p.team == Team::Red)
                .count();
            let blue_count = players
                .values()
                .filter(|p| p.world == WorldId::Maze && p.team == Team::Blue)
                .count();

            if let Some(req_player) = players.get_mut(&src_addr) {
                if req_player.world == WorldId::Lobby {
                    let current_maze = match &*maze {
                        Some(m) => m,
                        None => {
                            // shouldn't happen: match_running implies a maze exists
                            let _ = socket.send_to(&[MSG_START_DENIED], src_addr);
                            return;
                        }
                    };
                    // assign team by balancing current maze teams (skip for Gun Game)
                    if match_mode != MODE_GUN_GAME {
                        req_player.team = if red_count <= blue_count {
                            Team::Red
                        } else {
                            Team::Blue
                        };
                    } else {
                        req_player.team = Team::Spectator;
                    }

                    let (sx, sy, sz, _used_fallback) =
                        current_maze.random_spawn_world(MAZE_CELL_SIZE);
                    req_player.world = WorldId::Maze;
                    req_player.position = (sx, sy, sz);
                    // set per-player freeze for newly-teleported joiners
                    req_player.post_teleport_freeze_until =
                        Some(Instant::now() + Duration::from_secs(MATCH_POST_TELEPORT_FREEZE_SECS));

                    // include authoritative timer so clients don't get stuck on the
                    // default 120s value if packets reorder.  if we don't have a
                    // start time (shouldn't happen while match_running is true) just
                    // send the nominal duration.
                    let mut response = vec![MSG_WORLD_CHANGE, WorldId::Maze.as_u8()];
                    let (sx, sy, sz) = req_player.position;
                    response.extend_from_slice(&sx.to_be_bytes());
                    response.extend_from_slice(&sy.to_be_bytes());
                    response.extend_from_slice(&sz.to_be_bytes());
                    response.push(match_mode);
                    response.push(MATCH_POST_TELEPORT_FREEZE_SECS as u8);
                    // determine remaining seconds clamped to 255
                    // compute remaining timer, using u16::MAX as a stand-in for “infinite”
                    let timer_secs: u16 = if let Some(start) = *match_start_time {
                        let elapsed = Instant::now().duration_since(start);
                        match *match_duration {
                            Some(dur) => {
                                if elapsed >= dur {
                                    0u16
                                } else {
                                    (dur - elapsed).as_secs().min(u16::MAX as u64) as u16
                                }
                            }
                            None => u16::MAX,
                        }
                    } else {
                        match *match_duration {
                            Some(dur) => dur.as_secs().min(u16::MAX as u64) as u16,
                            None => u16::MAX,
                        }
                    };
                    response.push((timer_secs >> 8) as u8);
                    response.push((timer_secs & 0xFF) as u8);
                    let _ = socket.send_to(&response, req_player.socket_addr);

                    // send maze data to the joining client
                    current_maze.send(&socket, req_player.socket_addr);

                    *match_total_maze_entries += 1;
                    println!(
                        "Player '{}' joined running match (vacant slot)",
                        req_player.username
                    );

                    // Broadcast announcement to all connected players
                    let name_bytes = req_player.username.as_bytes();
                    let mut ann = vec![MSG_MATCH_JOINED];
                    ann.extend_from_slice(&req_player.id.to_be_bytes());
                    ann.push(name_bytes.len() as u8);
                    ann.extend_from_slice(name_bytes);
                    for p in players.values() {
                        let _ = socket.send_to(&ann, p.socket_addr);
                    }

                    // After assigning team and moving the player, broadcast
                    // authoritative state and player list so clients update.
                    broadcast::broadcast_state(socket, players);
                    broadcast::broadcast_player_list(socket, players);
                }
            }
        } else {
            // Inform the requester the match is running and full
            let _ = socket.send_to(&[MSG_START_DENIED], src_addr);
            println!("Start-game request rejected: match already running and full.");
        }
    }
}

/// The warmup countdown has expired; actually start the match by generating a new
/// maze and moving lobby players into it.
pub fn maybe_start_match(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    maze: &mut Option<MazeData>,
    match_running: &mut bool,
    match_start_time: &mut Option<Instant>,
    match_warmup_end: &mut Option<Instant>,
    last_match_time_sent: &mut Option<u16>,
    match_total_maze_entries: &mut usize,
    match_mode: u8,
    snd_state: &mut Option<crate::server::snd::SndState>,
    match_duration: Option<Duration>,
    bot_manager: &mut BotManager,
    autofill_bots_per_team: u8,
    autofill_diff_mask: u8,
) {
    if let Some(end) = *match_warmup_end {
        if Instant::now() >= end {
            // start the match now (generate maze, move lobby players into Maze)
            *maze = Some(MazeData::new(MAZE_WIDTH, MAZE_HEIGHT));
            let current_maze = maze.as_ref().unwrap();
            let lobby_count = players
                .values()
                .filter(|p| p.world == WorldId::Lobby)
                .count();
            let dur_desc = match match_duration {
                Some(d) => format!("{}s", d.as_secs()),
                None => "infinite".to_string(),
            };
            println!(
                "Match warmup complete — starting {} match with seed {} (moving {} players)",
                dur_desc, current_maze.seed, lobby_count
            );

            *match_running = true;
            *match_start_time = Some(Instant::now());
            *match_warmup_end = None;
            *match_total_maze_entries = lobby_count;
            // send initial authoritative match timer to all clients
            let initial_secs: u16 = match match_duration {
                Some(dur) => dur.as_secs().min(u16::MAX as u64) as u16,
                None => u16::MAX,
            };
            broadcast::broadcast_match_timer(socket, players, initial_secs);
            *last_match_time_sent = Some(initial_secs);
            // set per-player post-teleport freeze for each player moved into the Maze

            // Collect the list of players who will be moved into the maze so
            // we can assign teams and unique loadouts deterministically.
            // Use a deterministic order (player id ascending) so clients and
            // server will agree on previewed team assignments.
            let mut joiner_pairs: Vec<(std::net::SocketAddr, u32)> = players
                .iter()
                .filter_map(|(addr, p)| {
                    if p.world == WorldId::Lobby {
                        Some((*addr, p.id))
                    } else {
                        None
                    }
                })
                .collect();
            // sort by player id to get a stable ordering
            joiner_pairs.sort_by_key(|(_, id)| *id);
            let joiners: Vec<std::net::SocketAddr> =
                joiner_pairs.iter().map(|(addr, _)| *addr).collect();

            // track assigned primaries per team so teammates don't share
            let mut red_assigned: Vec<String> = Vec::new();
            let mut blue_assigned: Vec<String> = Vec::new();

            // For team-based modes compute two deterministic spawn positions
            // (red, blue) that are farthest apart in the generated maze.
            let team_spawns = if match_mode == crate::protocols::MODE_TEAM_DEATHMATCH
                || match_mode == crate::protocols::MODE_SEARCH_AND_DESTROY
            {
                Some(current_maze.team_spawn_positions(MAZE_CELL_SIZE))
            } else {
                None
            };

            // For S&D, compute bomb site and initialize round state
            if match_mode == crate::protocols::MODE_SEARCH_AND_DESTROY {
                let bomb_site = current_maze.bomb_site_position(MAZE_CELL_SIZE);
                let (red_spawn, blue_spawn) =
                    team_spawns.unwrap_or(((0.0, 0.0, 0.0), (0.0, 0.0, 0.0)));
                *snd_state = Some(crate::server::snd::SndState::new(
                    bomb_site, red_spawn, blue_spawn,
                ));
            }

            for (i, addr) in joiners.iter().enumerate() {
                if let Some(player) = players.get_mut(addr) {
                    player.world = WorldId::Maze;
                    // choose spawn based on match mode / team
                    if let Some((red_spawn, blue_spawn)) = team_spawns {
                        let team = if i % 2 == 0 {
                            crate::player::Team::Red
                        } else {
                            crate::player::Team::Blue
                        };
                        let spawn = match team {
                            crate::player::Team::Red => red_spawn,
                            crate::player::Team::Blue => blue_spawn,
                            _ => red_spawn,
                        };
                        player.position = spawn;
                    } else {
                        let (sx, sy, sz, _used_fallback) =
                            current_maze.random_spawn_world(MAZE_CELL_SIZE);
                        player.position = (sx, sy, sz);
                    }
                    player.post_teleport_freeze_until =
                        Some(Instant::now() + Duration::from_secs(MATCH_POST_TELEPORT_FREEZE_SECS));

                    // reset per-match player state
                    player.health = player.max_health;
                    player.is_alive = true;
                    player.kills = 0;
                    player.deaths = 0;
                    player.death_time = None;

                    if match_mode == MODE_GUN_GAME {
                        // Gun Game is free-for-all: no teams, reset gun progression
                        player.gun_level = 1;
                        player.team = Team::Spectator;
                        // assign the first weapon from Gun Game progression
                        player.current_weapon =
                            crate::server::game::GUN_GAME_PROGRESSION[0].to_string();
                    } else {
                        // Team modes (TDM, S&D): assign balanced teams
                        let team = if i % 2 == 0 { Team::Red } else { Team::Blue };
                        player.team = team;

                        // pick a primary for this player avoiding duplicates
                        let pool = match team {
                            Team::Red => &TEAM_LOADOUT_RED,
                            Team::Blue => &TEAM_LOADOUT_BLUE,
                            Team::Spectator => &TEAM_LOADOUT_RED,
                        };
                        let assigned = if team == Team::Red {
                            &mut red_assigned
                        } else {
                            &mut blue_assigned
                        };

                        if player.current_weapon.is_empty() {
                            let mut chosen = None;
                            for &w in pool.iter() {
                                if !assigned.iter().any(|a| a == w) {
                                    chosen = Some(w.to_string());
                                    break;
                                }
                            }
                            if chosen.is_none() {
                                chosen = pool.get(0).map(|s| s.to_string());
                            }
                            let weapon = chosen.unwrap_or_else(|| "Knife".to_string());
                            assigned.push(weapon.clone());
                            player.current_weapon = weapon.clone();
                        }
                    }

                    let mut response = vec![MSG_WORLD_CHANGE, WorldId::Maze.as_u8()];
                    // extract the authoritative spawn coordinates from the player
                    let (sx, sy, sz) = player.position;
                    response.extend_from_slice(&sx.to_be_bytes());
                    response.extend_from_slice(&sy.to_be_bytes());
                    response.extend_from_slice(&sz.to_be_bytes());
                    // attach match mode so clients know which mode is starting
                    response.push(match_mode);
                    // attach freeze duration so all clients use the same value
                    response.push(MATCH_POST_TELEPORT_FREEZE_SECS as u8);
                    // attach initial timer (same value already broadcast earlier)
                    let initial_secs: u16 = match match_duration {
                        Some(dur) => dur.as_secs().min(u16::MAX as u64) as u16,
                        None => u16::MAX,
                    };
                    response.push((initial_secs >> 8) as u8);
                    response.push((initial_secs & 0xFF) as u8);
                    let _ = socket.send_to(&response, player.socket_addr);

                    current_maze.send(&socket, player.socket_addr);
                }
            }

            // Despawn any shooting range bots before spawning game bots
            bot_manager.despawn_range_bots(players);

            // ── Auto-fill with bots ──
            // Spawn `autofill_bots_per_team` bots per team (equal teams).
            // Randomly pick from personalities whose difficulty matches the mask.
            if match_mode != MODE_GUN_GAME && autofill_bots_per_team > 0 {
                use rand::seq::SliceRandom;

                let all_personalities = [
                    BotPersonalityId::Heater,
                    BotPersonalityId::Stinger,
                    BotPersonalityId::Rex,
                    BotPersonalityId::Merlin,
                    BotPersonalityId::Viper,
                    BotPersonalityId::Phantom,
                    BotPersonalityId::Nexto,
                    BotPersonalityId::Bandit,
                    BotPersonalityId::Slater,
                    BotPersonalityId::Storm,
                    BotPersonalityId::Ghost,
                    BotPersonalityId::Sniper,
                    BotPersonalityId::Bulwark,
                    BotPersonalityId::Wraith,
                    BotPersonalityId::Overwatch,
                ];

                // filter by difficulty mask: bit0=Recruit, bit1=Easy, bit2=Normal, bit3=Hard, bit4=Nightmare
                let eligible: Vec<BotPersonalityId> = all_personalities
                    .iter()
                    .copied()
                    .filter(|pid| {
                        let (_, diff) = crate::bot::personality::get_personality(*pid);
                        let bit = match diff {
                            crate::bot::personality::BotDifficulty::Recruit   => 0,
                            crate::bot::personality::BotDifficulty::Easy      => 1,
                            crate::bot::personality::BotDifficulty::Normal    => 2,
                            crate::bot::personality::BotDifficulty::Hard      => 3,
                            crate::bot::personality::BotDifficulty::Nightmare => 4,
                        };
                        (autofill_diff_mask >> bit) & 1 == 1
                    })
                    .collect();

                if !eligible.is_empty() {
                    let mut rng = rand::thread_rng();
                    let total_bots = (autofill_bots_per_team as usize) * 2;

                    // ensure we don't exceed max players
                    let current_count = players.values().filter(|p| p.world == WorldId::Maze).count();
                    let slots_available = MATCH_MAX_PLAYERS.saturating_sub(current_count);
                    // must be even to keep teams equal
                    let bots_to_spawn = total_bots.min(slots_available) & !1;

                    // count current team sizes for balancing
                    let mut red_count = players.values().filter(|p| p.world == WorldId::Maze && p.team == Team::Red).count();
                    let mut blue_count = players.values().filter(|p| p.world == WorldId::Maze && p.team == Team::Blue).count();

                    // track assigned weapons per team to avoid duplicates
                    let mut red_weapons: Vec<String> = players.values()
                        .filter(|p| p.world == WorldId::Maze && p.team == Team::Red)
                        .map(|p| p.current_weapon.clone())
                        .collect();
                    let mut blue_weapons: Vec<String> = players.values()
                        .filter(|p| p.world == WorldId::Maze && p.team == Team::Blue)
                        .map(|p| p.current_weapon.clone())
                        .collect();

                    // track used personalities so we don't spawn duplicates
                    let mut used_personalities: Vec<BotPersonalityId> = Vec::new();

                    for _ in 0..bots_to_spawn {
                        // pick a random unused personality; stop if all used (no duplicates)
                        let remaining: Vec<&BotPersonalityId> = eligible
                            .iter()
                            .filter(|p| !used_personalities.contains(p))
                            .collect();
                        let pid = if remaining.is_empty() {
                            break;
                        } else {
                            **remaining.choose(&mut rng).unwrap()
                        };
                        used_personalities.push(pid);

                        let (pers, _) = crate::bot::personality::get_personality(pid);

                        // balance teams
                        let team = if red_count <= blue_count { Team::Red } else { Team::Blue };

                        // pick weapon
                        let pool = match team {
                            Team::Red => &TEAM_LOADOUT_RED,
                            Team::Blue => &TEAM_LOADOUT_BLUE,
                            _ => &TEAM_LOADOUT_RED,
                        };
                        let assigned = if team == Team::Red { &mut red_weapons } else { &mut blue_weapons };

                        let weapon = if !assigned.iter().any(|w| w == pers.weapon) {
                            pers.weapon.to_string()
                        } else {
                            let mut chosen = None;
                            for &w in pool.iter() {
                                if !assigned.iter().any(|a| a == w) {
                                    chosen = Some(w.to_string());
                                    break;
                                }
                            }
                            chosen.unwrap_or_else(|| pers.weapon.to_string())
                        };
                        assigned.push(weapon.clone());

                        let spawn_pos = if let Some((red_spawn, blue_spawn)) = team_spawns {
                            match team {
                                Team::Red => red_spawn,
                                Team::Blue => blue_spawn,
                                _ => red_spawn,
                            }
                        } else {
                            let (sx, sy, sz, _) = current_maze.random_spawn_world(MAZE_CELL_SIZE);
                            (sx, sy, sz)
                        };

                        bot_manager.spawn_personality_bot(players, pid, team, &weapon, spawn_pos);

                        match team {
                            Team::Red => red_count += 1,
                            Team::Blue => blue_count += 1,
                            _ => {}
                        }
                    }

                    if bots_to_spawn > 0 {
                        println!("Auto-filled {} bots into match ({} per team)", bots_to_spawn, bots_to_spawn / 2);
                    }
                } else {
                    println!("Auto-fill: no eligible bot personalities (diff mask {:05b})", autofill_diff_mask);
                }
            }

            // broadcast updated player list/state so clients update immediately
            broadcast::broadcast_state(socket, players);
            broadcast::broadcast_player_list(socket, players);

            // For S&D, start the first round now that all players are in the maze
            if match_mode == crate::protocols::MODE_SEARCH_AND_DESTROY {
                if let Some(ref mut snd) = snd_state {
                    snd.start_round(socket, players);
                }
            }
        }
    }
}

/// Check for early match end conditions such as last‑man standing.
pub fn check_early_end_conditions(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    maze: &mut Option<MazeData>,
    match_running: &mut bool,
    match_end_pending: &mut Option<(Instant, u32, String)>,
    match_total_maze_entries: &mut usize,
    match_start_time: &mut Option<Instant>,
    match_mode: u8,
    match_duration: &mut Option<Duration>,
) {
    // gun game should never end just because someone died;
    // kills are handled separately in hit logic
    if match_mode == MODE_GUN_GAME {
        return;
    }
    // count maze players and alive players in a single pass
    let mut maze_count = 0;
    let mut alive_count = 0;
    let mut last_alive: Option<&Player> = None;
    for p in players.values() {
        if p.world == WorldId::Maze {
            maze_count += 1;
            if p.is_alive {
                alive_count += 1;
                last_alive = Some(p);
            }
        }
    }

    if maze_count == 0 {
        println!("No players remain in match — ending match early");
        *match_running = false;
        *match_start_time = None;
        *maze = None;
        *match_duration = game::MATCH_DURATION;
    } else if match_mode != MODE_TEAM_DEATHMATCH
        && alive_count == 1
        && *match_total_maze_entries >= 2
    {
        // Last player standing wins — only trigger when at least 2 players
        // entered the maze during this match (not just a solo player). For
        // team-based modes (TDM, S&D) we don't end the match on a single
        // player's death — team scoring determines match end.
        let winner = last_alive.unwrap();
        let winner_id = winner.id;
        let winner_name = winner.username.clone();
        println!("Last player standing: '{}' wins the match", winner_name);

        // Broadcast MSG_MATCH_END (include countdown byte so clients can display timer)
        let mut end_msg = vec![MSG_MATCH_END];
        end_msg.extend_from_slice(&winner_id.to_be_bytes());
        let name_bytes = winner_name.as_bytes();
        end_msg.push(name_bytes.len() as u8);
        end_msg.extend_from_slice(name_bytes);
        end_msg.push(MATCH_END_COUNTDOWN_SECS);
        for p in players.values() {
            let _ = socket.send_to(&end_msg, p.socket_addr);
        }

        *match_end_pending = Some((
            Instant::now() + Duration::from_secs(MATCH_END_COUNTDOWN_SECS as u64),
            winner_id,
            winner_name,
        ));
    }

    // Team Deathmatch early win: if one team reaches the score threshold,
    // end the match with that team as the winner. Only applies to TDM.
    if match_mode == MODE_TEAM_DEATHMATCH {
        let mut red_kills: u32 = 0;
        let mut blue_kills: u32 = 0;
        for p in players.values() {
            if p.world == WorldId::Maze {
                match p.team {
                    Team::Red => red_kills = red_kills.saturating_add(p.kills),
                    Team::Blue => blue_kills = blue_kills.saturating_add(p.kills),
                    _ => {}
                }
            }
        }
        if red_kills >= MATCH_SCORE_TO_WIN || blue_kills >= MATCH_SCORE_TO_WIN {
            let winning_team = if red_kills >= MATCH_SCORE_TO_WIN { Team::Red } else { Team::Blue };
            let winner_team_name = if winning_team == Team::Red { "Red Team".to_string() } else { "Blue Team".to_string() };
            let winner_id = players
                .values()
                .filter(|p| p.world == WorldId::Maze && p.team == winning_team)
                .max_by_key(|p| p.kills)
                .map(|p| p.id)
                .unwrap_or(0);

            println!(
                "Team Deathmatch win: {} reached {} kills",
                winner_team_name, MATCH_SCORE_TO_WIN
            );
            // Broadcast MSG_MATCH_END with a placeholder id (0) and a team name.
            let mut end_msg = vec![MSG_MATCH_END];
            end_msg.extend_from_slice(&winner_id.to_be_bytes());
            let name_bytes = winner_team_name.as_bytes();
            end_msg.push(name_bytes.len() as u8);
            end_msg.extend_from_slice(name_bytes);
            end_msg.push(MATCH_END_COUNTDOWN_SECS);
            for p in players.values() {
                let _ = socket.send_to(&end_msg, p.socket_addr);
            }

            *match_end_pending = Some((
                Instant::now() + Duration::from_secs(MATCH_END_COUNTDOWN_SECS as u64),
                winner_id,
                winner_team_name,
            ));
        }
    }
}

/// Update the authoritative match timer and check for timeout-based end.
pub fn update_match_timer(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    _maze: &mut Option<MazeData>,
    match_running: &mut bool,
    match_start_time: &mut Option<Instant>,
    last_match_time_sent: &mut Option<u16>,
    _match_total_maze_entries: &mut usize,
    match_end_pending: &mut Option<(Instant, u32, String)>,
    match_mode: u8,
    match_duration: Option<Duration>,
) {
    if let Some(start) = match_start_time {
        let elapsed = Instant::now().duration_since(*start);

        // send authoritative remaining-time to clients (only when value changes)
        let remaining_secs: u16 = match match_duration {
            Some(dur) => {
                if elapsed >= dur {
                    0u16
                } else {
                    (dur - elapsed).as_secs().min(u16::MAX as u64) as u16
                }
            }
            None => u16::MAX,
        };
        // if the match is infinite we still need to send periodic
        // heartbeat packets so clients don't time out; finite matches are
        // already guaranteed to send every second as the value changes.
        if match_duration.is_none() || *last_match_time_sent != Some(remaining_secs) {
            let _ = broadcast::broadcast_match_timer(socket, players, remaining_secs);
            *last_match_time_sent = Some(remaining_secs);
        }

        // timeout -> show match-end screen then return to lobby after countdown
        if match_end_pending.is_none() && match_duration.map_or(false, |dur| elapsed >= dur) {
            println!("Match duration elapsed — showing end screen");
            *match_running = false;

            // For TDM: determine winner by team total kills
            let (winner_id, winner_name) = if match_mode == MODE_TEAM_DEATHMATCH {
                let mut red_kills: u32 = 0;
                let mut blue_kills: u32 = 0;
                for p in players.values() {
                    if p.world == WorldId::Maze {
                        match p.team {
                            Team::Red => red_kills = red_kills.saturating_add(p.kills),
                            Team::Blue => blue_kills = blue_kills.saturating_add(p.kills),
                            _ => {}
                        }
                    }
                }
                if red_kills > blue_kills {
                    let id = players.values().filter(|p| p.world == WorldId::Maze && p.team == Team::Red).max_by_key(|p| p.kills).map(|p| p.id).unwrap_or(0);
                    (id, "Red Team".to_string())
                } else if blue_kills > red_kills {
                    let id = players.values().filter(|p| p.world == WorldId::Maze && p.team == Team::Blue).max_by_key(|p| p.kills).map(|p| p.id).unwrap_or(0);
                    (id, "Blue Team".to_string())
                } else {
                    (0u32, "Draw".to_string())
                }
            } else {
                // Other modes: find the player with the most kills as the winner
                let winner = players
                    .values()
                    .filter(|p| p.world == WorldId::Maze)
                    .max_by_key(|p| p.kills);
                match winner {
                    Some(w) => (w.id, w.username.clone()),
                    None => (0, "Nobody".to_string()),
                }
            };

            // broadcast MSG_MATCH_END with countdown
            let mut end_msg = vec![MSG_MATCH_END];
            end_msg.extend_from_slice(&winner_id.to_be_bytes());
            let name_bytes = winner_name.as_bytes();
            end_msg.push(name_bytes.len() as u8);
            end_msg.extend_from_slice(name_bytes);
            end_msg.push(MATCH_END_COUNTDOWN_SECS);
            for p in players.values() {
                let _ = socket.send_to(&end_msg, p.socket_addr);
            }

            *match_end_pending = Some((
                Instant::now() + Duration::from_secs(MATCH_END_COUNTDOWN_SECS as u64),
                winner_id,
                winner_name,
            ));
        }
    }
}

/// Execute the previously scheduled match end (after countdown expires).
pub fn execute_scheduled_match_end(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    maze: &mut Option<MazeData>,
    match_running: &mut bool,
    match_start_time: &mut Option<Instant>,
    match_end_pending: &mut Option<(Instant, u32, String)>,
    last_match_time_sent: &mut Option<u16>,
    match_total_maze_entries: &mut usize,
    // reset duration back to default once the match actually ends
    match_duration: &mut Option<Duration>,
    winner_name: String,
    snd_state: &mut Option<crate::server::snd::SndState>,
    bot_manager: &mut BotManager,
) {
    println!(
        "Match end countdown expired — returning players to Lobby (winner {})",
        winner_name
    );

    // Despawn all bots before moving players back
    bot_manager.despawn_all(players);

    // Move all players back to Lobby, reset match state and clear maze
    broadcast::move_all_players_to_lobby(socket, players, true);

    // clear server-side maze and match flags
    *maze = None;
    *match_running = false;
    *match_start_time = None;

    // clear pending flag
    *match_end_pending = None;
    *last_match_time_sent = None;
    *match_total_maze_entries = 0;
    // restore duration to default
    *match_duration = game::MATCH_DURATION;
    // clear S&D state
    *snd_state = None;
}
