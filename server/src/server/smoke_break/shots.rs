/*
    when a hunter shoots a box, what happens depends on whos inside:
    a human hider: found, they lose up to 50 and hunt for the rest of the round, the hunter gets 50
    a bot hider: found, it drops where its box was, the hunter gets 50 (bots never score or lose anything)
    a decoy: it pops, the hunter loses 10 and their multiplier drops for the rest of the round
*/

use std::collections::HashMap;

use platform::Instant;

use super::round_start::{ is_decoy, is_smoke_entity, HUNTER_WEAPON };
use super::{
    SmokePhase,
    SmokeState,
    SMOKE_DECOY_MULT_STEP,
    SMOKE_MULT_MIN,
    SMOKE_POINTS_DECOY,
    SMOKE_POINTS_FIND,
    SMOKE_POINTS_FOUND,
};
use super::ability::{ RED_HANDED_FIND, RED_HANDED_LOSS };
use crate::net::PeerId;
use crate::player::Player;
use crate::protocols::SMOKE_SIDE_HUNTER;
use crate::server::boxes::Boxes;

/// what a shot did, so the right people get told
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shot {
    // a hider or a frozen hunter shooting, or the box was already gone, the box stays
    NotAllowed,
    Found(PeerId),
    Decoy,
    // an empty box (smoke break never has any, but a box can be anything)
    Empty,
}

impl SmokeState {
    /// a hunter shot box `id`
    pub(in crate::server) fn box_shot(
        &mut self,
        shooter: PeerId,
        id: u16,
        players: &mut HashMap<PeerId, Player>,
        boxes: &mut Boxes
    ) -> Shot {
        // only hunters, and only once they're out and before the round is over
        let live = matches!(self.phase, SmokePhase::Released | SmokePhase::Coins);
        if !live || !self.hunters.contains(&shooter) {
            return Shot::NotAllowed;
        }
        let inside = boxes.carrier_of(id);
        if !boxes.break_box(id) {
            return Shot::NotAllowed;
        }
        let shot = match inside {
            Some(found) if self.hiders.remove(&found) => {
                // gold right now or just was, caught red handed
                let (find, lose) = if self.red_handed(found) {
                    (RED_HANDED_FIND, RED_HANDED_LOSS)
                } else {
                    (SMOKE_POINTS_FIND, SMOKE_POINTS_FOUND)
                };
                self.goldens.remove(&found);
                self.add_points(shooter, find);
                // bots never lose anything, add_points ignores them
                self.add_points(found, lose);
                if is_smoke_entity(found) {
                    // a bot hider is out of the round, it drops where its box was
                    self.walkers.retain(|w| w.peer != found);
                    if let Some(p) = players.get_mut(&found) {
                        p.is_alive = false;
                        p.health = 0;
                        p.death_time = Some(Instant::now());
                    }
                } else {
                    // found humans hunt for the rest of the round
                    self.hunters.insert(found);
                    if let Some(p) = players.get_mut(&found) {
                        p.current_weapon = HUNTER_WEAPON.to_string();
                    }
                }
                Shot::Found(found)
            }
            Some(decoy) if is_decoy(decoy) => {
                self.add_points(shooter, SMOKE_POINTS_DECOY);
                let mult = self.multipliers.entry(shooter).or_insert(1.0);
                *mult = (*mult - SMOKE_DECOY_MULT_STEP).max(SMOKE_MULT_MIN);
                // it pops for good
                self.walkers.retain(|w| w.peer != decoy);
                players.remove(&decoy);
                Shot::Decoy
            }
            _ => Shot::Empty,
        };
        // that was the last one hiding
        if self.hiders.is_empty() {
            self.end_round(SMOKE_SIDE_HUNTER);
        }
        shot
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::WorldId;
    use crate::server::smoke_break::{ SMOKE_BLIND_TIME, SMOKE_POINTS_MVP, SMOKE_POINTS_WIN };

    fn humans(n: u32) -> HashMap<PeerId, Player> {
        (1..=n)
            .map(|i| {
                let mut p = Player::new(i, format!("p{i}"));
                p.world = WorldId::Maze;
                (PeerId(i), p)
            })
            .collect()
    }

    // a round with `humans` people and `bots` bot hiders, ticked until hunters are out
    fn released(humans_n: u32, bots: usize) -> (SmokeState, HashMap<PeerId, Player>, Boxes) {
        let mut players = humans(humans_n);
        let mut boxes = Boxes::new();
        let mut s = SmokeState::new(bots);
        s.start_round(&mut players, &mut boxes);
        s.tick(SMOKE_BLIND_TIME);
        assert_eq!(s.phase, SmokePhase::Released);
        (s, players, boxes)
    }

    // the box `peer` is in (ids count up from 1, a round makes about 40)
    fn box_of(boxes: &Boxes, peer: PeerId) -> u16 {
        (1..500).find(|id| boxes.carrier_of(*id) == Some(peer)).expect("not in a box")
    }

    fn hunter(s: &SmokeState) -> PeerId {
        *s.hunters.iter().next().unwrap()
    }

    #[test]
    fn finding_a_human_they_lose_50_and_hunt() {
        let (mut s, mut players, mut boxes) = released(3, 0);
        let shooter = hunter(&s);
        let found = *s.hiders.iter().next().unwrap();
        s.points.insert(found, 70);
        let id = box_of(&boxes, found);
        assert_eq!(s.box_shot(shooter, id, &mut players, &mut boxes), Shot::Found(found));
        assert_eq!(s.points[&found], 20, "70 - 50");
        assert_eq!(s.points[&shooter], SMOKE_POINTS_FIND);
        assert!(s.hunters.contains(&found) && !s.hiders.contains(&found));
        assert_eq!(players[&found].current_weapon, HUNTER_WEAPON);
        assert!(!boxes.is_carrying(found), "their box is gone");
        // the other one is still hiding so the round goes on
        assert_eq!(s.phase, SmokePhase::Released);
    }

    #[test]
    fn a_found_hider_never_goes_under_0() {
        let (mut s, mut players, mut boxes) = released(3, 0);
        let found = *s.hiders.iter().next().unwrap();
        s.points.insert(found, 30);
        let id = box_of(&boxes, found);
        s.box_shot(hunter(&s), id, &mut players, &mut boxes);
        assert_eq!(s.points[&found], 0);
    }

    #[test]
    fn finding_a_bot_drops_it_and_it_never_hunts() {
        let (mut s, mut players, mut boxes) = released(1, 2);
        let shooter = hunter(&s);
        let bot = *s.hiders.iter().next().unwrap();
        let id = box_of(&boxes, bot);
        assert_eq!(s.box_shot(shooter, id, &mut players, &mut boxes), Shot::Found(bot));
        assert!(!players[&bot].is_alive, "it drops where its box was");
        assert!(!s.hunters.contains(&bot), "bots never hunt");
        assert_eq!(s.points.get(&bot), None, "bots never score or lose");
        assert_eq!(s.points[&shooter], SMOKE_POINTS_FIND);
    }

    #[test]
    fn a_decoy_pops_costs_10_and_drops_the_multiplier() {
        let (mut s, mut players, mut boxes) = released(2, 0);
        let shooter = hunter(&s);
        s.points.insert(shooter, 40);
        let decoy = *players
            .keys()
            .find(|p| is_decoy(**p))
            .unwrap();
        let id = box_of(&boxes, decoy);
        assert_eq!(s.box_shot(shooter, id, &mut players, &mut boxes), Shot::Decoy);
        assert!(!players.contains_key(&decoy), "gone for good");
        assert_eq!(s.points[&shooter], 30);
        assert!((s.multipliers[&shooter] - 0.9).abs() < 1e-5);
    }

    #[test]
    fn the_multiplier_never_drops_under_its_floor() {
        let (mut s, mut players, mut boxes) = released(2, 0);
        let shooter = hunter(&s);
        let decoys: Vec<PeerId> = players
            .keys()
            .filter(|p| is_decoy(**p))
            .copied()
            .take(10)
            .collect();
        for d in decoys {
            let id = box_of(&boxes, d);
            s.box_shot(shooter, id, &mut players, &mut boxes);
        }
        assert!((s.multipliers[&shooter] - SMOKE_MULT_MIN).abs() < 1e-5);
    }

    #[test]
    fn hiders_cant_shoot_and_nobody_shoots_while_blind() {
        let mut players = humans(2);
        let mut boxes = Boxes::new();
        let mut s = SmokeState::new(0);
        s.start_round(&mut players, &mut boxes);
        let decoy = *players
            .keys()
            .find(|p| is_decoy(**p))
            .unwrap();
        let id = box_of(&boxes, decoy);
        assert_eq!(s.box_shot(hunter(&s), id, &mut players, &mut boxes), Shot::NotAllowed, "blind");
        s.tick(SMOKE_BLIND_TIME);
        let hider = *s.hiders.iter().next().unwrap();
        assert_eq!(s.box_shot(hider, id, &mut players, &mut boxes), Shot::NotAllowed, "a hider");
        assert!(players.contains_key(&decoy) && boxes.is_carrying(decoy), "the box is still there");
    }

    #[test]
    fn finding_the_last_hider_and_the_hunters_win() {
        let (mut s, mut players, mut boxes) = released(2, 0);
        let shooter = hunter(&s);
        let found = *s.hiders.iter().next().unwrap();
        let id = box_of(&boxes, found);
        s.box_shot(shooter, id, &mut players, &mut boxes);
        assert_eq!(s.phase, SmokePhase::Vote);
        // find, then the win bonus, then mvp (they did the most this round)
        assert_eq!(s.points[&shooter], SMOKE_POINTS_FIND + SMOKE_POINTS_WIN + SMOKE_POINTS_MVP);
        // the converted hider gets no win bonus
        assert_eq!(s.points.get(&found).copied().unwrap_or(0), 0);
    }
}
