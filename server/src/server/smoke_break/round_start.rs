/*
    a new round: random sides, everyone placed, hiders and decoys boxed, hunters frozen and blind
    decoys are bot players the server makes itself, they get their own range of peers
    so the bot manager (the fighting bots) never touches them
*/

use std::collections::HashMap;
use std::time::Duration;

use platform::Instant;
use rand::seq::SliceRandom;
use rand::Rng;

use super::spots::random_spot;
use super::walker::Walker;
use super::{hunter_count, SmokeState, SMOKE_BLIND_TIME};
use crate::net::{PeerId, BOT_PEER_BASE};
use crate::player::{Player, Team, WorldId};
use crate::server::boxes::Boxes;

// the crowd
pub const SMOKE_DECOYS: usize = 36;
// every hunter gets this and only this
pub const HUNTER_WEAPON: &str = "Desert Eagle";
// hunters wait in a row in the middle of the middle room while blind
const HUNTER_SPOT: (f32, f32) = (30.6, 9.0);
const HUNTER_GAP: f32 = 1.2;

// decoys get peers up here, way past where the fighting bots count up to
const DECOY_PEERS: u32 = BOT_PEER_BASE + 0x8000;
const SMOKE_PEERS_END: u32 = BOT_PEER_BASE + 0x9000;
// and player ids, fighting bots start at 9000
const DECOY_IDS: u32 = 30000;

/// a box the server walks around itself, the only players smoke break makes
pub fn is_decoy(peer: PeerId) -> bool {
    (DECOY_PEERS..SMOKE_PEERS_END).contains(&peer.0)
}

/// every decoy gone (a new round, or the match is over)
pub fn remove_smoke_entities(players: &mut HashMap<PeerId, Player>) {
    players.retain(|peer, _| !is_decoy(*peer));
}

/// Humans in the match, in a fixed order so a test can count on it (the round shuffles them anyway).
pub(super) fn humans_in_match(players: &HashMap<PeerId, Player>) -> Vec<PeerId> {
    let mut humans: Vec<PeerId> = players
        .iter()
        .filter(|(_, p)| !p.is_bot && p.world == WorldId::Maze)
        .map(|(peer, _)| *peer)
        .collect();
    humans.sort();
    humans
}

/// A decoy as a player: always a bot, no gun, facing somewhere random at a random spot.
fn spawn_box_player(players: &mut HashMap<PeerId, Player>, peer: PeerId, id: u32, name: &str, rng: &mut impl Rng) {
    let mut p = Player::new(id, name.to_string());
    let (x, z) = random_spot(rng);
    p.position = (x, 0.0, z);
    p.yaw = rng.gen_range(0.0..std::f32::consts::TAU);
    p.world = WorldId::Maze;
    p.is_bot = true;
    p.auto_respawn = false;
    p.current_weapon = String::new();
    players.insert(peer, p);
}



impl SmokeState {
    /// New round. False if there aren't enough players for one (someone to hunt, someone to hide).
    pub(in crate::server) fn start_round(&mut self, players: &mut HashMap<PeerId, Player>, boxes: &mut Boxes) -> bool {
        let mut rng = rand::thread_rng();
        let mut humans = humans_in_match(players);
        let hunter_n = hunter_count(humans.len(), humans.len());
        if hunter_n == 0 {
            return false;
        }

        // last round's crowd and boxes go, everyone starts clean
        boxes.clear_world(WorldId::Maze);
        remove_smoke_entities(players);
        self.walkers.clear();
        self.hunters.clear();
        self.hiders.clear();
        self.new_round();

        // random sides, the first few after the shuffle hunt
        humans.shuffle(&mut rng);
        let blind_until = Instant::now() + Duration::from_secs_f32(SMOKE_BLIND_TIME);
        for (i, peer) in humans.iter().enumerate() {
            let Some(p) = players.get_mut(peer) else { continue };
            p.health = p.max_health;
            p.is_alive = true;
            p.team = Team::Spectator;
            if i < hunter_n {
                // a row in the middle room, frozen till they're released
                let offset = (i as f32 - (hunter_n - 1) as f32 / 2.0) * HUNTER_GAP;
                p.position = (HUNTER_SPOT.0 + offset, 0.0, HUNTER_SPOT.1);
                p.current_weapon = HUNTER_WEAPON.to_string();
                p.post_teleport_freeze_until = Some(blind_until);
                self.hunters.insert(*peer);
            } else {
                let (x, z) = random_spot(&mut rng);
                p.position = (x, 0.0, z);
                p.post_teleport_freeze_until = None;
                self.hiders.insert(*peer);
            }
            // their game gets told where it is now
            self.respawns.push((*peer, p.position));
        }
        self.starting_hunters = self.hunters.clone();

        // decoys are walkers, only humans hide
        for i in 0..SMOKE_DECOYS as u32 {
            let peer = PeerId(DECOY_PEERS + i);
            spawn_box_player(players, peer, DECOY_IDS + i, "", &mut rng);
            self.walkers.push(Walker::new(peer));
        }

        // coins come out based on how many are hiding
        self.hiders_at_start = self.hiders.len();

        // every hider and decoy starts in a box
        let boxed: Vec<PeerId> = self.hiders.iter().copied().chain(self.walkers.iter().map(|w| w.peer)).collect();
        for peer in boxed {
            if !boxes.is_carrying(peer) {
                boxes.spawn_carried(players, peer);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn humans(n: u32) -> HashMap<PeerId, Player> {
        (1..=n)
            .map(|i| {
                let mut p = Player::new(i, format!("p{i}"));
                p.world = WorldId::Maze;
                (PeerId(i), p)
            })
            .collect()
    }

    #[test]
    fn round_start_boxes_hiders_and_decoys_and_freezes_hunters() {
        let (mut players, mut boxes) = (humans(2), Boxes::new());
        let mut s = SmokeState::new();
        assert!(s.start_round(&mut players, &mut boxes));
        assert_eq!((s.round, s.hunters.len(), s.hiders.len()), (1, 1, 1), "1 hunter, 1 hider");
        let decoys = players.keys().filter(|p| is_decoy(**p)).count();
        assert_eq!(decoys, SMOKE_DECOYS);
        // every hider and decoy is in a box, the hunter isn't
        for peer in s.hiders.iter().chain(players.keys().filter(|p| is_decoy(**p))) {
            assert!(boxes.is_carrying(*peer), "{peer} has no box");
        }
        let hunter = *s.hunters.iter().next().unwrap();
        assert!(!boxes.is_carrying(hunter));
        assert_eq!(players[&hunter].current_weapon, HUNTER_WEAPON);
        assert!(players[&hunter].post_teleport_freeze_until.is_some_and(|t| t > Instant::now()));
        // both humans get told where they are now
        assert_eq!(s.respawns.len(), 2);
    }

    #[test]
    fn alone_with_nobody_to_hide_no_round() {
        let (mut players, mut boxes) = (humans(1), Boxes::new());
        let mut s = SmokeState::new();
        assert!(!s.start_round(&mut players, &mut boxes));
        assert_eq!(s.round, 0);
        assert!(players.keys().all(|p| !is_decoy(*p)), "nothing spawned");
    }

    #[test]
    fn a_second_round_replaces_the_crowd() {
        let (mut players, mut boxes) = (humans(2), Boxes::new());
        let mut s = SmokeState::new();
        s.start_round(&mut players, &mut boxes);
        s.start_round(&mut players, &mut boxes);
        assert_eq!(s.round, 2);
        // still exactly one crowd, not two
        assert_eq!(players.keys().filter(|p| is_decoy(**p)).count(), SMOKE_DECOYS);
        assert_eq!(players.len(), 2 + SMOKE_DECOYS);
    }

    #[test]
    fn decoys_never_clash_with_fighting_bots() {
        // the fighting bots count up from BOT_PEER_BASE, decoys are thousands past them
        assert!(!is_decoy(PeerId(BOT_PEER_BASE + 50)));
        assert!(is_decoy(PeerId(DECOY_PEERS)));
        assert!(is_decoy(PeerId(DECOY_PEERS + 35)));
    }

}