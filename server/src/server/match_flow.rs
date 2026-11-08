//! Lobby settings, match start requests, match status and ending a match.

use std::time::Duration;
use platform::Instant;

use super::{broadcast, game, handlers, Server};
use crate::net::PeerId;
use crate::player::WorldId;
use crate::protocols::*;

impl Server {
    /// `MSG_LOBBY_SETTINGS` from a client: the host changed its lobby console.
    /// Anyone else's is ignored. Valid settings go to every player, so they
    /// can see what the next match will be.
    pub(super) fn handle_lobby_settings(&mut self, src_addr: PeerId, settings: LobbySettings) {
        if game::host_addr(&self.players) != Some(src_addr) {
            return;
        }
        let valid_mode = [MODE_GUN_GAME, MODE_TEAM_DEATHMATCH, MODE_SEARCH_AND_DESTROY].contains(&settings.mode);
        if !valid_mode {
            return;
        }
        let (red_cap, blue_cap) = if settings.mode == MODE_GUN_GAME {
            (game::MAX_FFA_BOTS, 0)
        } else {
            (game::MAX_BOTS_PER_TEAM, game::MAX_BOTS_PER_TEAM)
        };
        let settings = LobbySettings {
            red_bots: settings.red_bots.min(red_cap),
            blue_bots: settings.blue_bots.min(blue_cap),
            diff_mask: settings.diff_mask & 0b11111,
            ..settings
        };
        if settings != self.lobby_settings {
            self.lobby_settings = settings;
            let msg = ServerMsg::LobbySettings(settings).encode();
            broadcast::to_humans(&self.outbox, &self.players, &msg);
        }
    }

    /// `MSG_START_GAME`: the host starts a match from the lobby; while one runs,
    /// anyone in the lobby joins it. The mode, duration and bot settings only
    /// apply to a new match: a player joining never changes the one running.
    pub(super) fn handle_start_request(
        &mut self,
        src_addr: PeerId,
        mode: Option<u8>,
        duration: Option<u16>,
        autofill: Option<(u8, u8)>,
        blue_bots: Option<u8>,
    ) {
        self.end_match_if_abandoned();
        let new_match = !self.match_running
            && self.match_warmup_end.is_none()
            && self.match_end_pending.is_none()
            && game::host_addr(&self.players) == Some(src_addr);
        let mut requested_duration = self.match_duration;
        if new_match {
            if let Some(requested) = mode {
                if requested == MODE_GUN_GAME
                    || requested == MODE_TEAM_DEATHMATCH
                    || requested == MODE_SEARCH_AND_DESTROY
                {
                    self.match_mode = requested;
                }
            }
            // auto-fill settings: bot counts and difficulty mask
            if let Some((bots, diff_mask)) = autofill {
                if self.match_mode == MODE_GUN_GAME {
                    // no teams: one total
                    self.autofill_red_bots = bots.min(game::MAX_FFA_BOTS);
                    self.autofill_blue_bots = 0;
                } else {
                    self.autofill_red_bots = bots.min(game::MAX_BOTS_PER_TEAM);
                    self.autofill_blue_bots =
                        blue_bots.unwrap_or(bots).min(game::MAX_BOTS_PER_TEAM);
                }
                self.autofill_diff_mask = diff_mask & 0b11111;
            } else {
                self.autofill_red_bots = 0;
                self.autofill_blue_bots = 0;
                self.autofill_diff_mask = 0b11111;
            }
            // duration code: 0=default, 65535=unlimited; a non-zero value
            // overrides the mode default
            let dur_code: u16 = duration.unwrap_or(0);
            // what everyone's lobby console shows is what is starting now
            self.handle_lobby_settings(
                src_addr,
                LobbySettings {
                    mode: self.match_mode,
                    duration: dur_code,
                    red_bots: self.autofill_red_bots,
                    blue_bots: self.autofill_blue_bots,
                    diff_mask: self.autofill_diff_mask,
                },
            );
            requested_duration = if dur_code == u16::MAX {
                None // unlimited
            } else if dur_code > 0 {
                Some(Duration::from_secs(dur_code as u64))
            } else {
                match self.match_mode {
                    MODE_TEAM_DEATHMATCH => Some(Duration::from_secs(900)),
                    MODE_SEARCH_AND_DESTROY => None,
                    _ => game::MATCH_DURATION,
                }
            };
        }
        handlers::handle_start_game(
            &self.outbox,
            &mut self.players,
            &mut self.maze,
            &mut self.match_running,
            &mut self.match_start_time,
            &mut self.match_warmup_end,
            &mut self.match_end_pending,
            &mut self.match_total_maze_entries,
            self.match_mode,
            &mut self.match_duration,
            requested_duration,
            src_addr,
        );
        self.send_match_status(false);
    }

    /// The match phase as clients see it (`MATCH_STATUS_*`).
    fn match_status(&self) -> u8 {
        if self.match_end_pending.is_some() {
            MATCH_STATUS_ENDING
        } else if self.match_running {
            MATCH_STATUS_RUNNING
        } else if self.match_warmup_end.is_some() {
            MATCH_STATUS_STARTING
        } else {
            MATCH_STATUS_NONE
        }
    }

    /// Broadcast the match phase when it changed, and every few seconds anyway
    /// (packets get lost, and new players need it). `force` sends now.
    pub(super) fn send_match_status(&mut self, force: bool) {
        let status = self.match_status();
        let due = match self.last_status_sent {
            Some((sent, at)) => sent != status || at.elapsed() >= Duration::from_secs(2),
            None => true,
        };
        if force || due {
            let msg = ServerMsg::MatchStatus { status }.encode();
            broadcast::to_humans(&self.outbox, &self.players, &msg);
            self.last_status_sent = Some((status, Instant::now()));
        }
    }

    /// End a running match once every human has left the Maze (quit or went
    /// back to the lobby), so bots don't play on by themselves.
    pub(super) fn end_match_if_abandoned(&mut self) {
        if !self.match_running {
            return;
        }
        let humans_in_maze = self
            .players
            .values()
            .filter(|p| p.world == WorldId::Maze && !p.is_bot)
            .count();
        if humans_in_maze == 0 {
            log!("No players remain in match — ending match early");
            self.end_match_now();
        }
    }

    /// Handler for `MSG_END_MATCH`: the host stops the match and everyone goes
    /// back to the lobby, still connected. Ignored when no match is on.
    pub(super) fn handle_end_match(&mut self, src_addr: PeerId) {
        if !self.match_running && self.match_end_pending.is_none() {
            return;
        }
        if game::host_addr(&self.players) != Some(src_addr) {
            let msg = ServerMsg::Chat {
                from_id: 0,
                name: String::new(),
                text: "Only the host can end the match".to_string(),
            }
            .encode();
            let _ = self.outbox.send_to(&msg, src_addr);
            return;
        }
        let host = self.players.get(&src_addr).map(|p| p.username.clone()).unwrap_or_default();
        log!("Match ended by host '{}'", host);
        self.end_match_now();
        handlers::broadcast_notice(&self.outbox, &self.players, &format!("{} ended the match", host));
    }

    /// Stop the current match at once: bots despawn, every player returns to
    /// the lobby and match state resets. Connections are kept.
    fn end_match_now(&mut self) {
        handlers::execute_scheduled_match_end(
            &self.outbox,
            &mut self.players,
            &mut self.maze,
            &mut self.match_running,
            &mut self.match_start_time,
            &mut self.match_end_pending,
            &mut self.last_match_time_sent,
            &mut self.match_total_maze_entries,
            &mut self.match_duration,
            String::new(),
            &mut self.snd_state,
            &mut self.bot_manager,
        );
        self.send_match_status(false);
    }
}
