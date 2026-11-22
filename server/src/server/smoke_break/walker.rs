/*
    the decoy brain, named after the walking dead walkers lol. every decoy box and every bot hider runs one
    pause, pick an empty spot, face it (turn first or curve on the way), walk, repeat
    moves exactly like a hider box can (box speed, forward only, a human-ish turn rate) so copying it is fair
    knows where every box is and where every decoy is heading (the crowd map) so the crowd spreads out,
    and looks ahead so boxes rarely bump, before this they picked from 4 random spots and only dodged at the last second
*/

use std::collections::HashMap;
use rand::Rng;
use super::round_start::is_smoke_entity;
use super::spots::{random_spot, route, walkable};
use super::{SmokePhase, SmokeState};
use crate::net::PeerId;
use crate::player::Player;

// same as the client's BOX_SPEED, a hider can't go faster either
const WALK_SPEED: f32 = 1.8;
const BACK_SPEED: f32 = 1.0;
// radians a second, roughly what a mouse turn looks like while walking / standing
const TURN_WALKING: f32 = 2.2;
const TURN_STANDING: f32 = 3.0;
// pauses and walks last 1..X seconds
pub const RHYTHM_MAX: f32 = 8.0;
const SIT_CHANCE: f32 = 0.12;
const SIT_TIME: (f32, f32) = (10.0, 30.0);
// a long sit only happens with nobody this close, so clumps don't freeze together
const SIT_ALONE: f32 = 3.0;
const BACKUP_CHANCE: f32 = 0.1;
// spots tried when picking where to go, the emptiest wins
const TARGET_TRIES: usize = 12;
// the crowd map's squares, metres
const SQUARE: f32 = 3.0;
// look this far ahead and curve away from a box that'll be in the way
const LOOK_AHEAD: f32 = 2.5;
// a box closer to our path than this is in the way
const PATH_CLEAR: f32 = 1.1;
// how hard we curve away at most (radians off the waypoint)
const AVOID_TURN: f32 = 0.9;
// a box this close and in front still gets the hard swerve
const SWERVE_DIST: f32 = 1.0;
const SWERVE_ANGLE: f32 = 75.0 * std::f32::consts::PI / 180.0;
const ARRIVE: f32 = 0.4;


// every box at a glance, built once a tick: where they are, and how many boxes are in or headed for each square of the map.
pub struct Crowd {
    pub boxes: Vec<(f32, f32)>,
    counts: Vec<u16>,
    cols: usize,
}

impl Crowd {
    // 'boxes' is every box (humans too), 'heading' every spot a decoy is already walking to.
    pub fn new(boxes: Vec<(f32, f32)>, heading: &[(f32, f32)]) -> Self {
        let width = crate::maze::arena::ARENA_W as f32 * crate::server::game::MAZE_CELL_SIZE;
        let depth = crate::maze::arena::ARENA_H as f32 * crate::server::game::MAZE_CELL_SIZE;
        let cols = (width / SQUARE).ceil() as usize;
        let rows = (depth / SQUARE).ceil() as usize;
        let mut crowd = Crowd { boxes: Vec::new(), counts: vec![0; cols * rows], cols };
        for &spot in boxes.iter().chain(heading) {
            crowd.claim(spot);
        }
        crowd.boxes = boxes;
        crowd
    }

    fn square(&self, x: f32, z: f32) -> Option<usize> {
        let (c, r) = ((x / SQUARE).floor(), (z / SQUARE).floor());
        let i = r as usize * self.cols + c as usize;
        (c >= 0.0 && r >= 0.0 && (c as usize) < self.cols && i < self.counts.len()).then_some(i)
    }

    // one more box there (someone just picked it as where they're going).
    pub fn claim(&mut self, spot: (f32, f32)) {
        if let Some(i) = self.square(spot.0, spot.1) {
            self.counts[i] += 1;
        }
    }

    // boxes in and around the square at a spot (it and its 8 neighbours)
    fn crowding(&self, spot: (f32, f32)) -> u32 {
        let mut total = 0;
        for dz in [-SQUARE, 0.0, SQUARE] {
            for dx in [-SQUARE, 0.0, SQUARE] {
                if let Some(i) = self.square(spot.0 + dx, spot.1 + dz) {
                    total += self.counts[i] as u32;
                }
            }
        }
        total
    }
}


#[derive(Clone, Copy, Debug, PartialEq)]
enum Doing {
    Pause,
    // turning on the spot before a walk
    Turn,
    Walk,
    Back,
    Sit,
}

pub struct Walker {
    pub peer: PeerId,
    doing: Doing,
    // waypoints through the doorways, the last one is where we're going
    route: Vec<(f32, f32)>,
    // time left in whatever we're doing
    timer: f32,
    curve: bool,
}

impl Walker {
    pub fn new(peer: PeerId) -> Self {
        // start somewhere in the rhythm so they don't all set off together
        let timer = rand::thread_rng().gen_range(0.0..RHYTHM_MAX);
        Walker { peer, doing: Doing::Pause, route: Vec::new(), timer, curve: false }
    }

    // where were walking to
    pub fn heading(&self) -> Option<(f32, f32)> {
        matches!(self.doing, Doing::Walk | Doing::Turn).then(|| self.route.last().copied()).flatten()
    }
    
    // one step of the brain
    pub fn update(&mut self, dt: f32, me: &mut Player, crowd: &mut Crowd) {
        let mut rng = rand::thread_rng();
        let here = (me.position.0, me.position.2);
        self.timer -= dt;
        match self.doing {
            Doing::Pause | Doing::Sit => {
                if self.timer <= 0.0 {
                    self.start_walk(here, crowd, &mut rng);
                }
            }
            Doing::Turn => {
                let Some(&next) = self.route.first() else {
                    return self.pause(here, crowd, &mut rng);
                };
                if turn_towards(me, next, TURN_STANDING * dt).abs() < 0.05 {
                    self.doing = Doing::Walk;
                }
            }
            Doing::Back => {
                let back = (-me.yaw.cos() * BACK_SPEED * dt, -me.yaw.sin() * BACK_SPEED * dt);
                if self.timer <= 0.0 || !step(me, back) {
                    self.pause(here, crowd, &mut rng);
                }
            }
            Doing::Walk => self.walk(dt, me, crowd, &mut rng),
        }
    }
    
    fn walk(&mut self, dt: f32, me: &mut Player, crowd: &Crowd, rng: &mut impl Rng) {
        let here = (me.position.0, me.position.2);
        while self.route.first().is_some_and(|p| dist(here, *p) < ARRIVE) {
            self.route.remove(0);
        }
        if self.route.is_empty() {
            return self.arrived(here, crowd, rng);
        }
        if self.timer <= 0.0 {
            // walked long enough, stop wherever we are
            return self.arrived(here, crowd, rng);
        }
        let (fx, fz) = (me.yaw.cos(), me.yaw.sin());
        let (mut closest, mut side) = (f32::MAX, 0.0);
        for o in &crowd.boxes {
            let (dx, dz) = (o.0 - here.0, o.1 - here.1);
            // how far along our path it is, and how far to the side of it
            let ahead = dx * fx + dz * fz;
            let across = dx * -fz + dz * fx;
            if ahead > 0.05 && ahead < LOOK_AHEAD && across.abs() < PATH_CLEAR && ahead < closest {
                closest = ahead;
                // curve the other way from where it sits, if it's dead ahead pick a side
                side = if across.abs() < 0.05 { if rng.gen_bool(0.5) { 1.0 } else { -1.0 } } else { -across.signum() };
            }
        }
        if closest < SWERVE_DIST {
            // too close to curve, sidestep off our path then carry on
            let yaw = me.yaw + side * SWERVE_ANGLE;
            let off = (here.0 + yaw.cos() * 1.5, here.1 + yaw.sin() * 1.5);
            if walkable(off.0, off.1) {
                self.route.insert(0, off);
            }
        }
        // the closer it is the harder we lean away from the waypoint
        let lean = if closest < LOOK_AHEAD { side * AVOID_TURN * (1.0 - closest / LOOK_AHEAD) } else { 0.0 };
        let target = self.route[0];
        let toward = (target.1 - here.1).atan2(target.0 - here.0) + lean;
        let aim = (here.0 + toward.cos(), here.1 + toward.sin());
        let left = turn_towards(me, aim, TURN_WALKING * dt);
        // way off to the side, turn more before moving on
        if left.abs() > 1.2 {
            return;
        }
        let speed = WALK_SPEED * dt;
        if !step(me, (me.yaw.cos() * speed, me.yaw.sin() * speed)) {
            // the curve ran into a wall, square up and try again, or give up
            if self.curve || lean != 0.0 {
                self.curve = false;
                self.doing = Doing::Turn;
            } else {
                self.pause(here, crowd, rng);
            }
        }
    }

    
    fn arrived(&mut self, here: (f32, f32), crowd: &Crowd, rng: &mut impl Rng) {
        if rng.gen::<f32>() < BACKUP_CHANCE {
            // a human backs up a step now and then, so we do too
            self.doing = Doing::Back;
            self.timer = rng.gen_range(0.4..1.2);
            return;
        }
        self.pause(here, crowd, rng);
    }

    fn pause(&mut self, here: (f32, f32), crowd: &Crowd, rng: &mut impl Rng) {
        self.route.clear();
        let alone = !crowd.boxes.iter().any(|o| dist(here, *o) < SIT_ALONE && dist(here, *o) > 0.01);
        if alone && rng.gen::<f32>() < SIT_CHANCE {
            self.doing = Doing::Sit;
            self.timer = rng.gen_range(SIT_TIME.0..SIT_TIME.1);
        } else {
            self.doing = Doing::Pause;
            self.timer = rng.gen_range(1.0..RHYTHM_MAX);
        }
    }

    fn start_walk(&mut self, here: (f32, f32), crowd: &mut Crowd, rng: &mut impl Rng) {
        // how long this walk lasts is decided first (the 1..X rhythm), then we go somewhere we can reach in that time
        let walk_for = rng.gen_range(1.0..RHYTHM_MAX);
        let reach = walk_for * WALK_SPEED;
        // keep trying spots till we have a few in reach, the one with the fewest boxes around wins
        let target = (0..TARGET_TRIES * 4)
            .map(|_| random_spot(rng))
            .filter(|s| route_length(here, *s) <= reach)
            .take(TARGET_TRIES)
            .min_by_key(|s| crowd.crowding(*s))
            .unwrap_or_else(|| random_spot(rng));
        // claimed now, so a decoy picking later this same tick sees it as taken
        crowd.claim(target);
        self.route = route(here, target);
        self.curve = rng.gen_bool(0.5);
        self.doing = if self.curve { Doing::Walk } else { Doing::Turn };
        // a little slack for turning
        self.timer = walk_for + 0.5;
    }
}



// how far the walk from 'from' to 'to' is, through the doorways.
fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

// how far the walk from 'from' to 'to' is, through the doorways.
fn route_length(from: (f32, f32), to: (f32, f32)) -> f32 {
    let mut at = from;
    let mut length = 0.0;
    for p in route(from, to) {
        length += dist(at, p);
        at = p;
    }
    length
}

// turn up to 'max' radians towards 'to', returns how far off we still are.
fn turn_towards(me: &mut Player, to: (f32, f32), max: f32) -> f32 {
    let want = (to.1 - me.position.2).atan2(to.0 - me.position.0);
    let mut diff = want - me.yaw;
    while diff > std::f32::consts::PI {
        diff -= std::f32::consts::TAU;
    }
    while diff < -std::f32::consts::PI {
        diff += std::f32::consts::TAU;
    }
    let turn = diff.clamp(-max, max);
    me.yaw += turn;
    diff - turn
}

// move by 'by' if the box still fits there.
fn step(me: &mut Player, by: (f32, f32)) -> bool {
    let (x, z) = (me.position.0 + by.0, me.position.2 + by.1);
    if !walkable(x, z) {
        return false;
    }
    me.position.0 = x;
    me.position.2 = z;
    true
}

impl SmokeState {
    /// every decoy and bot hider takes a step, they all see the same crowd map, nobody moves during the vote
    pub(in crate::server) fn walk_crowd(&mut self, dt: f32, players: &mut HashMap<PeerId, Player>) {
        if self.phase == SmokePhase::Vote {
            return;
        }
        // every box, humans hiding too, so decoys keep clear of them the same way
        let boxes: Vec<(f32, f32)> = players
            .iter()
            .filter(|(peer, p)| p.is_alive && (is_smoke_entity(**peer) || self.hiders.contains(peer)))
            .map(|(_, p)| (p.position.0, p.position.2))
            .collect();
        let heading: Vec<(f32, f32)> = self.walkers.iter().filter_map(|w| w.heading()).collect();
        let mut crowd = Crowd::new(boxes, &heading);
        for w in self.walkers.iter_mut() {
            let Some(p) = players.get_mut(&w.peer).filter(|p| p.is_alive) else { continue };
            // a box doesn't dodge itself
            let me = (p.position.0, p.position.2);
            crowd.boxes.retain(|b| *b != me);
            w.update(dt, p, &mut crowd);
            crowd.boxes.push((p.position.0, p.position.2));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // three minutes of 36 decoys at 30 ticks a second, they stay on the map, move, and rarely bump
    #[test]
    fn decoys_stay_in_the_map_move_and_rarely_bump() {
        let mut rng = rand::thread_rng();
        let mut walkers: Vec<Walker> = (0..36).map(|i| Walker::new(PeerId(i))).collect();
        let mut players: Vec<Player> = (0..36)
            .map(|i| {
                let mut p = Player::new(i, "decoy".into());
                let (x, z) = random_spot(&mut rng);
                p.position = (x, 0.0, z);
                p
            })
            .collect();
        let starts: Vec<(f32, f32)> = players.iter().map(|p| (p.position.0, p.position.2)).collect();
        let mut touching = std::collections::HashSet::new();
        let mut bumps = 0;
        for _ in 0..30 * 180 {
            let boxes: Vec<(f32, f32)> = players.iter().map(|p| (p.position.0, p.position.2)).collect();
            let heading: Vec<(f32, f32)> = walkers.iter().filter_map(|w| w.heading()).collect();
            let mut crowd = Crowd::new(boxes, &heading);
            for (w, p) in walkers.iter_mut().zip(players.iter_mut()) {
                // a box doesn't dodge itself
                let me = (p.position.0, p.position.2);
                crowd.boxes.retain(|b| *b != me);
                w.update(1.0 / 30.0, p, &mut crowd);
                crowd.boxes.push((p.position.0, p.position.2));
                assert!(walkable(p.position.0, p.position.2), "walked into a wall at {:?}", p.position);
            }
            // a bump is two boxes starting to overlap
            for i in 0..players.len() {
                for j in i + 1..players.len() {
                    let a = (players[i].position.0, players[i].position.2);
                    let b = (players[j].position.0, players[j].position.2);
                    if dist(a, b) < 0.8 {
                        if touching.insert((i, j)) {
                            bumps += 1;
                        }
                    } else if dist(a, b) > 1.0 {
                        touching.remove(&(i, j));
                    }
                }
            }
        }
        let moved = players.iter().zip(&starts).filter(|(p, s)| dist((p.position.0, p.position.2), **s) > 1.0).count();
        println!("36 decoys, 3 min: {bumps} bumps, {moved} moved");
        assert!(moved >= 32, "only {moved} of 36 moved");
        // the old brain bumped 800+ times, this one about 15-35, some bumping is fine (zero would make humans stand out)
        assert!(bumps < 60, "{bumps} bumps in 3 min is too many");
    }
}