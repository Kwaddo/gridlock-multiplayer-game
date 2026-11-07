use std::collections::HashMap;
use crate::net::{Outbox, PeerId};
use std::time::Duration;
use platform::Instant;

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
        MAZE_CELL_SIZE, MIN_PLAYERS_TO_START, TEAM_LOADOUT_BLUE,
        TEAM_LOADOUT_RED,
    },
};

/// Handle a `MSG_START_GAME` packet sent from a lobby player.  This routine does not
/// actually start the match, but instead updates state and may broadcast countdowns.
pub fn handle_start_game(
    socket: &Outbox,
    players: &mut HashMap<PeerId, Player>,
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
    src_addr: PeerId,
) {
    // A match nobody plays in any more is ended by `Server` before this runs.
    if match_warmup_end.is_some() {
        let _ = socket.send_to(&ServerMsg::StartDenied { reason: Some(START_DENIED_STARTING) }.encode(), src_addr);
        return;
    }
    if match_end_pending.is_some() {
        // the match is over; joining now would only show the end screen
        let _ = socket.send_to(&ServerMsg::StartDenied { reason: Some(START_DENIED_ENDING) }.encode(), src_addr);
        return;
    }

    if !*match_running {
        // Only the host (first-joined human player) may start a new match.
        if game::host_addr(players) != Some(src_addr) {
            let _ = socket.send_to(&ServerMsg::StartDenied { reason: Some(START_DENIED_NOT_HOST) }.encode(), src_addr);
            log!("Start-game request from {} rejected: not the host.", src_addr);
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

            let cd = ServerMsg::MatchCountdown { secs: MATCH_START_COUNTDOWN_SECS, mode: Some(match_mode) }
                .encode();
            for (addr, _) in players.iter() {
                let _ = socket.send_to(&cd, *addr);
            }

            let dur_desc = if requested_duration == game::MATCH_DURATION {
                "default".to_string()
            } else if requested_duration.is_none() {
                "infinite".to_string()
            } else {
                requested_duration.map_or_else(|| "?".to_string(), |d| format!("{}s", d.as_secs()))
            };
            log!(
                "Match start requested by {} — mode {} — duration {} — countdown {}s",
                src_addr, match_mode, dur_desc, MATCH_START_COUNTDOWN_SECS
            );
        } else {
            log!(
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
                            let _ = socket.send_to(&ServerMsg::StartDenied { reason: None }.encode(), src_addr);
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
                    let response = ServerMsg::WorldChange {
                        world: WorldId::Maze.as_u8(),
                        spawn: req_player.position,
                        mode: Some(match_mode),
                        freeze_secs: Some(MATCH_POST_TELEPORT_FREEZE_SECS as u8),
                        match_timer: Some(timer_secs),
                    }
                    .encode();
                    let _ = socket.send_to(&response, src_addr);

                    current_maze.send(&socket, src_addr);

                    *match_total_maze_entries += 1;
                    log!(
                        "Player '{}' joined running match (vacant slot)",
                        req_player.username
                    );

                    // Broadcast announcement to all connected players
                    let ann = ServerMsg::MatchJoined {
                        id: req_player.id,
                        name: req_player.username.clone(),
                    }
                    .encode();
                    for (addr, _) in players.iter() {
                        let _ = socket.send_to(&ann, *addr);
                    }

                    // After assigning team and moving the player, broadcast
                    // authoritative state and player list so clients update.
                    broadcast::broadcast_state(socket, players);
                    broadcast::broadcast_player_list(socket, players);
                }
            }
        } else {
            // Inform the requester the match is running and full
            let _ = socket.send_to(&ServerMsg::StartDenied { reason: None }.encode(), src_addr);
            log!("Start-game request rejected: match already running and full.");
        }
    }
}

/// The warmup countdown has expired; actually start the match by generating a new
/// maze and moving lobby players into it.
pub fn maybe_start_match(
    socket: &Outbox,
    players: &mut HashMap<PeerId, Player>,
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
    autofill_red_bots: u8,
    autofill_blue_bots: u8,
    autofill_diff_mask: u8,
) {
    if let Some(end) = *match_warmup_end {
        if Instant::now() >= end {
            // start the match now (generate maze, move lobby players into Maze).
            // Bots are planned first so the maze is sized for everyone who
            // will play in it.
            let humans = players
                .values()
                .filter(|p| p.world == WorldId::Lobby && !p.is_bot)
                .count();
            let bot_slots = bot_team_slots(
                match_mode,
                autofill_red_bots,
                autofill_blue_bots,
                humans,
                eligible_bot_personalities(autofill_diff_mask).len(),
            );
            let side = game::maze_side_for_players(humans + bot_slots.len());
            *maze = Some(MazeData::new(side, side));
            let Some(current_maze) = maze.as_ref() else { return; };
            let lobby_count = players
                .values()
                .filter(|p| p.world == WorldId::Lobby)
                .count();
            let dur_desc = match match_duration {
                Some(d) => format!("{}s", d.as_secs()),
                None => "infinite".to_string(),
            };
            log!(
                "Match warmup complete — starting {} match with seed {} on a {}x{} maze (moving {} players, {} bots)",
                dur_desc, current_maze.seed, side, side, lobby_count, bot_slots.len()
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
            let mut joiner_pairs: Vec<(PeerId, u32)> = players
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
            let joiners: Vec<PeerId> =
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

                    // attach initial timer (same value already broadcast earlier)
                    let initial_secs: u16 = match match_duration {
                        Some(dur) => dur.as_secs().min(u16::MAX as u64) as u16,
                        None => u16::MAX,
                    };
                    let response = ServerMsg::WorldChange {
                        world: WorldId::Maze.as_u8(),
                        // the authoritative spawn coordinates from the player
                        spawn: player.position,
                        // match mode so clients know which mode is starting
                        mode: Some(match_mode),
                        // freeze duration so all clients use the same value
                        freeze_secs: Some(MATCH_POST_TELEPORT_FREEZE_SECS as u8),
                        match_timer: Some(initial_secs),
                    }
                    .encode();
                    let _ = socket.send_to(&response, *addr);

                    current_maze.send(&socket, *addr);
                }
            }

            // Despawn any shooting range bots before spawning game bots
            bot_manager.despawn_range_bots(players);

            // ── Auto-fill with bots ──
            // `bot_slots` (worked out before the maze was sized) holds the team
            // of each bot to add: the host's Red/Blue counts in team modes, or
            // a plain count of teamless bots in Gun Game, where every bot
            // starts on the first progression weapon. Personalities are picked
            // at random from those whose difficulty matches the mask.
            if !bot_slots.is_empty() {
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                let mut pool = eligible_bot_personalities(autofill_diff_mask);
                pool.shuffle(&mut rng);

                // track assigned weapons per team to avoid duplicates
                let mut red_weapons: Vec<String> = players.values()
                    .filter(|p| p.world == WorldId::Maze && p.team == Team::Red)
                    .map(|p| p.current_weapon.clone())
                    .collect();
                let mut blue_weapons: Vec<String> = players.values()
                    .filter(|p| p.world == WorldId::Maze && p.team == Team::Blue)
                    .map(|p| p.current_weapon.clone())
                    .collect();

                for (&team, pid) in bot_slots.iter().zip(pool) {
                    let (pers, _) = crate::bot::personality::get_personality(pid);

                    if team == Team::Spectator {
                        let (sx, sy, sz, _) = game::random_spawn_far(
                            current_maze,
                            players,
                            MAZE_CELL_SIZE * 3.0,
                        );
                        bot_manager.spawn_personality_bot(
                            players,
                            pid,
                            Team::Spectator,
                            game::GUN_GAME_PROGRESSION[0],
                            (sx, sy, sz),
                        );
                        continue;
                    }

                    // pick weapon: the personality's own unless a teammate has it
                    let loadout = if team == Team::Red { &TEAM_LOADOUT_RED } else { &TEAM_LOADOUT_BLUE };
                    let assigned = if team == Team::Red { &mut red_weapons } else { &mut blue_weapons };
                    let weapon = if !assigned.iter().any(|w| w == pers.weapon) {
                        pers.weapon.to_string()
                    } else {
                        loadout
                            .iter()
                            .find(|w| !assigned.iter().any(|a| a == *w))
                            .map(|w| w.to_string())
                            .unwrap_or_else(|| pers.weapon.to_string())
                    };
                    assigned.push(weapon.clone());

                    let spawn_pos = match team_spawns {
                        Some((red_spawn, _)) if team == Team::Red => red_spawn,
                        Some((_, blue_spawn)) => blue_spawn,
                        None => {
                            let (sx, sy, sz, _) = current_maze.random_spawn_world(MAZE_CELL_SIZE);
                            (sx, sy, sz)
                        }
                    };

                    bot_manager.spawn_personality_bot(players, pid, team, &weapon, spawn_pos);
                }

                let red = bot_slots.iter().filter(|t| **t == Team::Red).count();
                let blue = bot_slots.iter().filter(|t| **t == Team::Blue).count();
                if red + blue == 0 {
                    log!("Auto-filled {} bots into free-for-all match", bot_slots.len());
                } else {
                    log!("Auto-filled {} bots into match ({} red, {} blue)", red + blue, red, blue);
                }
            } else if autofill_red_bots + autofill_blue_bots > 0 {
                log!("Auto-fill: no eligible bot personalities (diff mask {:05b})", autofill_diff_mask);
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
    socket: &Outbox,
    players: &mut HashMap<PeerId, Player>,
    maze: &mut Option<MazeData>,
    match_running: &mut bool,
    match_end_pending: &mut Option<(Instant, u32, String)>,
    match_total_maze_entries: &mut usize,
    match_start_time: &mut Option<Instant>,
    match_mode: u8,
    match_duration: &mut Option<Duration>,
    team_score: game::TeamScore,
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
        log!("No players remain in match — ending match early");
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
        let Some(winner) = last_alive else { return; };
        let winner_id = winner.id;
        let winner_name = winner.username.clone();
        log!("Last player standing: '{}' wins the match", winner_name);

        // Broadcast MSG_MATCH_END (include countdown byte so clients can display timer)
        let end_msg = ServerMsg::MatchEnd {
            winner_id: winner_id,
            winner_name: winner_name.to_string(),
            countdown: Some(MATCH_END_COUNTDOWN_SECS),
        }
        .encode();
        for (addr, _) in players.iter() {
            let _ = socket.send_to(&end_msg, *addr);
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
        if let Some(winning_team) = team_score.winner() {
            let winner_team_name = if winning_team == Team::Red { "Red Team".to_string() } else { "Blue Team".to_string() };
            let winner_id = players
                .values()
                .filter(|p| p.world == WorldId::Maze && p.team == winning_team)
                .max_by_key(|p| p.kills)
                .map(|p| p.id)
                .unwrap_or(0);

            log!(
                "Team Deathmatch win: {} reached {} kills",
                winner_team_name, MATCH_SCORE_TO_WIN
            );
            // Broadcast MSG_MATCH_END with a placeholder id (0) and a team name.
            let end_msg = ServerMsg::MatchEnd {
                winner_id: winner_id,
                winner_name: winner_team_name.to_string(),
                countdown: Some(MATCH_END_COUNTDOWN_SECS),
            }
            .encode();
            for (addr, _) in players.iter() {
                let _ = socket.send_to(&end_msg, *addr);
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
    socket: &Outbox,
    players: &mut HashMap<PeerId, Player>,
    _maze: &mut Option<MazeData>,
    match_running: &mut bool,
    match_start_time: &mut Option<Instant>,
    last_match_time_sent: &mut Option<u16>,
    _match_total_maze_entries: &mut usize,
    match_end_pending: &mut Option<(Instant, u32, String)>,
    match_mode: u8,
    match_duration: Option<Duration>,
    team_score: game::TeamScore,
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
            log!("Match duration elapsed — showing end screen");
            *match_running = false;

            // For TDM: determine winner by team total kills
            let (winner_id, winner_name) = if match_mode == MODE_TEAM_DEATHMATCH {
                let (red_kills, blue_kills) = (team_score.red, team_score.blue);
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
            let end_msg = ServerMsg::MatchEnd {
                winner_id: winner_id,
                winner_name: winner_name.to_string(),
                countdown: Some(MATCH_END_COUNTDOWN_SECS),
            }
            .encode();
            for (addr, _) in players.iter() {
                let _ = socket.send_to(&end_msg, *addr);
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
    socket: &Outbox,
    players: &mut HashMap<PeerId, Player>,
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
    log!(
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

/// Match-bot personalities whose difficulty is enabled in `diff_mask`
/// (bit0=Recruit, bit1=Easy, bit2=Normal, bit3=Hard, bit4=Nightmare).
fn eligible_bot_personalities(diff_mask: u8) -> Vec<BotPersonalityId> {
    use crate::bot::personality::{get_personality, BotDifficulty};
    const ALL: [BotPersonalityId; 15] = [
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
    ALL.into_iter()
        .filter(|pid| {
            let bit = match get_personality(*pid).1 {
                BotDifficulty::Recruit => 0,
                BotDifficulty::Easy => 1,
                BotDifficulty::Normal => 2,
                BotDifficulty::Hard => 3,
                BotDifficulty::Nightmare => 4,
            };
            (diff_mask >> bit) & 1 == 1
        })
        .collect()
}

/// The team of each auto-fill bot to add, in spawn order. Team modes take
/// `red` and `blue` bots, interleaved so that if the match is too full for
/// all of them, both teams lose bots evenly. Gun Game has no teams: `red` is
/// the total and every bot is teamless. Never more bots than free match
/// slots or distinct personalities.
fn bot_team_slots(match_mode: u8, red: u8, blue: u8, humans: usize, personalities: usize) -> Vec<Team> {
    let mut slots = Vec::new();
    if match_mode == MODE_GUN_GAME {
        slots.resize(red as usize, Team::Spectator);
    } else {
        for i in 0..red.max(blue) {
            if i < red {
                slots.push(Team::Red);
            }
            if i < blue {
                slots.push(Team::Blue);
            }
        }
    }
    slots.truncate(MATCH_MAX_PLAYERS.saturating_sub(humans).min(personalities));
    slots
}

#[cfg(test)]
mod bot_slot_tests {
    use super::*;

    #[test]
    fn team_modes_take_each_count() {
        let slots = bot_team_slots(MODE_TEAM_DEATHMATCH, 1, 3, 2, 15);
        assert_eq!(slots, vec![Team::Red, Team::Blue, Team::Blue, Team::Blue]);
    }

    #[test]
    fn gun_game_bots_are_teamless() {
        let slots = bot_team_slots(MODE_GUN_GAME, 5, 0, 1, 15);
        assert_eq!(slots, vec![Team::Spectator; 5]);
    }

    #[test]
    fn capped_by_free_slots_and_personalities() {
        assert_eq!(bot_team_slots(MODE_TEAM_DEATHMATCH, 8, 8, 14, 15).len(), 2);
        assert_eq!(bot_team_slots(MODE_GUN_GAME, 15, 0, 0, 4).len(), 4);
        let full = bot_team_slots(MODE_TEAM_DEATHMATCH, 4, 4, 12, 15);
        assert_eq!(full, vec![Team::Red, Team::Blue, Team::Red, Team::Blue]);
    }
}
