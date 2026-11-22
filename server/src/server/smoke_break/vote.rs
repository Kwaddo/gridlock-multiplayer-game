/*
    the vote at the end of every round, stay or leave
    not voting counts as leave so nobody gets dragged into another round while afk,
    whoever stays plays on if there's enough of them (bots count as hiders), otherwise the match is over
*/

use std::collections::HashMap;

use super::round_start::{humans_in_match, is_smoke_entity};
use super::{hunter_count, SmokePhase, SmokeState};
use crate::net::PeerId;
use crate::player::Player;

/// how the vote's going
#[derive(Clone, Debug, PartialEq)]
pub enum VoteEnd {
    // still time left and not everyone has voted
    Waiting,
    // another round, these humans go back to the lobby first
    NextRound { leavers: Vec<PeerId> },
    // not enough people stayed for a round, the most points wins
    MatchOver,
}

impl SmokeState {
    /// a human voted, it only counts during the vote and they can change their mind till it ends
    pub(in crate::server) fn vote(&mut self, peer: PeerId, stay: bool) {
        if self.phase == SmokePhase::Vote && !is_smoke_entity(peer) {
            self.votes.insert(peer, stay);
        }
    }

    /// once time's up or everyone has voted: who leaves, and whether there's another round
    pub(in crate::server) fn check_vote(&self, players: &HashMap<PeerId, Player>) -> VoteEnd {
        if self.phase != SmokePhase::Vote {
            return VoteEnd::Waiting;
        }
        let humans = humans_in_match(players);
        let everyone_voted = !humans.is_empty() && humans.iter().all(|h| self.votes.contains_key(h));
        if self.phase_timer > 0.0 && !everyone_voted {
            return VoteEnd::Waiting;
        }
        // stay only if they said stay, no vote is a leave
        let (stay, leave): (Vec<PeerId>, Vec<PeerId>) =
            humans.into_iter().partition(|h| self.votes.get(h) == Some(&true));
        if hunter_count(stay.len() + self.bot_hiders.len(), stay.len()) == 0 {
            return VoteEnd::MatchOver;
        }
        VoteEnd::NextRound { leavers: leave }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::WorldId;
    use crate::server::boxes::Boxes;
    use crate::server::smoke_break::{SMOKE_BLIND_TIME, SMOKE_RELEASED_TIME, SMOKE_ROUND_TIME, SMOKE_VOTE_TIME};

    // a round with `humans` people and `bots` bot hiders, played out to the vote
    fn at_the_vote(humans: u32, bots: usize) -> (SmokeState, HashMap<PeerId, Player>) {
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
        s.tick(SMOKE_ROUND_TIME);
        assert_eq!(s.phase, SmokePhase::Vote);
        (s, players)
    }

    #[test]
    fn votes_only_count_during_the_vote_and_bots_dont_vote() {
        let (mut s, players) = at_the_vote(2, 1);
        let bot = *players.keys().find(|p| is_smoke_entity(**p)).unwrap();
        s.vote(bot, true);
        assert!(s.votes.is_empty(), "bots don't vote");
        s.vote(PeerId(1), true);
        assert_eq!(s.votes.get(&PeerId(1)), Some(&true));
        // change their mind before it ends
        s.vote(PeerId(1), false);
        assert_eq!(s.votes.get(&PeerId(1)), Some(&false));
    }

    #[test]
    fn it_waits_until_time_is_up_or_everyone_voted() {
        let (mut s, players) = at_the_vote(3, 0);
        s.vote(PeerId(1), true);
        assert_eq!(s.check_vote(&players), VoteEnd::Waiting);
        s.vote(PeerId(2), true);
        s.vote(PeerId(3), true);
        // everyone voted, no waiting out the clock
        assert_eq!(s.check_vote(&players), VoteEnd::NextRound { leavers: vec![] });
    }

    #[test]
    fn not_voting_counts_as_leave() {
        let (mut s, players) = at_the_vote(3, 0);
        s.vote(PeerId(1), true);
        s.vote(PeerId(2), true);
        s.tick(SMOKE_VOTE_TIME);
        assert_eq!(s.check_vote(&players), VoteEnd::NextRound { leavers: vec![PeerId(3)] });
    }

    #[test]
    fn not_enough_stay_and_the_match_is_over() {
        let (mut s, players) = at_the_vote(2, 0);
        s.vote(PeerId(1), true);
        s.vote(PeerId(2), false);
        // one human alone with no bots can't play a round
        assert_eq!(s.check_vote(&players), VoteEnd::MatchOver);
    }

    #[test]
    fn staying_alone_with_bot_hiders_keeps_going() {
        let (mut s, players) = at_the_vote(2, 2);
        s.vote(PeerId(1), true);
        s.vote(PeerId(2), false);
        assert_eq!(s.check_vote(&players), VoteEnd::NextRound { leavers: vec![PeerId(2)] });
    }

    #[test]
    fn a_new_round_forgets_the_votes() {
        let (mut s, mut players) = at_the_vote(2, 0);
        s.vote(PeerId(1), true);
        s.vote(PeerId(2), true);
        s.start_round(&mut players, &mut Boxes::new());
        assert!(s.votes.is_empty());
        assert_eq!(s.check_vote(&players), VoteEnd::Waiting, "not the vote any more");
    }
}