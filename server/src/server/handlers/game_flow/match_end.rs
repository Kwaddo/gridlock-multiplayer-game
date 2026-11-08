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
    game::{MATCH_END_COUNTDOWN_SECS, MATCH_SCORE_TO_WIN},
};

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

/// How often an infinite match re-sends its (unchanging) timer as a heartbeat.
const MATCH_TIMER_HEARTBEAT: Duration = Duration::from_secs(1);

/// Update the authoritative match timer and check for timeout-based end.
pub fn update_match_timer(
    socket: &Outbox,
    players: &mut HashMap<PeerId, Player>,
    _maze: &mut Option<MazeData>,
    match_running: &mut bool,
    match_start_time: &mut Option<Instant>,
    last_match_time_sent: &mut Option<u16>,
    last_match_heartbeat: &mut Option<Instant>,
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
        // The heartbeat goes out at most once a second: well inside the
        // client's connection-lost timeout, without a packet on every tick.
        let heartbeat_due = match_duration.is_none()
            && last_match_heartbeat.map_or(true, |at| at.elapsed() >= MATCH_TIMER_HEARTBEAT);
        if heartbeat_due || *last_match_time_sent != Some(remaining_secs) {
            let _ = broadcast::broadcast_match_timer(socket, players, remaining_secs);
            *last_match_time_sent = Some(remaining_secs);
            *last_match_heartbeat = Some(Instant::now());
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
