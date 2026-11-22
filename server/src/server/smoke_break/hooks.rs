/*
    smoke break plugged into the server: the tick, what gets sent to who, and the packets that matter
    the rules live in the other files, this one only moves things between them and the server
*/

use std::time::Duration;

use platform::Instant;

use super::coins::GoldenNews;
use super::round_start::{humans_in_match, HUNTER_WEAPON};
use super::shots::Shot;
use super::{AfterResults, SmokePhase};
use crate::net::PeerId;
use crate::player::{Team, WorldId};
use crate::protocols::*;
use crate::server::{broadcast, game, handlers, Server};

/// how often everyone gets their view of the round, often enough for the golden dots and the clock
const SEND_EVERY: Duration = Duration::from_millis(200);

impl Server {
    /// a smoke break match is on right now
    pub(in crate::server) fn smoke_active(&self) -> bool {
        self.match_running && self.match_mode == MODE_SMOKE_BREAK && self.smoke_state.is_some()
    }

    /// one tick of the round, then whatever it needs sent
    pub(in crate::server) fn tick_smoke_break(&mut self, dt: f32) {
        if !self.smoke_active() {
            return;
        }
        let players_before = self.players.len();
        if self.keep_sides_tidy() {
            // under 2 people left, nobody to hunt or hide from: the match is over, most points wins
            handlers::broadcast_notice(&self.outbox, &self.players, "Not enough players left, the match is over");
            return self.end_smoke_break();
        }
        let rounds = self.smoke_rounds;
        let Some(smoke) = self.smoke_state.as_mut() else { return };

        // the very first round starts on the first tick of the match, and knows how many rounds the lobby picked
        if smoke.round == 0 {
            smoke.max_rounds = rounds;
        }
        if smoke.round == 0 && !smoke.start_round(&mut self.players, &mut self.boxes) {
            let text = "Smoke Break needs at least 2 players";
            handlers::broadcast_notice(&self.outbox, &self.players, text);
            return self.end_smoke_break();
        }

        smoke.tick(dt);
        smoke.walk_crowd(dt, &mut self.players);
        smoke.pick_up_coins(&self.players);
        smoke.tick_ability(dt, &self.players);

        // the results board ran out: the next round, or that was the last
        match smoke.after_results(humans_in_match(&self.players).len()) {
            AfterResults::Waiting => {}
            AfterResults::MatchOver => return self.end_smoke_break(),
            AfterResults::NextRound => {
                if !smoke.start_round(&mut self.players, &mut self.boxes) {
                    return self.end_smoke_break();
                }
            }
        }

        if self.players.len() != players_before {
            // decoys came or went, everyone's lists need it now
            broadcast::broadcast_state(&self.outbox, &self.players);
            broadcast::broadcast_player_list(&self.outbox, &self.players);
        }
        self.send_smoke_break();
    }

    /// people who left drop out of the round, people who just walked in hunt (they missed the hide)
    // I realize this makes it so the rule of hunters number < hiders number is not always true, but I figure its fine so we can have anyone join in
    // and for the next round its gonna be a new set of hunters and hiders anyway, so its not like it matters
    /// true when fewer than 2 are left and the match has to end
    fn keep_sides_tidy(&mut self) -> bool {
        let Some(smoke) = self.smoke_state.as_mut() else { return false };
        let players = &mut self.players;
        let here = |peer: &PeerId| super::is_decoy(*peer) || players.get(peer).is_some_and(|p| p.world == WorldId::Maze);
        smoke.hunters.retain(here);
        smoke.hiders.retain(here);
        smoke.starting_hunters.retain(here);
        if smoke.round > 0 {
            for peer in humans_in_match(players) {
                if !smoke.hunters.contains(&peer) && !smoke.hiders.contains(&peer) {
                    smoke.hunters.insert(peer);
                    if let Some(p) = players.get_mut(&peer) {
                        p.current_weapon = HUNTER_WEAPON.to_string();
                        p.team = Team::Spectator;
                    }
                }
            }
        }
        if smoke.round > 0 && humans_in_match(players).len() < 2 {
            return true;
        }
        let live = matches!(smoke.phase, SmokePhase::Blind | SmokePhase::Released | SmokePhase::Coins);
        if smoke.round > 0 && live {
            if smoke.hiders.is_empty() {
                // the last hider walked out, so that's a hunters win
                smoke.end_round(SMOKE_SIDE_HUNTER);
            } else if smoke.hunters.is_empty() {
                // every hunter walked out, the hiders win it and the next round reshuffles
                smoke.end_round(SMOKE_SIDE_HIDER);
            }
        }
        false
    }

    /// what changed goes out now, everyone's own view of the round goes out every SEND_EVERY
    fn send_smoke_break(&mut self) {
        let Some(smoke) = self.smoke_state.as_mut() else { return };

        // the round just started, move everyone's game to where the server put them
        if !smoke.respawns.is_empty() {
            let text = format!("Round {}", smoke.round);
            handlers::broadcast_notice(&self.outbox, &self.players, &text);
        }
        for (peer, at) in smoke.respawns.drain(..) {
            let msg = ServerMsg::Respawn { position: at, kind: None }.encode();
            let _ = self.outbox.send_to(&msg, peer);
        }
        // every coin grab, for the sound
        for at in smoke.taken.drain(..) {
            let msg = ServerMsg::SmokeCoinTaken { at }.encode();
            broadcast::to_humans(&self.outbox, &self.players, &msg);
        }
        // golden coin news, hiders get more than hunters
        let news: Vec<GoldenNews> = smoke.golden_news.drain(..).collect();
        for item in news {
            let (to_hiders, to_hunters) = match item {
                GoldenNews::Out => (
                    "A golden coin just appeared! It's on your minimap for 30 s".to_string(),
                    "A golden coin just appeared somewhere...".to_string(),
                ),
                GoldenNews::Taken(peer) => {
                    let name = self.players.get(&peer).map(|p| p.username.clone()).unwrap_or_default();
                    let text = format!("{name} grabbed the golden coin!");
                    (text.clone(), text)
                }
                GoldenNews::Gone => ("The golden coin is gone".to_string(), "The golden coin is gone".to_string()),
            };
            for peer in humans_in_match(&self.players) {
                let text = if smoke.side_of(peer) == SMOKE_SIDE_HIDER { &to_hiders } else { &to_hunters };
                let msg = ServerMsg::Chat { from_id: 0, name: String::new(), text: text.clone() }.encode();
                let _ = self.outbox.send_to(&msg, peer);
            }
        }

        // box shots for the kill feed, everyone sees who found who and who's shooting decoys
        for line in smoke.feed.drain(..) {
            let id_of = |peer: PeerId| self.players.get(&peer).map_or(0, |p| p.id);
            let msg = ServerMsg::SmokeFeed {
                shooter: id_of(line.shooter),
                target: line.found.map_or(0, id_of),
                red_handed: line.red_handed,
                points: line.points.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
            }
            .encode();
            broadcast::to_humans(&self.outbox, &self.players, &msg);
        }

        if self.smoke_last_send.is_some_and(|t| t.elapsed() < SEND_EVERY) {
            return;
        }
        self.smoke_last_send = Some(Instant::now());
        let scores = ServerMsg::SmokeScores(smoke.scores(&self.players)).encode();
        let golden = ServerMsg::SmokeGolden(smoke.golden_ids(&self.players)).encode();
        for peer in humans_in_match(&self.players) {
            let messages = [
                ServerMsg::SmokeRound(smoke.round_for(peer)).encode(),
                ServerMsg::SmokeCoins(smoke.coins_for(peer)).encode(),
                ServerMsg::SmokeGoldCoin(smoke.gold_coin_for(peer)).encode(),
                ServerMsg::SmokeDots(smoke.dots_for(peer, &self.players)).encode(),
            ];
            for msg in messages.iter().chain([&scores, &golden]) {
                let _ = self.outbox.send_to(msg, peer);
            }
        }
    }

    /// the last round's done, too few are left, or the host ended it: the end screen, the usual countdown sends everyone home
    pub(in crate::server) fn end_smoke_break(&mut self) {
        let (winner_id, winner_name) = self
            .smoke_state
            .as_ref()
            .and_then(|s| s.leader(&self.players))
            .unwrap_or((0, "Nobody".to_string()));
        log!("Smoke Break over: {} has the most points", winner_name);
        let end = ServerMsg::MatchEnd {
            winner_id,
            winner_name: winner_name.clone(),
            countdown: Some(game::MATCH_END_COUNTDOWN_SECS),
        }
        .encode();
        broadcast::to_humans(&self.outbox, &self.players, &end);
        self.match_end_pending = Some((
            Instant::now() + Duration::from_secs(game::MATCH_END_COUNTDOWN_SECS as u64),
            winner_id,
            winner_name,
        ));
        self.match_running = false;
    }

    /// the match is fully over (any way it ended): the round, the crowd and the maze's boxes all go
    pub(in crate::server) fn forget_smoke_break(&mut self) {
        if self.smoke_state.is_some() && !self.match_running && self.match_end_pending.is_none() {
            self.smoke_state = None;
            self.smoke_last_send = None;
            self.boxes.clear_world(WorldId::Maze);
            super::remove_smoke_entities(&mut self.players);
        }
    }

    /// a box got shot during smoke break, the round decides what happens
    pub(in crate::server) fn smoke_box_shot(&mut self, peer: PeerId, id: u16) {
        let Some(smoke) = self.smoke_state.as_mut() else { return };
        let players_before = self.players.len();
        match smoke.box_shot(peer, id, &mut self.players, &mut self.boxes) {
            // not theirs to shoot, their game already broke it so it gets the list back
            // finds and decoys go out as feed lines with the next send, no chat notice
            Shot::NotAllowed => self.boxes.resend_to(peer),
            Shot::Found(_) | Shot::Decoy | Shot::Empty => {}
        }
        if self.players.len() != players_before {
            broadcast::broadcast_state(&self.outbox, &self.players);
        }
        broadcast::broadcast_player_list(&self.outbox, &self.players);
    }

    pub(in crate::server) fn smoke_go_golden(&mut self, peer: PeerId) {
        if let Some(smoke) = self.smoke_state.as_mut() {
            smoke.go_golden(peer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::smoke_break::{is_decoy, SmokeState, SMOKE_BLIND_TIME, SMOKE_RELEASED_TIME};

    // two people joined and standing in a smoke break match, first tick not run yet
    fn match_of_two() -> (Server, PeerId, PeerId) {
        let mut server = Server::new();
        let (a, b) = (PeerId(1), PeerId(2));
        server.handle_packet(a, &ClientMsg::join("alice").encode());
        server.handle_packet(b, &ClientMsg::join("bob").encode());
        for p in server.players.values_mut() {
            p.world = WorldId::Maze;
        }
        server.match_mode = MODE_SMOKE_BREAK;
        server.match_running = true;
        server.maze = Some(crate::maze::MazeData::smoke_break_arena());
        server.smoke_state = Some(SmokeState::new());
        (server, a, b)
    }

    fn messages(out: &[(PeerId, Vec<u8>)], peer: PeerId) -> Vec<ServerMsg> {
        out.iter().filter(|(p, _)| *p == peer).filter_map(|(_, m)| ServerMsg::decode(m)).collect()
    }

    #[test]
    fn the_first_tick_starts_round_1_and_moves_everyone() {
        let (mut server, a, b) = match_of_two();
        server.tick();
        let smoke = server.smoke_state.as_ref().unwrap();
        assert_eq!(smoke.round, 1);
        assert!(server.players.keys().any(|p| is_decoy(*p)), "the crowd is out");
        let out = server.drain_outbox();
        for peer in [a, b] {
            let got = messages(&out, peer);
            assert!(got.iter().any(|m| matches!(m, ServerMsg::Respawn { .. })), "{peer} wasn't moved");
            assert!(got.iter().any(|m| matches!(m, ServerMsg::SmokeRound(_))), "{peer} got no round");
        }
    }

    #[test]
    fn coins_only_ever_reach_hiders() {
        let (mut server, a, b) = match_of_two();
        server.tick();
        {
            let smoke = server.smoke_state.as_mut().unwrap();
            smoke.tick(SMOKE_BLIND_TIME);
            smoke.tick(SMOKE_RELEASED_TIME);
        }
        server.smoke_last_send = None;
        server.drain_outbox();
        server.tick();
        let out = server.drain_outbox();
        let smoke = server.smoke_state.as_ref().unwrap();
        let hunter = *smoke.hunters.iter().next().unwrap();
        let hider = if hunter == a { b } else { a };
        let coins = |peer| {
            messages(&out, peer)
                .into_iter()
                .filter_map(|m| if let ServerMsg::SmokeCoins(c) = m { Some(c) } else { None })
                .last()
                .expect("no coin message")
        };
        assert!(coins(hunter).is_empty());
        // the hider may have just grabbed one they were standing on, so compare with what's actually out
        assert!(!smoke.coins.is_empty());
        assert_eq!(coins(hider).len(), smoke.coins.len());
    }

    #[test]
    fn alone_the_match_ends_with_a_notice() {
        let mut server = Server::new();
        server.handle_packet(PeerId(1), &ClientMsg::join("alice").encode());
        server.players.get_mut(&PeerId(1)).unwrap().world = WorldId::Maze;
        server.match_mode = MODE_SMOKE_BREAK;
        server.match_running = true;
        server.smoke_state = Some(SmokeState::new());
        server.tick();
        assert!(!server.match_running);
        assert!(server.match_end_pending.is_some());
    }

    #[test]
    fn one_left_and_the_match_ends_with_a_winner() {
        let (mut server, _, _) = match_of_two();
        server.tick();
        let hider = *server.smoke_state.as_ref().unwrap().hiders.iter().next().unwrap();
        server.players.remove(&hider);
        server.tick();
        assert!(!server.match_running);
        assert!(server.match_end_pending.is_some(), "the winner screen");
    }

    #[test]
    fn every_hunter_leaving_gives_the_hiders_the_round() {
        let (mut server, _, _) = match_of_two();
        server.handle_packet(PeerId(3), &ClientMsg::join("cara").encode());
        server.players.get_mut(&PeerId(3)).unwrap().world = WorldId::Maze;
        server.tick();
        let hunter = *server.smoke_state.as_ref().unwrap().hunters.iter().next().unwrap();
        server.players.remove(&hunter);
        server.tick();
        let smoke = server.smoke_state.as_ref().unwrap();
        assert_eq!((smoke.phase, smoke.winner), (SmokePhase::Results, SMOKE_SIDE_HIDER));
    }

    #[test]
    fn after_the_results_the_next_round_starts_on_its_own() {
        let (mut server, _, _) = match_of_two();
        server.tick();
        {
            let smoke = server.smoke_state.as_mut().unwrap();
            smoke.end_round(SMOKE_SIDE_HIDER);
            smoke.phase_timer = 0.0;
        }
        server.tick();
        let smoke = server.smoke_state.as_ref().unwrap();
        assert_eq!((smoke.round, smoke.phase), (2, SmokePhase::Blind));
    }

    #[test]
    fn the_last_round_ends_the_match() {
        let (mut server, _, _) = match_of_two();
        server.tick();
        {
            let smoke = server.smoke_state.as_mut().unwrap();
            smoke.max_rounds = 1;
            smoke.end_round(SMOKE_SIDE_HIDER);
            smoke.phase_timer = 0.0;
        }
        server.tick();
        assert!(server.match_end_pending.is_some());
    }

    #[test]
    fn the_first_round_takes_the_lobbys_round_count() {
        let (mut server, _, _) = match_of_two();
        server.smoke_rounds = 10;
        server.tick();
        assert_eq!(server.smoke_state.as_ref().unwrap().max_rounds, 10);
    }

    #[test]
    fn the_host_ending_it_shows_the_winner_first() {
        let (mut server, a, _) = match_of_two();
        server.tick();
        server.handle_packet(a, &ClientMsg::EndMatch.encode());
        assert!(server.match_end_pending.is_some(), "the winner screen, not straight home");
    }

    #[test]
    fn nobody_takes_damage_and_nobody_climbs_out() {
        let (mut server, a, b) = match_of_two();
        server.tick();
        let hp = server.players[&b].health;
        let hit = ClientMsg::Hit { victim_id: server.players[&b].id, damage: 50.0, weapon: None, backstab: None, headshot: None };
        server.handle_packet(a, &hit.encode());
        assert_eq!(server.players[&b].health, hp, "no damage in smoke break");
        let hider = *server.smoke_state.as_ref().unwrap().hiders.iter().next().unwrap();
        server.handle_packet(hider, &ClientMsg::BoxExit.encode());
        server.tick();
        assert!(server.boxes.is_carrying(hider), "hiders can't leave their box");
    }

    #[test]
    fn when_the_match_ends_the_crowd_never_reaches_the_lobby() {
        let (mut server, _, _) = match_of_two();
        server.tick();
        server.handle_packet(PeerId(1), &ClientMsg::EndMatch.encode());
        // the winner screen's countdown runs out
        if let Some(end) = server.match_end_pending.as_mut() {
            end.0 = Instant::now();
        }
        // one tick sends everyone home, the next clears the smoke break away
        server.tick();
        server.tick();
        assert!(server.players.keys().all(|p| !is_decoy(*p)), "the decoys are gone");
        assert!(server.smoke_state.is_none());
    }
}