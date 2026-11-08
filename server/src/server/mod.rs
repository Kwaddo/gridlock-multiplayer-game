use std::collections::HashMap;
use std::time::Duration;
use platform::Instant;

pub mod broadcast;
pub mod game;
pub mod handlers;
pub mod snd;
pub mod utils;
mod match_flow;
mod packet;
mod tick;
#[cfg(test)]
mod tests;

use crate::bot::manager::BotManager;
use crate::maze::MazeData;
use crate::net::{Outbox, PeerId};
use crate::player::Player;
use crate::protocols::*;

/// Authoritative game state. Previously the locals of a blocking `run()` loop;
/// now driven by whichever host owns the transport.
pub struct Server {
    outbox: Outbox,
    players: HashMap<PeerId, Player>,
    bot_manager: BotManager,
    // cached assets manifest, sent to each client on join
    // shown to clients browsing for games
    name: String,
    // Do not generate a full gameplay maze at server start — create per-match instead.
    maze: Option<MazeData>, // Some(...) when a match is running
    match_running: bool,
    match_start_time: Option<Instant>,
    // current running match length (None=infinite); initially use the default
    match_duration: Option<Duration>,
    // warmup / countdown before match actually begins
    match_warmup_end: Option<Instant>,
    // scheduled match end (winner, delayed return-to-lobby)
    match_end_pending: Option<(Instant, u32, String)>,
    // last sent authoritative match timer (to avoid spamming identical values)
    last_match_time_sent: Option<u16>,
    /// Last match timer sent: an infinite match repeats it as a 1 Hz heartbeat.
    last_match_heartbeat: Option<Instant>,
    // last sent warmup countdown (MSG_MATCH_COUNTDOWN)
    last_warmup_sent: Option<u8>,
    // last sent match-end countdown (MSG_MATCH_END countdown byte)
    last_end_countdown_sent: Option<u8>,
    // current match mode (defaults to Team Deathmatch)
    match_mode: u8,
    // how many players entered the maze during this match (for last-man-standing detection)
    match_total_maze_entries: usize,
    // Search & Destroy state (only Some when S&D mode is active)
    snd_state: Option<snd::SndState>,
    // Auto-fill bot settings (set by the client that starts the match)
    // auto-fill bot counts: Red/Blue in team modes; Gun Game uses only
    // `autofill_red_bots`, as the total (0 = no bots)
    autofill_red_bots: u8,
    autofill_blue_bots: u8,
    autofill_diff_mask: u8,     // all difficulties enabled by default
    // what the host has picked in its lobby console, passed on to everyone
    lobby_settings: LobbySettings,
    // Track last tick time for delta-time calculation
    last_tick: Instant,
    // Regen fires at most once per second to avoid per-packet health spikes
    last_regen_tick: Instant,
    last_state_broadcast: Option<Instant>,
    /// Last run of the tick's rate-limited bookkeeping (inactivity timeouts).
    last_bookkeeping: Option<Instant>,
    /// Player count at the last state broadcast: a join or leave sends at once.
    last_broadcast_players: usize,
    /// When each player last chatted (chat rate limit).
    last_chat: HashMap<PeerId, Instant>,
    /// Last `MSG_MATCH_STATUS` broadcast: value and time.
    last_status_sent: Option<(u8, Instant)>,
    /// Team kills this match (team modes).
    team_score: game::TeamScore,
    /// Last `MSG_TEAM_SCORE` broadcast: value and time.
    last_team_score_sent: Option<(game::TeamScore, Instant)>,
}

impl Server {
    pub fn new() -> Self {
        Server {
            outbox: Outbox::default(),
            players: HashMap::new(),
            bot_manager: BotManager::new(),
            name: "Gridlock".to_string(),
            maze: None,
            match_running: false,
            match_start_time: None,
            match_duration: game::MATCH_DURATION,
            match_warmup_end: None,
            match_end_pending: None,
            last_match_time_sent: None,
            last_match_heartbeat: None,
            last_warmup_sent: None,
            last_end_countdown_sent: None,
            match_mode: MODE_TEAM_DEATHMATCH,
            match_total_maze_entries: 0,
            snd_state: None,
            autofill_red_bots: 0,
            autofill_blue_bots: 0,
            autofill_diff_mask: 0b11111,
            lobby_settings: LobbySettings {
                mode: MODE_TEAM_DEATHMATCH,
                duration: 0,
                red_bots: 0,
                blue_bots: 0,
                diff_mask: 0b11111,
            },
            last_tick: Instant::now(),
            last_regen_tick: Instant::now(),
            last_state_broadcast: None,
            last_bookkeeping: None,
            last_broadcast_players: 0,
            last_chat: HashMap::new(),
            last_status_sent: None,
            team_score: game::TeamScore::default(),
            last_team_score_sent: None,
        }
    }

    /// Name shown to clients browsing for games.
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Connected players, bots included (the same count discovery reports).
    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    /// Packets queued since the last call, as `(destination, bytes)`.
    pub fn drain_outbox(&mut self) -> Vec<(PeerId, Vec<u8>)> {
        self.outbox.drain()
    }

    /// Whether `peer` currently owns a player (joined and not timed out).
    pub fn has_peer(&self, peer: PeerId) -> bool {
        self.players.contains_key(&peer)
    }

    /// Transport reported the peer gone (e.g. WebRTC channel closed).
    pub fn peer_disconnected(&mut self, src_addr: PeerId) {
        handlers::handle_disconnect(&self.outbox, &mut self.players, src_addr);
    }
}
