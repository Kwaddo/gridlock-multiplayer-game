/*
    COINS. only hiders ever know where they are and they come out when the coin phase starts, a human hider walking over one grabs it,
    a coin going missing always means a hider took it (or the golden one expired)
*/

use std::collections::HashMap;

use super::spots::random_spot;
use super::{coin_count, SmokeState, SMOKE_POINTS_COIN, SMOKE_ROUND_TIME};
use crate::net::PeerId;
use crate::player::Player;
use rand::Rng;


/// how close a hider has to get to grab a coin
pub const COIN_PICKUP: f32 = 0.9;
/// coins at least this far apart so they don't clump
const COIN_SPACING: f32 = 3.0;


/// the golden coin, once a round, worth as much as a find
pub const GOLDEN_POINTS: i32 = 50;
/// how long it stays out before it goes, seconds
pub const GOLDEN_LIFE: f32 = 30.0;
/// it never comes out in the first or last this many seconds of the coin phase
const GOLDEN_QUIET: f32 = 30.0;

/// what happened to the golden coin, so the right people get told
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GoldenNews {
    // hiders get told where, hunters only that it's out
    Out,
    Taken(PeerId),
    // nobody took it in time
    Gone,
}

impl SmokeState {
    /// the coin phase starts: 4 per human hider (never under 10) at random spots
    pub(super) fn place_coins(&mut self) {
        let mut rng = rand::thread_rng();
        let want = coin_count(self.hiders_at_start);
        self.coins.clear();
        // a spot too close to another coin gets rolled again, the tries cap stops it looping forever
        let mut tries = 0;
        while self.coins.len() < want && tries < want * 50 {
            tries += 1;
            let (x, z) = random_spot(&mut rng);
            if self.coins.iter().all(|c| flat(*c, (x, z)) >= COIN_SPACING) {
                self.coins.push((x, 0.0, z));
            }
        }

        // the golden coin's moment this round, somewhere in the middle of the coin phase
        self.golden_due = Some(rng.gen_range(GOLDEN_QUIET..SMOKE_ROUND_TIME - GOLDEN_QUIET));
        self.golden = None;
    }

    
    /// the golden coin comes out when it's due and goes after GOLDEN_LIFE if nobody takes it
    pub(super) fn tick_golden(&mut self, dt: f32) {
        let into_phase = SMOKE_ROUND_TIME - self.phase_timer;
        if self.golden_due.is_some_and(|due| into_phase >= due) {
            self.golden_due = None;
            let (x, z) = random_spot(&mut rand::thread_rng());
            self.golden = Some(((x, 0.0, z), GOLDEN_LIFE));
            self.golden_news.push(GoldenNews::Out);
        }
        let expired = match self.golden.as_mut() {
            Some((_, left)) => {
                *left -= dt;
                *left <= 0.0
            }
            None => false,
        };
        if expired {
            self.golden = None;
            self.golden_news.push(GoldenNews::Gone);
        }
    }

    /// human hiders standing on a coin grab it (+10 times their multiplier)
    pub(in crate::server) fn pick_up_coins(&mut self, players: &HashMap<PeerId, Player>) {
        let humans: Vec<PeerId> = self.hiders.iter().copied().collect();
        for peer in humans {
            let Some(p) = players.get(&peer) else { continue };
            let here = (p.position.0, p.position.2);
            if let Some(i) = self.coins.iter().position(|c| flat(*c, here) <= COIN_PICKUP) {
                let coin = self.coins.remove(i);
                // remembered so everyone hears it once sounds exist
                self.taken.push(coin);
                self.add_points(peer, SMOKE_POINTS_COIN);
            }
            // the golden coin
            if let Some((at, _)) = self.golden {
                if flat(at, here) <= COIN_PICKUP {
                    self.golden = None;
                    self.taken.push(at);
                    self.golden_news.push(GoldenNews::Taken(peer));
                    self.add_points(peer, GOLDEN_POINTS);
                }
            }
        }
    }
}

/// flat distance from a coin to a spot, height doesn't matter
fn flat(coin: (f32, f32, f32), at: (f32, f32)) -> f32 {
    ((coin.0 - at.0).powi(2) + (coin.2 - at.1).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::WorldId;
    use crate::server::boxes::Boxes;
    use crate::server::smoke_break::spots::walkable;
    use crate::server::smoke_break::{SmokePhase, SMOKE_BLIND_TIME, SMOKE_RELEASED_TIME};

    // a round with `humans` people, ticked into the coin phase
    fn coin_phase(humans: u32) -> (SmokeState, HashMap<PeerId, Player>) {
        let mut players: HashMap<PeerId, Player> = (1..=humans)
            .map(|i| {
                let mut p = Player::new(i, format!("p{i}"));
                p.world = WorldId::Maze;
                (PeerId(i), p)
            })
            .collect();
        let mut s = SmokeState::new();
        s.start_round(&mut players, &mut Boxes::new());
        s.tick(SMOKE_BLIND_TIME);
        s.tick(SMOKE_RELEASED_TIME);
        assert_eq!(s.phase, SmokePhase::Coins);
        (s, players)
    }

    fn human_hider(s: &SmokeState) -> PeerId {
        *s.hiders.iter().next().unwrap()
    }

    #[test]
    fn coins_come_out_spread_apart_where_a_box_fits() {
        let (s, _) = coin_phase(2);
        assert_eq!(s.coins.len(), crate::server::smoke_break::SMOKE_COINS_BASE);
        for (i, a) in s.coins.iter().enumerate() {
            assert!(walkable(a.0, a.2), "coin in a wall at {a:?}");
            for b in &s.coins[i + 1..] {
                assert!(flat(*a, (b.0, b.2)) >= COIN_SPACING, "coins too close");
            }
        }
    }

    #[test]
    fn walking_over_a_coin_grabs_it() {
        let (mut s, mut players) = coin_phase(2);
        let hider = human_hider(&s);
        let coin = s.coins[0];
        // they already earned an alive point while released
        let before = s.points.get(&hider).copied().unwrap_or(0);
        players.get_mut(&hider).unwrap().position = coin;
        s.pick_up_coins(&players);
        assert_eq!(s.coins.len(), crate::server::smoke_break::SMOKE_COINS_BASE - 1);
        assert_eq!(s.taken, vec![coin], "the grab is remembered for the sound");
        assert_eq!(s.points[&hider], before + SMOKE_POINTS_COIN);
    }

    #[test]
    fn hunters_cant_take_coins() {
        let (mut s, mut players) = coin_phase(2);
        let hunter = *s.hunters.iter().next().unwrap();
        // tuck the hider away so they can't grab anything themselves
        let hider = human_hider(&s);
        players.get_mut(&hider).unwrap().position = (0.0, 0.0, 0.0);
        let coin = s.coins[0];
        players.get_mut(&hunter).unwrap().position = coin;
        s.pick_up_coins(&players);
        // the hider might be standing on some other coin, so check the one under the hunter
        assert!(s.coins.contains(&coin), "a hunter can't take a coin");
    }

    #[test]
    fn every_coin_taken_and_the_hiders_win() {
        let (mut s, mut players) = coin_phase(2);
        let hider = human_hider(&s);
        for coin in s.coins.clone() {
            players.get_mut(&hider).unwrap().position = coin;
            s.pick_up_coins(&players);
        }
        assert!(s.coins.is_empty());
        s.tick(0.0);
        assert_eq!(s.phase, SmokePhase::Results);
    }

    #[test]
    fn more_hiders_more_coins() {
        // 8 humans: 2 hunt, 6 hide = 24 coins
        let (s, _) = coin_phase(8);
        assert_eq!(s.coins.len(), 24);
    }

    // force the golden coin out now
    fn golden_now(s: &mut SmokeState) -> (f32, f32, f32) {
        s.golden_due = Some(0.0);
        s.tick(0.0);
        s.golden.expect("golden coin didn't come out").0
    }

    #[test]
    fn golden_coin_comes_out_once_in_the_middle_of_the_coin_phase() {
        for _ in 0..20 {
            let (mut s, _) = coin_phase(2);
            let mut out_at = Vec::new();
            for second in 0..SMOKE_ROUND_TIME as u32 {
                s.tick(1.0);
                for news in s.golden_news.drain(..) {
                    if news == GoldenNews::Out {
                        out_at.push(second + 1);
                    }
                }
            }
            assert_eq!(out_at.len(), 1, "came out {} times", out_at.len());
            let t = out_at[0] as f32;
            assert!(t >= GOLDEN_QUIET && t <= SMOKE_ROUND_TIME - GOLDEN_QUIET + 1.0, "came out at {t}s");
        }
    }

    #[test]
    fn golden_coin_goes_after_30s_if_nobody_takes_it() {
        let (mut s, _) = coin_phase(2);
        golden_now(&mut s);
        s.tick(GOLDEN_LIFE - 1.0);
        assert!(s.golden.is_some());
        s.tick(1.0);
        assert!(s.golden.is_none());
        assert_eq!(s.golden_news.last(), Some(&GoldenNews::Gone));
    }

    #[test]
    fn grabbing_the_golden_coin_is_worth_a_find() {
        let (mut s, mut players) = coin_phase(2);
        let hider = human_hider(&s);
        let at = golden_now(&mut s);
        // a normal coin can land right next to it, keep them out of the way so we only count the golden one
        s.coins.clear();
        let before = s.points.get(&hider).copied().unwrap_or(0);
        players.get_mut(&hider).unwrap().position = at;
        s.pick_up_coins(&players);
        assert_eq!(s.points[&hider], before + GOLDEN_POINTS);
        assert!(s.golden.is_none());
        assert_eq!(s.golden_news.last(), Some(&GoldenNews::Taken(hider)));
    }

    #[test]
    fn golden_coin_doesnt_count_toward_all_coins_taken() {
        let (mut s, mut players) = coin_phase(2);
        let hider = human_hider(&s);
        golden_now(&mut s);
        // tuck it in a wall so the walk below can't grab it by accident
        s.golden = Some(((0.0, 0.0, 0.0), 999.0));
        for coin in s.coins.clone() {
            players.get_mut(&hider).unwrap().position = coin;
            s.pick_up_coins(&players);
        }
        // every normal coin gone, the golden one still out, hiders still win
        s.tick(0.0);
        assert_eq!(s.phase, SmokePhase::Results);
        assert!(s.golden.is_none(), "it goes when the round ends");
    }
}