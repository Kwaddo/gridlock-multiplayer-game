use std::collections::HashMap;
use crate::net::{Outbox, PeerId};
use std::time::Duration;
use platform::Instant;

use crate::bot::manager::BotManager;
use crate::maze::MazeData;
use crate::player::Team;
use crate::player::{Player, WorldId};
use crate::protocols::*;

use crate::server::{
    broadcast, game,
    game::{
        MATCH_POST_TELEPORT_FREEZE_SECS, MAZE_CELL_SIZE, TEAM_LOADOUT_BLUE, TEAM_LOADOUT_RED,
    },
};

use super::bot_fill::{bot_team_slots, eligible_bot_personalities};

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
            // the lobby's teams are what the match plays (team modes)
            game::balance_lobby_teams(players);
            let (red_humans, blue_humans) = game::lobby_team_counts(players);
            let humans = red_humans + blue_humans;
            let bot_slots = bot_team_slots(
                match_mode,
                autofill_red_bots,
                autofill_blue_bots,
                (red_humans, blue_humans),
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

            for addr in joiners.iter() {
                if let Some(player) = players.get_mut(addr) {
                    player.world = WorldId::Maze;
                    // choose spawn based on match mode / team
                    if let Some((red_spawn, blue_spawn)) = team_spawns {
                        // balanced in the lobby (see game::balance_lobby_teams)
                        let team = if player.team == Team::Blue { Team::Blue } else { Team::Red };
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
                        let team = if player.team == Team::Blue { Team::Blue } else { Team::Red };
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
