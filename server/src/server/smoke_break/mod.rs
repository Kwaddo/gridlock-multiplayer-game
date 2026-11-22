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
mod spots;
mod walker;
mod round_start;
mod coins;
mod shots;
mod ability;
mod view;
mod alias;

mod hooks;

pub use round_start::{is_decoy, remove_smoke_entities, HUNTER_WEAPON};
pub use alias::{alias_for, masked_info, masked_pose, masked_state};

use walker::Walker;

// what part of the round we're in, numbers match SMOKE_PHASE_* so they go on the wire as they are
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmokePhase {
    Blind = 0, // hunters frozen and blind, hiders spread out
    Released = 1, // hunters out, no coins yet so hiders can practise blending in
    Coins = 2, // coins out
    Results = 3, // round over, scores up for a few seconds, then the next round or the end
}

// how long each phase lasts
pub const SMOKE_BLIND_TIME: f32 = 15.0;
pub const SMOKE_RELEASED_TIME: f32 = 5.0;
pub const SMOKE_ROUND_TIME: f32 = 180.0; // starts when the coins come out
pub const SMOKE_RESULTS_TIME: f32 = 10.0;

// how many rounds a match can be, picked in the lobby
pub const SMOKE_ROUND_CHOICES: [u8; 3] = [3, 5, 10];
pub const SMOKE_DEFAULT_ROUNDS: u8 = 3;

// at most this many hiders for every hunter, 6 players = 5 + 1, 7 = 5 + 2
pub const SMOKE_HIDERS_PER_HUNTER: usize = 5;

// coins are 4 per hider but never under 10, so 2 hiders = 10, 3 = 12, 5 = 20
pub const SMOKE_COINS_BASE: usize = 10;
pub const SMOKE_COINS_PER_HIDER: usize = 4;

// points as of now
pub const SMOKE_POINTS_COIN: i32 = 5;
pub const SMOKE_POINTS_ALIVE: i32 = 1; // every SMOKE_ALIVE_EVERY seconds still hidden
pub const SMOKE_ALIVE_EVERY: f32 = 5.0; // as of now every 5 seconds alive is 1 point, will likely increase time to 10 seconds or more
pub const SMOKE_POINTS_FIND: i32 = 50;
pub const SMOKE_POINTS_FOUND: i32 = -50; // a found hider loses this but never goes under 0
pub const SMOKE_POINTS_DECOY: i32 = -10; // shot a decoy, no hp lost just points
pub const SMOKE_POINTS_WIN: i32 = 30;
pub const SMOKE_POINTS_MVP: i32 = 10;

// every decoy you shoot makes your gains worse for the rest of the round, so shooting at 0 points still costs something
pub const SMOKE_DECOY_MULT_STEP: f32 = 0.1; // 1.0 -> 0.9 -> 0.8 ...
pub const SMOKE_MULT_MIN: f32 = 0.3; // never worse than this, a find is still worth something
// every find wins some of it back and can go past 1, 3 clean finds in a row hits the cap
pub const SMOKE_FIND_MULT_STEP: f32 = 0.2; // the find pays at the old multiplier, then this goes on
pub const SMOKE_MULT_MAX: f32 = 1.5;

// a found hider shooting a decoy locks their gun this long, each miss the next step, capped at the last
pub const SMOKE_GUN_LOCKS: [f32; 5] = [2.0, 3.0, 5.0, 10.0, 20.0];
// this long without a miss (once the lock is over) and they're back to the first step
pub const SMOKE_CLEAN_RESET: f32 = 30.0;

/// a found hider's gun: how far up the lock steps they are, time left locked, time since the lock ended
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GunLock {
    pub step: usize,
    pub locked_for: f32,
    pub clean_for: f32,
}

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

    // a brain for every decoy this round
    walkers: Vec<Walker>,
    // humans the round start moved, their games get told where they are now
    pub(super) respawns: Vec<(PeerId, (f32, f32, f32))>,

    /// where every coin still out is
    pub coins: Vec<(f32, f32, f32)>,
    /// human hiders when the round started, how many coins come out is based on this
    hiders_at_start: usize,
    /// coins grabbed since the last send, everyone hears them
    pub(super) taken: Vec<(f32, f32, f32)>,
    /// how far into the coin phase the golden coin comes out, None once it's out or the round is done with it
    golden_due: Option<f32>,
    /// the golden coin while it's out: where, and seconds left before it goes
    pub golden: Option<((f32, f32, f32), f32)>,
    /// golden coin news since the last send (out, taken, gone)
    pub(super) golden_news: Vec<coins::GoldenNews>,
    /// who went golden this round already, it's once a round
    golden_used: HashSet<PeerId>,
    /// everyone gold right now or still in their red handed window
    pub goldens: HashMap<PeerId, ability::Golden>,
    /// how many rounds this match plays, picked in the lobby
    pub max_rounds: u8,
    /// who won the round once it's over (SMOKE_SIDE_*), SMOKE_NO_WINNER while it plays
    pub winner: u8,
    /// box shots since the last send, for everyone's kill feed
    pub(super) feed: Vec<shots::FeedLine>,
    /// hiders found this round, they hunt but can only lose points now
    pub found: HashSet<PeerId>,
    /// found hiders' guns
    pub gun_locks: HashMap<PeerId, GunLock>,
    /// box aliases handed out this match, the next one counts on from here (alias.rs)
    aliases_used: u32,
    /// when each hunter's last box shot went through, they can't fire faster than the deagle
    last_shot: HashMap<PeerId, platform::Instant>,
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
            walkers: Vec::new(),
            respawns: Vec::new(),
            coins: Vec::new(),
            hiders_at_start: 0,
            taken: Vec::new(),
            golden_due: None,
            golden: None,
            golden_news: Vec::new(),
            golden_used: HashSet::new(),
            goldens: HashMap::new(),
            max_rounds: SMOKE_DEFAULT_ROUNDS,
            winner: SMOKE_NO_WINNER,
            feed: Vec::new(),
            found: HashSet::new(),
            gun_locks: HashMap::new(),
            aliases_used: 0,
            last_shot: HashMap::new(),
        }
    }

    /// gives (or takes) points and says what really changed, after the multiplier and the 0 floor
    fn add_points(&mut self, peer: PeerId, points: i32) -> i32 {
        // found hiders lost their round, they can only lose points from here
        if points > 0 && self.found.contains(&peer) {
            return 0;
        }
        let points = if points > 0 {
            let mult = self.multipliers.get(&peer).copied().unwrap_or(1.0);
            ((points as f32) * mult).round() as i32
        } else {
            points
        };
        let total = self.points.entry(peer).or_insert(0);
        let before = *total;
        *total = (before + points).max(0);
        // the round counts what really changed so the mvp matches the scoreboard
        let changed = *total - before;
        *self.round_points.entry(peer).or_insert(0) += changed;
        changed
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
        self.golden_used.clear();
        self.goldens.clear();
        self.winner = SMOKE_NO_WINNER;
        self.found.clear();
        self.gun_locks.clear();
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
                self.tick_gun_locks(dt);
                if self.phase_timer <= 0.0 {
                    self.place_coins();
                    self.set_phase(SmokePhase::Coins, SMOKE_ROUND_TIME);
                }
            }
            SmokePhase::Coins => {
                self.alive_points(dt);
                self.tick_golden(dt);
                self.tick_gun_locks(dt);
                // every coin taken, or they outlasted the clock
                if self.coins.is_empty() || self.phase_timer <= 0.0 {
                    self.end_round(SMOKE_SIDE_HIDER);
                }
            }
            // the board is up, hooks.rs starts the next round (or ends the match) once it runs out
            SmokePhase::Results => {}
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

    // the round is over, winners get their bonus, the rounds best gets the mvp, then the results board
    fn end_round(&mut self, winner: u8) {
        // mvp from what they did this round, before the side bonus
        // only someone still in the match, and a tie goes to the lowest peer so it's the same every time
        let mvp = self.round_points
            .iter()
            .filter(|(peer, _)| self.hunters.contains(peer) || self.hiders.contains(peer) || self.starting_hunters.contains(peer))
            .max_by(|(a, a_pts), (b, b_pts)| a_pts.cmp(b_pts).then(b.cmp(a)))
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
        self.coins.clear();
        self.golden = None;
        self.golden_due = None;
        self.goldens.clear();
        self.winner = winner;
        self.set_phase(SmokePhase::Results, SMOKE_RESULTS_TIME);
    }
}

/// what happens when the results board runs out
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AfterResults {
    Waiting,
    NextRound,
    MatchOver,
}

impl SmokeState {
    /// once the board's time is up: another round, or that was the last one (or too few are left)
    pub(in crate::server) fn after_results(&self, humans: usize) -> AfterResults {
        if self.phase != SmokePhase::Results || self.phase_timer > 0.0 {
            AfterResults::Waiting
        } else if self.round >= self.max_rounds as u16 || hunter_count(humans, humans) == 0 {
            AfterResults::MatchOver
        } else {
            AfterResults::NextRound
        }
    }
}

// how many player hunt is decided by the number of players and the number of humans, but never more than players - 1, and as of now we only have human hunters cause
// idk how wed do AI ones like ??
pub fn hunter_count(players: usize, humans: usize) -> usize {
    if players < 2 || humans == 0 {
        return 0;
    }
    let want = (players + SMOKE_HIDERS_PER_HUNTER) / (SMOKE_HIDERS_PER_HUNTER + 1);
    want.max(1)
        .min(humans)
        .min(players - 1)
}

pub fn coin_count(hiders: usize) -> usize {
    (hiders * SMOKE_COINS_PER_HIDER).max(SMOKE_COINS_BASE)
}
