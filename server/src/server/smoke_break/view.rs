/*
    what each player sees
    everyone: their own copy of the round, the scores (humans only), which boxes are gold
    hiders only: the coins and the golden coin, hunters only: the golden dots
    building them is here, sending them is in hooks.rs
*/

use std::collections::HashMap;

use super::round_start::{humans_in_match, is_smoke_entity};
use super::{SmokePhase, SmokeState};
use crate::net::PeerId;
use crate::player::Player;
use crate::protocols::*;

impl SmokeState {
    pub fn side_of(&self, peer: PeerId) -> u8 {
        if self.hunters.contains(&peer) {
            SMOKE_SIDE_HUNTER
        } else {
            SMOKE_SIDE_HIDER
        }
    }

    /// a human still hiding, the only ones who ever get told where coins are
    fn hiding(&self, peer: PeerId) -> bool {
        self.hiders.contains(&peer) && !is_smoke_entity(peer)
    }

    /// the round as `peer` sees it
    pub(in crate::server) fn round_for(&self, peer: PeerId, players: &HashMap<PeerId, Player>) -> SmokeRound {
        let in_vote = self.phase == SmokePhase::Vote;
        SmokeRound {
            round: self.round,
            phase: self.phase as u8,
            phase_timer: self.phase_timer.max(0.0),
            coins_left: self.coins.len() as u16,
            side: self.side_of(peer),
            votes_stay: if in_vote { self.votes.values().filter(|stay| **stay).count() as u8 } else { 0 },
            voters: if in_vote { humans_in_match(players).len() as u8 } else { 0 },
            golden_ready: self.can_go_golden(peer),
        }
    }

    /// every human in the match with their points, best first, bots and decoys aren't players here
    pub(in crate::server) fn scores(&self, players: &HashMap<PeerId, Player>) -> Vec<SmokeScore> {
        let mut rows: Vec<SmokeScore> = humans_in_match(players)
            .into_iter()
            .filter_map(|peer| {
                let p = players.get(&peer)?;
                let points = self.points.get(&peer).copied().unwrap_or(0);
                Some(SmokeScore { id: p.id, points, side: self.side_of(peer) })
            })
            .collect();
        rows.sort_by(|a, b| b.points.cmp(&a.points).then(a.id.cmp(&b.id)));
        rows
    }

    /// whoever has the most points, for the end of the match
    pub(in crate::server) fn leader(&self, players: &HashMap<PeerId, Player>) -> Option<(u32, String)> {
        let best = self.scores(players).into_iter().next()?;
        let name = players.values().find(|p| p.id == best.id)?.username.clone();
        Some((best.id, name))
    }

    /// where the coins are, hiders only, everyone else gets an empty list (never an old one)
    pub(in crate::server) fn coins_for(&self, peer: PeerId) -> Vec<(f32, f32, f32)> {
        if self.hiding(peer) {
            self.coins.clone()
        } else {
            Vec::new()
        }
    }

    /// the golden coin and its time left, hiders only
    pub(in crate::server) fn gold_coin_for(&self, peer: PeerId) -> Option<((f32, f32, f32), f32)> {
        self.golden.filter(|_| self.hiding(peer))
    }

    /// player ids of everyone gold right now, everyone gets this so the box glows for whoever can see it
    pub(in crate::server) fn golden_ids(&self, players: &HashMap<PeerId, Player>) -> Vec<u32> {
        self.goldens
            .iter()
            .filter(|(_, g)| g.is_gold())
            .filter_map(|(peer, _)| players.get(peer).map(|p| p.id))
            .collect()
    }

    /// minimap dots for hunters: gold hiders where they are now, and where the gold wore off for a bit after
    pub(in crate::server) fn dots_for(&self, peer: PeerId, players: &HashMap<PeerId, Player>) -> Vec<((f32, f32, f32), bool)> {
        if !self.hunters.contains(&peer) {
            return Vec::new();
        }
        self.goldens
            .iter()
            .filter_map(|(gold_peer, g)| {
                if g.is_gold() {
                    players.get(gold_peer).map(|p| (p.position, true))
                } else if g.show_last_seen() {
                    g.seen_at.map(|at| (at, false))
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::WorldId;
    use crate::server::boxes::Boxes;
    use crate::server::smoke_break::round_start::is_decoy;
    use crate::server::smoke_break::{SMOKE_BLIND_TIME, SMOKE_RELEASED_TIME};

    // `humans` people and `bots` bot hiders, ticked into the coin phase
    fn coin_phase(humans: u32, bots: usize) -> (SmokeState, HashMap<PeerId, Player>) {
        let mut players: HashMap<PeerId, Player> = (1..=humans)
            .map(|i| {
                let mut p = Player::new(i, format!("p{i}"));
                p.world = WorldId::Maze;
                (PeerId(i), p)
            })
            .collect();
        let mut s = SmokeState::new(bots);
        s.start_round(&mut players, &mut Boxes::new());
        s.tick(SMOKE_BLIND_TIME);
        s.tick(SMOKE_RELEASED_TIME);
        (s, players)
    }

    fn sides(s: &SmokeState) -> (PeerId, PeerId) {
        let hunter = *s.hunters.iter().next().unwrap();
        let hider = *s.hiders.iter().find(|p| !is_smoke_entity(**p)).unwrap();
        (hunter, hider)
    }

    #[test]
    fn hunters_never_get_coins_or_the_golden_coin() {
        let (mut s, _) = coin_phase(2, 0);
        let (hunter, hider) = sides(&s);
        s.golden = Some(((5.0, 0.0, 5.0), 20.0));
        assert!(s.coins_for(hunter).is_empty());
        assert_eq!(s.gold_coin_for(hunter), None);
        assert_eq!(s.coins_for(hider).len(), 20);
        assert!(s.gold_coin_for(hider).is_some());
    }

    #[test]
    fn hiders_never_get_golden_dots() {
        let (mut s, players) = coin_phase(3, 0);
        let (hunter, hider) = sides(&s);
        assert!(s.go_golden(hider));
        assert_eq!(s.dots_for(hunter, &players), vec![(players[&hider].position, true)]);
        let other_hider = *s.hiders.iter().find(|p| **p != hider).unwrap();
        assert!(s.dots_for(other_hider, &players).is_empty());
    }

    #[test]
    fn everyone_is_told_who_is_gold() {
        let (mut s, players) = coin_phase(2, 0);
        let (_, hider) = sides(&s);
        assert!(s.golden_ids(&players).is_empty());
        s.go_golden(hider);
        assert_eq!(s.golden_ids(&players), vec![players[&hider].id]);
    }

    #[test]
    fn the_round_shows_your_side_and_if_golden_is_ready() {
        let (mut s, players) = coin_phase(2, 0);
        let (hunter, hider) = sides(&s);
        assert_eq!(s.round_for(hunter, &players).side, SMOKE_SIDE_HUNTER);
        assert!(!s.round_for(hunter, &players).golden_ready, "hunters don't go golden");
        assert_eq!(s.round_for(hider, &players).side, SMOKE_SIDE_HIDER);
        assert!(s.round_for(hider, &players).golden_ready);
        s.go_golden(hider);
        assert!(!s.round_for(hider, &players).golden_ready, "used it");
        assert_eq!(s.round_for(hider, &players).coins_left, 20);
    }

    #[test]
    fn scores_are_humans_only_best_first() {
        let (mut s, players) = coin_phase(2, 3);
        let (hunter, hider) = sides(&s);
        s.points.insert(hider, 40);
        s.points.insert(hunter, 90);
        let rows = s.scores(&players);
        assert_eq!(rows.len(), 2, "no bots, no decoys");
        assert_eq!(rows[0].id, players[&hunter].id);
        assert!(players.keys().any(|p| is_decoy(*p)));
        assert_eq!(s.leader(&players).map(|(id, _)| id), Some(players[&hunter].id));
    }
}