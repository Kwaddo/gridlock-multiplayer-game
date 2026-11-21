/*
    Smoke Break, the server's side of the round
    hiders are boxes in a crowd of decoy boxes, hunters have to spot the humans
    built the same way as snd.rs, phases on timers and everything lives here on the server
*/

use std::collections::{ HashMap, HashSet };

use crate::protocols::*;
use crate::net::PeerId;

#[cfg(test)]
mod tests;

// what part of the round we're in, numbers match SMOKE_PHASE_* so they go on the wire as they are
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmokePhase {
    Blind = 0, // hunters frozen and blind, hiders spread out
    Released = 1, // hunters out, no coins yet so hiders can practise blending in
    Coins = 2, // coins out
    Vote = 3, // round over, scores up, stay or leave
}

// how long each phase lasts
pub const SMOKE_BLIND_TIME: f32 = 15.0;
pub const SMOKE_RELEASED_TIME: f32 = 5.0;
pub const SMOKE_ROUND_TIME: f32 = 180.0; // starts when the coins come out
pub const SMOKE_VOTE_TIME: f32 = 15.0;

// at most this many hiders for every hunter, 6 players = 5 + 1, 7 = 5 + 2
pub const SMOKE_HIDERS_PER_HUNTER: usize = 5;

// coins are 4 per hider but never under 20, so 5 hiders = 20, 6 = 24, 10 = 40
pub const SMOKE_COINS_BASE: usize = 20;
pub const SMOKE_COINS_PER_HIDER: usize = 4;

// points as of now
pub const SMOKE_POINTS_COIN: i32 = 10;
pub const SMOKE_POINTS_ALIVE: i32 = 1; // every SMOKE_ALIVE_EVERY seconds still hidden
pub const SMOKE_ALIVE_EVERY: f32 = 5.0; // as of now every 5 seconds alive is 1 point, will likely increase time to 10 seconds or more
pub const SMOKE_POINTS_FIND: i32 = 50;
pub const SMOKE_POINTS_DECOY: i32 = -10; // shot a decoy, no hp lost just points
pub const SMOKE_POINTS_WIN: i32 = 30;
pub const SMOKE_POINTS_MVP: i32 = 10;

// every decoy you shoot makes your gains worse for the rest of the round, so shooting at 0 points still costs something
pub const SMOKE_DECOY_MULT_STEP: f32 = 0.1; // 1.0 -> 0.9 -> 0.8 ...
pub const SMOKE_MULT_MIN: f32 = 0.3; // never worse than this, a find is still worth something


// everything the server remembers while a match plays out
pub struct SmokeState {
    pub round: u16,
    pub phase: SmokePhase,
    pub phase_timer: f32, // seconds left in this phase
    pub hunters: HashSet<PeerId>,
    // whoever started the round hunting gets the win bonus, found hiders don't cause they lost L gg get good
    starting_hunters: HashSet<PeerId>,
    // still hidden goats, if they get found they go into the hunters set
    pub hiders: HashSet<PeerId>,
    // the whole match, kept across rounds
    pub points: HashMap<PeerId, i32>,
    // this round only, for the mvp
    round_points: HashMap<PeerId, i32>,
    // this round only, every decoy shot lowers it, gains get multiplied by it
    multipliers: HashMap<PeerId, f32>,
    // counts up to SMOKE_ALIVE_EVERY then pays every hider
    alive_clock: f32,
}

impl SmokeState {
    pub fn new() -> Self {
        SmokeState {
            round: 0,
            phase: SmokePhase::Blind,
            phase_timer: SMOKE_BLIND_TIME,
            hunters: HashSet::new(),
            starting_hunters: HashSet::new(),
            hiders: HashSet::new(),
            points: HashMap::new(),
            round_points: HashMap::new(),
            multipliers: HashMap::new(),
            alive_clock: 0.0,
        }
    }

    fn add_points(&mut self, peer: PeerId, points: i32) {
        let points = if points > 0 {
            let mult = self.multipliers.get(&peer).copied().unwrap_or(1.0); 
            (points as f32 * mult).round() as i32 
        } else {
            points
        };
        let total = self.points.entry(peer).or_insert(0);
        let before = *total;
        *total = (before + points).max(0); 
        // the round counts what really changed so the mvp matches the scoreboard
        *self.round_points.entry(peer).or_insert(0) += *total - before;
    }

    fn set_phase(&mut self, phase: SmokePhase, time: f32) {
        self.phase = phase;
        self.phase_timer = time;
    }


    fn new_round(&mut self) {
        self.round += 1;
        self.set_phase(SmokePhase::Blind, SMOKE_BLIND_TIME);
        self.round_points.clear();
        self.multipliers.clear();
        self.alive_clock = 0.0;
    }

    // one server tick of the clock that moves the round on when a phase runs out
    pub(in crate::server) fn tick(&mut self, dt: f32) {
        self.phase_timer -= dt;
        match self.phase {
            SmokePhase::Blind => {
                if self.phase_timer <= 0.0 {
                    self.set_phase(SmokePhase::Released, SMOKE_RELEASED_TIME);
                }
            }
            SmokePhase::Released => {
                self.alive_points(dt);
                if self.phase_timer <= 0.0 {
                    self.set_phase(SmokePhase::Coins, SMOKE_ROUND_TIME);
                }
            }
            SmokePhase::Coins => {
                self.alive_points(dt);
                // they outlasted the clock
                if self.phase_timer <= 0.0 {
                    self.end_round(SMOKE_SIDE_HIDER);
                }
            }
            // what happens after the vote comes with the vote piece
            SmokePhase::Vote => {}
        }
    }

    // every SMOKE_ALIVE_EVERY seconds still hidden pays every hider
    fn alive_points(&mut self, dt: f32) {
        self.alive_clock += dt;
        while self.alive_clock >= SMOKE_ALIVE_EVERY {
            self.alive_clock -= SMOKE_ALIVE_EVERY;
            let hidden: Vec<PeerId> = self.hiders.iter().copied().collect();
            for peer in hidden {
                self.add_points(peer, SMOKE_POINTS_ALIVE);
            }
        }
    }

    // the round is over, winners get their bonus, the rounds best gets the mvp, then everyone votes if they want to continue or no
    fn end_round(&mut self, winner: u8) {
        // mvp from what they did this round, before the side bonus
        let mvp = self
            .round_points
            .iter()
            .max_by_key(|(_, pts)| **pts)
            .filter(|(_, pts)| **pts > 0)
            .map(|(peer, _)| *peer);
        let winners: Vec<PeerId> = if winner == SMOKE_SIDE_HIDER {
            self.hiders.iter().copied().collect()
        } else {
            self.starting_hunters.iter().copied().collect()
        };
        for peer in winners {
            self.add_points(peer, SMOKE_POINTS_WIN);
        }
        if let Some(peer) = mvp {
            self.add_points(peer, SMOKE_POINTS_MVP);
        }
        self.set_phase(SmokePhase::Vote, SMOKE_VOTE_TIME);
    }
}

// how many player hunt is decided by the number of players and the number of humans, but never more than players - 1, and as of now we only have human hunters cause 
// idk how wed do AI ones like ?? 
pub fn hunter_count(players: usize, humans: usize) -> usize {
    if players < 2 || humans == 0 {
        return 0;
    }
    let want = (players + SMOKE_HIDERS_PER_HUNTER) / (SMOKE_HIDERS_PER_HUNTER + 1);
    want.max(1).min(humans).min(players - 1)
}

pub fn coin_count(hiders: usize) -> usize {
    (hiders * SMOKE_COINS_PER_HIDER).max(SMOKE_COINS_BASE)
}


