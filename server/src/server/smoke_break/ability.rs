/*
    going golden, a human hider's once a round gamble
    for 5 s your box is gold and earns 5 points a second, hunters see you on their minimap the whole time
    and keep a "last seen" dot for 5 s after, get found while golden or in the 10 s after and you're caught red handed
*/

use std::collections::HashMap;

use super::round_start::is_smoke_entity;
use super::{SmokePhase, SmokeState};
use crate::net::PeerId;
use crate::player::Player;

/// how long you're gold
pub const GOLDEN_TIME: f32 = 5.0;
/// points a second while gold (25 over the whole thing)
pub const GOLDEN_RATE: f32 = 5.0;
/// found within this long after you stop being gold still counts as red handed
pub const RED_HANDED_TIME: f32 = 10.0;
/// how long hunters keep the "last seen" dot after you stop being gold
pub const LAST_SEEN_TIME: f32 = 5.0;
/// caught red handed: the hider loses this and the hunter gets that instead of the usual -50 / +50
pub const RED_HANDED_LOSS: i32 = -100;
pub const RED_HANDED_FIND: i32 = 75;
/// no going golden in the last this many seconds of the round, so there's always time to get caught
pub const GOLDEN_CUTOFF: f32 = 30.0;

/// one hider's golden moment, from pressing the key to the end of the red handed window
pub struct Golden {
    /// counts down from GOLDEN_TIME + RED_HANDED_TIME, gold while it's above RED_HANDED_TIME
    left: f32,
    /// fills up between points while gold
    pay_clock: f32,
    /// where they were when the gold wore off, hunters keep a dot here for a bit
    pub seen_at: Option<(f32, f32, f32)>,
}

impl Golden {
    pub fn is_gold(&self) -> bool {
        self.left > RED_HANDED_TIME
    }

    /// hunters still see the "last seen" dot
    pub fn show_last_seen(&self) -> bool {
        !self.is_gold() && self.left > RED_HANDED_TIME - LAST_SEEN_TIME
    }
}

impl SmokeState {
    /// a hider pressed the golden key, false if they can't right now
    pub(in crate::server) fn go_golden(&mut self, peer: PeerId) -> bool {
        if !self.can_go_golden(peer) {
            return false;
        }
        self.golden_used.insert(peer);
        let golden = Golden { left: GOLDEN_TIME + RED_HANDED_TIME, pay_clock: 0.0, seen_at: None };
        self.goldens.insert(peer, golden);
        true
    }

    /// gold hiders earn, every golden timer runs down
    pub(in crate::server) fn tick_ability(&mut self, dt: f32, players: &HashMap<PeerId, Player>) {
        let mut pay: Vec<PeerId> = Vec::new();
        for (peer, g) in self.goldens.iter_mut() {
            let was_gold = g.is_gold();
            if was_gold {
                g.pay_clock += dt;
                while g.pay_clock >= 1.0 / GOLDEN_RATE {
                    g.pay_clock -= 1.0 / GOLDEN_RATE;
                    pay.push(*peer);
                }
            }
            g.left -= dt;
            // the gold just wore off, hunters' dot stays where they were
            if was_gold && !g.is_gold() {
                g.seen_at = players.get(peer).map(|p| p.position);
            }
        }
        self.goldens.retain(|_, g| g.left > 0.0);
        for peer in pay {
            self.add_points(peer, 1);
        }
    }

    /// whether pressing the key would work right now, the hud shows it and go_golden checks it
    pub(super) fn can_go_golden(&self, peer: PeerId) -> bool {
        let live = matches!(self.phase, SmokePhase::Released | SmokePhase::Coins);
        // too close to the end, nobody would have time to catch them
        let too_late = self.phase == SmokePhase::Coins && self.phase_timer <= GOLDEN_CUTOFF;
        // humans still hiding, once a round
        live && !too_late && !is_smoke_entity(peer) && self.hiders.contains(&peer) && !self.golden_used.contains(&peer)
    }

    /// gold right now, or was in the last RED_HANDED_TIME seconds
    pub(super) fn red_handed(&self, peer: PeerId) -> bool {
        self.goldens.contains_key(&peer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::WorldId;
    use crate::server::boxes::Boxes;
    use crate::server::smoke_break::{SMOKE_BLIND_TIME, SMOKE_RELEASED_TIME, SMOKE_ROUND_TIME};

    // 2 humans, one hunts one hides, ticked until hunters are out
    fn released() -> (SmokeState, HashMap<PeerId, Player>, Boxes, PeerId, PeerId) {
        let mut players: HashMap<PeerId, Player> = (1..=2)
            .map(|i| {
                let mut p = Player::new(i, format!("p{i}"));
                p.world = WorldId::Maze;
                (PeerId(i), p)
            })
            .collect();
        let mut boxes = Boxes::new();
        let mut s = SmokeState::new(0);
        s.start_round(&mut players, &mut boxes);
        s.tick(SMOKE_BLIND_TIME);
        let hunter = *s.hunters.iter().next().unwrap();
        let hider = *s.hiders.iter().next().unwrap();
        (s, players, boxes, hunter, hider)
    }

    fn box_of(boxes: &Boxes, peer: PeerId) -> u16 {
        (1..500).find(|id| boxes.carrier_of(*id) == Some(peer)).expect("not in a box")
    }

    // run the golden timers at 30 ticks a second
    fn run(s: &mut SmokeState, players: &HashMap<PeerId, Player>, seconds: f32) {
        for _ in 0..(seconds * 30.0).round() as u32 {
            s.tick_ability(1.0 / 30.0, players);
        }
    }

    #[test]
    fn going_golden_pays_25_over_5_seconds() {
        let (mut s, players, _, _, hider) = released();
        let before = s.points.get(&hider).copied().unwrap_or(0);
        assert!(s.go_golden(hider));
        run(&mut s, &players, GOLDEN_TIME + 1.0);
        let earned = s.points[&hider] - before;
        assert!((24..=25).contains(&earned), "earned {earned}");
        assert!(!s.goldens[&hider].is_gold(), "it wears off");
    }

    #[test]
    fn once_a_round() {
        let (mut s, mut players, mut boxes, _, hider) = released();
        assert!(s.go_golden(hider));
        run(&mut s, &players, GOLDEN_TIME + RED_HANDED_TIME + 1.0);
        assert!(!s.go_golden(hider), "already used it this round");
        // a new round, it's back
        s.start_round(&mut players, &mut boxes);
        s.tick(SMOKE_BLIND_TIME);
        let hider = *s.hiders.iter().find(|p| !is_smoke_entity(**p)).unwrap();
        assert!(s.go_golden(hider));
    }

    #[test]
    fn hunters_bots_and_blind_hiders_cant() {
        let mut players: HashMap<PeerId, Player> = HashMap::new();
        let mut me = Player::new(1, "p".into());
        me.world = WorldId::Maze;
        players.insert(PeerId(1), me);
        let mut s = SmokeState::new(2);
        s.start_round(&mut players, &mut Boxes::new());
        let bot = *s.hiders.iter().next().unwrap();
        assert!(!s.go_golden(bot), "blind, and a bot");
        s.tick(SMOKE_BLIND_TIME);
        assert!(!s.go_golden(bot), "bots never go golden");
        assert!(!s.go_golden(PeerId(1)), "the hunter");
    }

    #[test]
    fn no_going_golden_in_the_last_30_seconds() {
        let (mut s, _, _, _, hider) = released();
        s.tick(SMOKE_RELEASED_TIME);
        // run the coin phase down to the cutoff
        s.tick(SMOKE_ROUND_TIME - GOLDEN_CUTOFF);
        assert!(s.phase_timer <= GOLDEN_CUTOFF);
        assert!(!s.go_golden(hider), "too late");
        assert!(!s.golden_used.contains(&hider), "trying too late doesn't use up their turn");
    }

    #[test]
    fn found_while_golden_is_red_handed() {
        let (mut s, mut players, mut boxes, hunter, hider) = released();
        s.points.insert(hider, 150);
        s.go_golden(hider);
        run(&mut s, &players, 1.0);
        let hider_before = s.points[&hider];
        let id = box_of(&boxes, hider);
        s.box_shot(hunter, id, &mut players, &mut boxes);
        assert_eq!(s.points[&hider], hider_before + RED_HANDED_LOSS);
        // +75, then it was the last hider so +30 win and +10 mvp
        assert_eq!(s.points[&hunter], RED_HANDED_FIND + 30 + 10);
    }

    #[test]
    fn still_red_handed_for_10_seconds_after_then_normal() {
        let (mut s, players, _, _, hider) = released();
        s.go_golden(hider);
        run(&mut s, &players, GOLDEN_TIME + RED_HANDED_TIME - 1.0);
        assert!(s.red_handed(hider), "still in the window");
        run(&mut s, &players, 2.0);
        assert!(!s.red_handed(hider), "window's over, a find is normal again");
    }

    #[test]
    fn hunters_keep_a_last_seen_spot_after_the_gold_wears_off() {
        let (mut s, players, _, _, hider) = released();
        s.go_golden(hider);
        run(&mut s, &players, GOLDEN_TIME + 0.5);
        let g = &s.goldens[&hider];
        assert_eq!(g.seen_at, Some(players[&hider].position));
        assert!(g.show_last_seen());
        run(&mut s, &players, LAST_SEEN_TIME);
        assert!(!s.goldens[&hider].show_last_seen(), "the dot fades after 5 s");
    }
}