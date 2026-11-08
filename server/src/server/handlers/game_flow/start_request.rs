use std::collections::HashMap;
use crate::net::{Outbox, PeerId};
use std::time::Duration;
use platform::Instant;

use crate::maze::MazeData;
use crate::player::Team;
use crate::player::{Player, WorldId};
use crate::protocols::*;

use crate::server::{
    broadcast, game,
    game::{
        MATCH_MAX_PLAYERS, MATCH_POST_TELEPORT_FREEZE_SECS, MATCH_START_COUNTDOWN_SECS,
        MAZE_CELL_SIZE, MIN_PLAYERS_TO_START,
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
