//! The periodic simulation step and its timed broadcasts.

use std::time::Duration;
use platform::Instant;

use super::{broadcast, game, handlers, Server};
use crate::protocols::*;

impl Server {
    /// Periodic simulation step; call after each packet and at least every 100ms.
    pub fn tick(&mut self) {
        let now_tick = Instant::now();
        let dt = now_tick.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now_tick;

        // Auto-respawn dead players EXCEPT in Search & Destroy mode (one life per round)
        if self.match_mode != MODE_SEARCH_AND_DESTROY {
            game::process_auto_respawns(&self.outbox, self.maze.as_ref(), &mut self.players, self.match_mode);
        }

        // S&D tick: advance round state
        if self.match_running && self.match_mode == MODE_SEARCH_AND_DESTROY {
            if let Some(ref mut snd) = self.snd_state {
                if let Some((winner_name, winner_id)) = snd.tick(dt, &self.outbox, &mut self.players) {
                    // S&D match is over
                    log!("S&D match over: {} wins", winner_name);
                    let end_msg = ServerMsg::MatchEnd {
                        winner_id: winner_id,
                        winner_name: winner_name.to_string(),
                        countdown: Some(game::MATCH_END_COUNTDOWN_SECS),
                    }
                    .encode();
                    for (addr, _) in self.players.iter() {
                        let _ = self.outbox.send_to(&end_msg, *addr);
                    }
                    self.match_end_pending = Some((
                        Instant::now() + Duration::from_secs(game::MATCH_END_COUNTDOWN_SECS as u64),
                        winner_id,
                        winner_name,
                    ));
                    self.match_running = false;
                    self.snd_state = None;
                }
            }
        }
        // tick bots
        self.bot_manager.tick(
            &self.outbox,
            &mut self.players,
            &mut self.match_end_pending,
            self.match_mode,
            self.maze.as_ref(),
            &mut self.team_score,
        );
        if self.bot_manager.handle_range_challenge_timeout(&mut self.players) {
            broadcast::broadcast_state(&self.outbox, &self.players);
            broadcast::broadcast_player_list(&self.outbox, &self.players);
        }
        // players come and go and return from matches: keep the lobby's
        // teams even for the next match
        if game::balance_lobby_teams(&mut self.players) {
            broadcast::broadcast_player_list(&self.outbox, &self.players);
        }
        if self.last_regen_tick.elapsed() >= Duration::from_secs(1) {
            game::process_lobby_regen(&mut self.players);
            self.last_regen_tick = Instant::now();
        }
        let due = self
            .last_state_broadcast
            .map_or(true, |t| t.elapsed() >= game::STATE_BROADCAST_INTERVAL);
        if due || self.players.len() != self.last_broadcast_players {
            self.last_state_broadcast = Some(Instant::now());
            self.last_broadcast_players = self.players.len();
            broadcast::broadcast_state_with_events(&self.outbox, &self.players);
            // One-shot movement events have now gone out; they stay set until
            // then, however many position updates arrive in between.
            for player in self.players.values_mut() {
                player.evt_jump_start = false;
                player.evt_landed = false;
            }
        }

        let timed_out = game::cleanup_inactive_players(&mut self.players);
        if !timed_out.is_empty() {
            for p in &timed_out {
                handlers::announce_left(&self.outbox, &self.players, p, LEAVE_TIMED_OUT);
            }
            // players were removed — broadcast so other clients drop their characters
            broadcast::broadcast_state(&self.outbox, &self.players);
            broadcast::broadcast_player_list(&self.outbox, &self.players);
        }
        if self.last_chat.len() > self.players.len() {
            let players = &self.players;
            self.last_chat.retain(|addr, _| players.contains_key(addr));
        }

        self.end_match_if_abandoned();

        // Warmup expiry: actually start the match when the warmup countdown ends
        if self.match_warmup_end.is_some() && !self.match_running {
            // every match starts from nil
            self.team_score = game::TeamScore::default();
            handlers::maybe_start_match(
                &self.outbox,
                &mut self.players,
                &mut self.maze,
                &mut self.match_running,
                &mut self.match_start_time,
                &mut self.match_warmup_end,
                &mut self.last_match_time_sent,
                &mut self.match_total_maze_entries,
                self.match_mode,
                &mut self.snd_state,
                self.match_duration,
                &mut self.bot_manager,
                self.autofill_red_bots,
                self.autofill_blue_bots,
                self.autofill_diff_mask,
            );
        }

        // Periodically rebroadcast warmup countdown so clients that missed
        // the original packet can catch up (helps offscreen/paused clients).
        if let Some(end) = self.match_warmup_end {
            let secs = if Instant::now() >= end {
                0u8
            } else {
                end.saturating_duration_since(Instant::now())
                    .as_secs()
                    .min(255) as u8
            };
            if self.last_warmup_sent != Some(secs) {
                let cd = ServerMsg::MatchCountdown { secs, mode: Some(self.match_mode) }.encode();
                for (addr, _) in self.players.iter() {
                    let _ = self.outbox.send_to(&cd, *addr);
                }
                self.last_warmup_sent = Some(secs);
            }
        } else {
            self.last_warmup_sent = None;
        }

        // If a match is running, check player counts for early end conditions (skip for S&D, it handles its own)
        if self.match_running && self.match_end_pending.is_none() && self.match_mode != MODE_SEARCH_AND_DESTROY {
            handlers::check_early_end_conditions(
                &self.outbox,
                &mut self.players,
                &mut self.maze,
                &mut self.match_running,
                &mut self.match_end_pending,
                &mut self.match_total_maze_entries,
                &mut self.match_start_time,
                self.match_mode,
                &mut self.match_duration,
                self.team_score,
            );
        }

        // Basic match timer: end match after MATCH_DURATION (skip for S&D, rounds handle timing)
        if self.match_running && self.match_mode != MODE_SEARCH_AND_DESTROY {
            handlers::update_match_timer(
                &self.outbox,
                &mut self.players,
                &mut self.maze,
                &mut self.match_running,
                &mut self.match_start_time,
                &mut self.last_match_time_sent,
                &mut self.match_total_maze_entries,
                &mut self.match_end_pending,
                self.match_mode,
                self.match_duration,
                self.team_score,
            );
        }

        // If a match-end was scheduled (winner reached score or last-man-standing), execute it when the countdown expires
        if self.match_end_pending.as_ref().map_or(false, |(when, _, _)| Instant::now() >= *when) {
            let winner_name = self.match_end_pending.as_ref().map(|(_, _, n)| n.clone()).unwrap_or_default();
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
                winner_name,
                &mut self.snd_state,
                &mut self.bot_manager,
            );
        }

        // Rebroadcast match-end countdown periodically so offscreen clients
        // display the remaining end-of-match timer even if they missed the
        // original MSG_MATCH_END packet.
        if let Some((when, winner_id, winner_name)) = self.match_end_pending.as_ref() {
            let secs = if Instant::now() >= *when {
                0u8
            } else {
                when.saturating_duration_since(Instant::now())
                    .as_secs()
                    .min(255) as u8
            };
            if self.last_end_countdown_sent != Some(secs) {
                let end_msg = ServerMsg::MatchEnd {
                    winner_id: *winner_id,
                    winner_name: winner_name.to_string(),
                    countdown: Some(secs),
                }
                .encode();
                for (addr, _) in self.players.iter() {
                    let _ = self.outbox.send_to(&end_msg, *addr);
                }
                self.last_end_countdown_sent = Some(secs);
            }
        } else {
            self.last_end_countdown_sent = None;
        }
        self.send_match_status(false);
        self.send_team_score();
    }

    /// Broadcast the team score during a Team Deathmatch: when it changes, and
    /// every few seconds anyway (packets get lost, and new players need it).
    fn send_team_score(&mut self) {
        let in_play = self.match_mode == MODE_TEAM_DEATHMATCH
            && (self.match_running || self.match_end_pending.is_some());
        if !in_play {
            self.last_team_score_sent = None;
            return;
        }
        let score = self.team_score;
        let due = match self.last_team_score_sent {
            Some((sent, at)) => sent != score || at.elapsed() >= Duration::from_secs(2),
            None => true,
        };
        if due {
            let clamp = |v: u32| v.min(u16::MAX as u32) as u16;
            let msg = ServerMsg::TeamScore {
                red: clamp(score.red),
                blue: clamp(score.blue),
                to_win: clamp(game::MATCH_SCORE_TO_WIN),
            }
            .encode();
            broadcast::to_humans(&self.outbox, &self.players, &msg);
            self.last_team_score_sent = Some((score, Instant::now()));
        }
    }
}
