/*
    when a hunter shoots a box, what happens depends on whos inside:
    a hider: found, they lose up to 50 and hunt for the rest of the round, the hunter gets 50
    a decoy: it pops, the hunter loses 10 and their multiplier drops for the rest of the round
*/

use std::collections::HashMap;

use super::round_start::{ is_decoy, HUNTER_WEAPON };
use super::{
    GunLock,
    SmokePhase,
    SmokeState,
    SMOKE_DECOY_MULT_STEP,
    SMOKE_CLEAN_RESET,
    SMOKE_FIND_MULT_STEP,
    SMOKE_GUN_LOCKS,
    SMOKE_MULT_MAX,
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

/// one line for everyone's kill feed: who shot, who got found (None for a decoy), and what it cost
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeedLine {
    pub shooter: PeerId,
    pub found: Option<PeerId>,
    pub red_handed: bool,
    // what the found hider lost, or the shooter for a decoy, after the 0 floor
    pub points: i32,
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
        // a found hider's locked gun can't break anything
        if self.gun_locks.get(&shooter).is_some_and(|l| l.locked_for > 0.0) {
            return Shot::NotAllowed;
        }
        let inside = boxes.carrier_of(id);
        if !boxes.break_box(id) {
            return Shot::NotAllowed;
        }
        let shot = match inside {
            Some(found) if self.hiders.remove(&found) => {
                // gold right now or just was, caught red handed
                let red_handed = self.red_handed(found);
                let (find, lose) = if red_handed {
                    (RED_HANDED_FIND, RED_HANDED_LOSS)
                } else {
                    (SMOKE_POINTS_FIND, SMOKE_POINTS_FOUND)
                };
                self.goldens.remove(&found);
                // paid at the multiplier they had, then the find wins some of it back
                self.add_points(shooter, find);
                let mult = self.multipliers.entry(shooter).or_insert(1.0);
                *mult = (*mult + SMOKE_FIND_MULT_STEP).min(SMOKE_MULT_MAX);
                let lost = self.add_points(found, lose);
                self.feed.push(FeedLine { shooter, found: Some(found), red_handed, points: lost });
                // a found hider finding someone gets nothing for it (add_points says 0), but it clears their lock steps
                if let Some(lock) = self.gun_locks.get_mut(&shooter) {
                    *lock = GunLock::default();
                }
                // found hiders hunt for the rest of the round, but can only lose points now
                self.found.insert(found);
                self.hunters.insert(found);
                if let Some(p) = players.get_mut(&found) {
                    p.current_weapon = HUNTER_WEAPON.to_string();
                    // out of the box, everyone sees who they are now
                    p.smoke_alias = None;
                }
                Shot::Found(found)
            }
            Some(decoy) if is_decoy(decoy) => {
                let lost = self.add_points(shooter, SMOKE_POINTS_DECOY);
                self.feed.push(FeedLine { shooter, found: None, red_handed: false, points: lost });
                let mult = self.multipliers.entry(shooter).or_insert(1.0);
                *mult = (*mult - SMOKE_DECOY_MULT_STEP).max(SMOKE_MULT_MIN);
                // a found hider missing locks their gun, longer every miss
                if self.found.contains(&shooter) {
                    let lock = self.gun_locks.entry(shooter).or_default();
                    lock.locked_for = SMOKE_GUN_LOCKS[lock.step.min(SMOKE_GUN_LOCKS.len() - 1)];
                    lock.step += 1;
                    lock.clean_for = 0.0;
                }
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

    /// locks run down, then 30 s clean after one and the steps start over
    pub(super) fn tick_gun_locks(&mut self, dt: f32) {
        for lock in self.gun_locks.values_mut() {
            if lock.locked_for > 0.0 {
                lock.locked_for = (lock.locked_for - dt).max(0.0);
            } else if lock.step > 0 {
                lock.clean_for += dt;
                if lock.clean_for >= SMOKE_CLEAN_RESET {
                    *lock = GunLock::default();
                }
            }
        }
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

    // a round with `humans_n` people, ticked until hunters are out
    fn released(humans_n: u32) -> (SmokeState, HashMap<PeerId, Player>, Boxes) {
        let mut players = humans(humans_n);
        let mut boxes = Boxes::new();
        let mut s = SmokeState::new();
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
        let (mut s, mut players, mut boxes) = released(3);
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
        let (mut s, mut players, mut boxes) = released(3);
        let found = *s.hiders.iter().next().unwrap();
        s.points.insert(found, 30);
        let id = box_of(&boxes, found);
        s.box_shot(hunter(&s), id, &mut players, &mut boxes);
        assert_eq!(s.points[&found], 0);
    }

    #[test]
    fn a_decoy_pops_costs_10_and_drops_the_multiplier() {
        let (mut s, mut players, mut boxes) = released(2);
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
        let (mut s, mut players, mut boxes) = released(2);
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
    fn a_find_pays_at_the_old_multiplier_then_bumps_it() {
        let (mut s, mut players, mut boxes) = released(4);
        let shooter = hunter(&s);
        s.multipliers.insert(shooter, 0.8);
        let found = *s.hiders.iter().next().unwrap();
        let id = box_of(&boxes, found);
        s.box_shot(shooter, id, &mut players, &mut boxes);
        assert_eq!(s.points[&shooter], 40, "50 at x0.8");
        assert!((s.multipliers[&shooter] - 1.0).abs() < 1e-5, "0.8 + 0.2");
    }

    #[test]
    fn the_multiplier_never_goes_over_its_cap() {
        let (mut s, mut players, mut boxes) = released(6);
        let shooter = hunter(&s);
        s.multipliers.insert(shooter, 1.4);
        let found = *s.hiders.iter().next().unwrap();
        let id = box_of(&boxes, found);
        s.box_shot(shooter, id, &mut players, &mut boxes);
        assert!((s.multipliers[&shooter] - SMOKE_MULT_MAX).abs() < 1e-5);
    }

    #[test]
    fn every_box_shot_goes_in_the_feed() {
        let (mut s, mut players, mut boxes) = released(3);
        let shooter = hunter(&s);
        s.points.insert(shooter, 40);
        let decoy = *players.keys().find(|p| is_decoy(**p)).unwrap();
        let id = box_of(&boxes, decoy);
        s.box_shot(shooter, id, &mut players, &mut boxes);
        let found = *s.hiders.iter().next().unwrap();
        s.points.insert(found, 30);
        let id = box_of(&boxes, found);
        s.box_shot(shooter, id, &mut players, &mut boxes);
        assert_eq!(s.feed, vec![
            FeedLine { shooter, found: None, red_handed: false, points: -10 },
            // they only had 30 to lose
            FeedLine { shooter, found: Some(found), red_handed: false, points: -30 },
        ]);
    }

    // the first hider found, so they hunt now (needs a 3rd player still hiding)
    fn a_convert(s: &mut SmokeState, players: &mut HashMap<PeerId, Player>, boxes: &mut Boxes) -> PeerId {
        let found = *s.hiders.iter().next().unwrap();
        let id = box_of(boxes, found);
        s.box_shot(hunter(s), id, players, boxes);
        assert!(s.found.contains(&found));
        found
    }

    fn decoy_box(players: &HashMap<PeerId, Player>, boxes: &Boxes) -> u16 {
        let decoy = *players.keys().find(|p| is_decoy(**p)).unwrap();
        box_of(boxes, decoy)
    }

    #[test]
    fn a_convert_finding_someone_gets_nothing_but_the_hider_still_loses() {
        let (mut s, mut players, mut boxes) = released(3);
        let convert = a_convert(&mut s, &mut players, &mut boxes);
        let before = s.points.get(&convert).copied().unwrap_or(0);
        let last = *s.hiders.iter().next().unwrap();
        s.points.insert(last, 80);
        let id = box_of(&boxes, last);
        assert_eq!(s.box_shot(convert, id, &mut players, &mut boxes), Shot::Found(last));
        assert_eq!(s.points[&last], 30, "80 - 50");
        // and no win bonus or mvp either, the round ended with that find
        assert_eq!(s.points.get(&convert).copied().unwrap_or(0), before);
    }

    #[test]
    fn a_converts_misses_lock_the_gun_longer_each_time() {
        let (mut s, mut players, mut boxes) = released(3);
        let convert = a_convert(&mut s, &mut players, &mut boxes);
        for want in [2.0, 3.0, 5.0, 10.0, 20.0, 20.0] {
            let id = decoy_box(&players, &boxes);
            assert_eq!(s.box_shot(convert, id, &mut players, &mut boxes), Shot::Decoy);
            assert_eq!(s.gun_locks[&convert].locked_for, want);
            s.tick_gun_locks(want);
        }
    }

    #[test]
    fn a_locked_gun_breaks_nothing() {
        let (mut s, mut players, mut boxes) = released(3);
        let convert = a_convert(&mut s, &mut players, &mut boxes);
        let id = decoy_box(&players, &boxes);
        s.box_shot(convert, id, &mut players, &mut boxes);
        let id = decoy_box(&players, &boxes);
        assert_eq!(s.box_shot(convert, id, &mut players, &mut boxes), Shot::NotAllowed);
        assert!(boxes.carrier_of(id).is_some(), "the box is still there");
    }

    #[test]
    fn thirty_clean_seconds_after_the_lock_start_the_steps_over() {
        let (mut s, mut players, mut boxes) = released(3);
        let convert = a_convert(&mut s, &mut players, &mut boxes);
        let id = decoy_box(&players, &boxes);
        s.box_shot(convert, id, &mut players, &mut boxes);
        // the 2 s lock, then 29 s clean: not yet
        s.tick_gun_locks(2.0);
        s.tick_gun_locks(SMOKE_CLEAN_RESET - 1.0);
        assert_eq!(s.gun_locks[&convert].step, 1);
        s.tick_gun_locks(1.0);
        assert_eq!(s.gun_locks[&convert], GunLock::default());
    }

    #[test]
    fn a_find_starts_the_steps_over() {
        let (mut s, mut players, mut boxes) = released(4);
        let convert = a_convert(&mut s, &mut players, &mut boxes);
        for _ in 0..2 {
            let id = decoy_box(&players, &boxes);
            s.box_shot(convert, id, &mut players, &mut boxes);
            s.tick_gun_locks(10.0);
        }
        let next = *s.hiders.iter().next().unwrap();
        let id = box_of(&boxes, next);
        s.box_shot(convert, id, &mut players, &mut boxes);
        assert_eq!(s.gun_locks[&convert], GunLock::default());
    }

    #[test]
    fn starting_hunters_never_get_locked() {
        let (mut s, mut players, mut boxes) = released(2);
        let shooter = hunter(&s);
        for _ in 0..3 {
            let id = decoy_box(&players, &boxes);
            assert_eq!(s.box_shot(shooter, id, &mut players, &mut boxes), Shot::Decoy);
        }
        assert!(s.gun_locks.is_empty());
    }

    #[test]
    fn hiders_cant_shoot_and_nobody_shoots_while_blind() {
        let mut players = humans(2);
        let mut boxes = Boxes::new();
        let mut s = SmokeState::new();
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
        let (mut s, mut players, mut boxes) = released(2);
        let shooter = hunter(&s);
        let found = *s.hiders.iter().next().unwrap();
        let id = box_of(&boxes, found);
        s.box_shot(shooter, id, &mut players, &mut boxes);
        assert_eq!(s.phase, SmokePhase::Results);
        // find at x1.0, then the win bonus and mvp at x1.2 (the find bumped it)
        let after_find = |pts: i32| (pts as f32 * (1.0 + SMOKE_FIND_MULT_STEP)).round() as i32;
        assert_eq!(s.points[&shooter], SMOKE_POINTS_FIND + after_find(SMOKE_POINTS_WIN) + after_find(SMOKE_POINTS_MVP));
        assert_eq!(s.winner, SMOKE_SIDE_HUNTER);
        // the converted hider gets no win bonus
        assert_eq!(s.points.get(&found).copied().unwrap_or(0), 0);
    }
}
