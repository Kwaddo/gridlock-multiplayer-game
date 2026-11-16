use std::collections::HashMap;
use crate::net::{Outbox, PeerId};
use std::time::Duration;
use platform::Instant;

use crate::bot::manager::BotManager;
use crate::maze::MazeData;
use crate::player::{Player, WorldId};
use crate::protocols::*;

use crate::server::snd::SndState;
use crate::server::{
    game,
    game::{MATCH_START_COUNTDOWN_SECS, MIN_PLAYERS_TO_START},
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
    bot_manager: &mut BotManager,
    snd_state: &mut Option<SndState>,
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
        // match is already running: the requester joins it as it is
        super::late_join::join_running_match(
            socket,
            players,
            maze,
            bot_manager,
            snd_state,
            match_mode,
            *match_start_time,
            *match_duration,
            match_total_maze_entries,
            src_addr,
        );
    }
}
