/*
    Cardboard boxes
    'nuff said
*/

use std::collections::HashMap;
use std::time::Duration;
use platform::Instant;

use super::{smoke_break, Server};
use crate::net::PeerId;
use crate::player::{ Player, WorldId };
use crate::protocols::*;

pub const LOBBY_BOX_SPOT: (f32, f32, f32) = (-4.0, 0.0, -4.0);
// a destroyed lobby box comes back after this
const LOBBY_BOX_RESPAWN: Duration = Duration::from_secs(5);

/// how often we resend box information, since packets can get lost
const RESEND_EVERY: Duration = Duration::from_secs(2);

struct ServerBox {
    id: u16,
    world: WorldId,
    position: (f32, f32, f32),
    yaw: f32,
    carrier: Option<PeerId>, // carrier = the player who is currently carrying this box (if any)
    // carrier's health when we last looked, if it drops they took damage and the box breaks
    carrier_health: u32,
    lobby: bool, // lobby test box respawns, nothing else does (for now)
}

pub(super) struct Boxes {
    list: Vec<ServerBox>,
    next_id: u16,
    lobby_respawn_at: Option<Instant>,
    changed: bool,
    sent_world: HashMap<PeerId, WorldId>, // what world each player was last sent box info for
    last_full: Option<Instant>, // when we last sent a full box update to all players
}

impl Boxes {
    pub fn new() -> Self {
        let mut boxes = Boxes {
            list: Vec::new(),
            next_id: 1,
            lobby_respawn_at: None,
            changed: true,
            sent_world: HashMap::new(),
            last_full: None,
        };
        boxes.spawn(WorldId::Lobby, LOBBY_BOX_SPOT, 0.0, true);
        boxes
    }

    fn spawn(&mut self, world: WorldId, position: (f32, f32, f32), yaw: f32, lobby: bool) {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.list.push(ServerBox {
            id,
            world,
            position,
            yaw,
            carrier: None,
            carrier_health: 0,
            lobby,
        });
        self.changed = true;
    }

    fn destroy(&mut self, index: usize) {
        let gone = self.list.remove(index);
        if gone.lobby {
            self.lobby_respawn_at = Some(Instant::now() + LOBBY_BOX_RESPAWN);
        }
        self.changed = true;
    }

    /// a box that starts on someone, smoke break puts every hider and decoy in one like this
    pub fn spawn_carried(&mut self, players: &HashMap<PeerId, Player>, peer: PeerId) {
        let Some(p) = players.get(&peer) else {
            return;
        };
        self.spawn(p.world, p.position, p.yaw, false);
        if let Some(b) = self.list.last_mut() {
            b.carrier = Some(peer);
            b.carrier_health = p.health;
        }
    }

    /// every box in a world gone (a smoke break round or match is over)
    pub fn clear_world(&mut self, world: WorldId) {
        let before = self.list.len();
        self.list.retain(|b| b.world != world);
        if self.list.len() != before {
            self.changed = true;
        }
    }

    
    /// who's inside this box, if anyone
    pub fn carrier_of(&self, id: u16) -> Option<PeerId> {
        self.list.iter().find(|b| b.id == id).and_then(|b| b.carrier)
    }

    /// gone, like a shot but without the shooter checks (smoke break decides those itself)
    pub fn break_box(&mut self, id: u16) -> bool {
        match self.list.iter().position(|b| b.id == id) {
            Some(i) => {
                self.destroy(i);
                true
            }
            None => false,
        }
    }


    /// the box `peer` is in, if any
    #[cfg(test)]
    pub fn box_of(&self, peer: PeerId) -> Option<u16> {
        self.list.iter().find(|b| b.carrier == Some(peer)).map(|b| b.id)
    }

    pub fn is_carrying(&self, peer: PeerId) -> bool {
        self.list.iter().any(|b| b.carrier == Some(peer))
    }

    // attempt to make a player carry a box
    pub fn enter(
        &mut self,
        players: &HashMap<PeerId, Player>,
        peer: PeerId,
        id: u16,
        bomb_carrier: Option<u32>
    ) -> bool {
        let Some(p) = players.get(&peer) else {
            return false;
        };
        if !p.is_alive || bomb_carrier == Some(p.id) || self.is_carrying(peer) {
            return false;
        }
        // find the box with the given id
        let Some(b) = self.list.iter_mut().find(|b| b.id == id) else {
            return false;
        };
        // check if the player is close enough to the box and in the same world (range = BOX_ENTER_RANGE)
        let (dx, dz) = (p.position.0 - b.position.0, p.position.2 - b.position.2);
        if b.carrier.is_some() || b.world != p.world || (dx * dx + dz * dz).sqrt() > BOX_ENTER_RANGE {
            return false;
        }
        b.carrier = Some(peer);
        b.carrier_health = p.health;
        self.changed = true;
        true
    }

    // player gets out, the box stays where they are, facing where they faced
    pub fn exit(&mut self, players: &HashMap<PeerId, Player>, peer: PeerId) {
        let Some(b) = self.list.iter_mut().find(|b| b.carrier == Some(peer)) else {
            return;
        };
        if let Some(p) = players.get(&peer) {
            b.position = p.position;
            b.yaw = p.yaw;
        }
        b.carrier = None;
        self.changed = true;
    }

    // someone shot this box (theirs from inside, or anyone's), its gone
    pub fn shot(&mut self, players: &HashMap<PeerId, Player>, peer: PeerId, id: u16) {
        let Some(p) = players.get(&peer) else {
            return;
        };
        if !p.is_alive {
            return;
        }
        if let Some(i) = self.list.iter().position(|b| b.id == id && b.world == p.world) {
            self.destroy(i);
        }
    }

    // update box positions based on the players carrying them, and respawn lobby boxes if needed
    pub fn update(&mut self, players: &HashMap<PeerId, Player>) {
        let mut i = 0;
        // while less than the length of the list, check each box and update its position if its being carried by a player.
        // if the player is no longer valid (not alive, wrong world, or health dropped), destroy the box
        while i < self.list.len() {
            let b = &mut self.list[i];
            if let Some(peer) = b.carrier {
                match players.get(&peer) {
                    Some(p) if p.is_alive && p.world == b.world && p.health >= b.carrier_health => {
                        b.position = p.position;
                        b.yaw = p.yaw;
                        // regen goes up, that's fine, only a drop counts
                        b.carrier_health = p.health;
                    }
                    _ => {
                        self.destroy(i);
                        continue;
                    }
                }
            }
            i += 1;
        }
        if self.lobby_respawn_at.is_some_and(|t| Instant::now() >= t) {
            self.lobby_respawn_at = None;
            self.spawn(WorldId::Lobby, LOBBY_BOX_SPOT, 0.0, true);
        }
    }

    pub fn resend_to(&mut self, peer: PeerId) {
        self.sent_world.remove(&peer);
    }

    // generate a list of boxes for a given world, including the player id of the carrier (if any)
    // in smoke break a box someone else is in shows its alias instead (and a pose that gives nothing away),
    // `me` is the receiver when they're in a box themselves, they get theirs under their real id
    fn list_for(&self, world: WorldId, players: &HashMap<PeerId, Player>, me: Option<PeerId>) -> Vec<u8> {
        let boxes = self.list
            .iter()
            .filter(|b| b.world == world)
            .map(|b| {
                let carried = b.carrier.and_then(|peer| players.get(&peer).map(|p| (peer, p)));
                let alias = carried.and_then(|(peer, p)| smoke_break::alias_for(peer, p, me));
                let (position, yaw) = match alias {
                    Some(_) => smoke_break::masked_pose(b.position, b.yaw),
                    None => (b.position, b.yaw),
                };
                CardboardBox {
                    id: b.id,
                    position,
                    yaw,
                    // wire carrier is the player id (or alias), 0 = empty
                    carrier: alias.or(carried.map(|(_, p)| p.id)).unwrap_or(0),
                }
            })
            .collect();
        (ServerMsg::Boxes { world: world as u8, boxes }).encode()
    }
}

impl Server {
    // run box rules then send each human their world list if it has changed since the last time we sent it to them
    pub(super) fn sync_boxes(&mut self) {
        self.boxes.update(&self.players);
        for (peer, p) in self.players.iter_mut() {
            p.in_box = self.boxes.is_carrying(*peer);
            // out of the box (found, left the maze, the round's over) is out of disguise
            if !p.in_box {
                p.smoke_alias = None;
            }
        }
        let full = self.boxes.last_full.map_or(true, |t| t.elapsed() >= RESEND_EVERY);
        if full {
            self.boxes.last_full = Some(Instant::now());
        }
        // send each human player their world list if it has changed since the last time we sent it to them
        // one list per world, plus one each for whoever is boxed in smoke break (see list_for)
        let mut packets: HashMap<(u8, Option<PeerId>), Vec<u8>> = HashMap::new();
        for (peer, p) in self.players.iter().filter(|(_, p)| !p.is_bot) {
            let stale = self.boxes.sent_world.get(peer) != Some(&p.world);
            if !(full || stale || self.boxes.changed) {
                continue;
            }
            let me = p.smoke_alias.map(|_| *peer);
            let packet = packets
                .entry((p.world as u8, me))
                .or_insert_with(|| self.boxes.list_for(p.world, &self.players, me));
            let _ = self.outbox.send_to(packet, *peer);
            self.boxes.sent_world.insert(*peer, p.world);
        }
        self.boxes.changed = false;
        // forget whoever left
        let players = &self.players;
        self.boxes.sent_world.retain(|peer, _| players.contains_key(peer));
    }

    // handle a player trying to enter a box, if they are close enough and not already carrying one
    pub(super) fn handle_box_enter(&mut self, peer: PeerId, id: u16) {
        let bomb_carrier = self.snd_state.as_ref().and_then(|s| s.bomb_carrier);
        if !self.boxes.enter(&self.players, peer, id, bomb_carrier) {
            self.boxes.resend_to(peer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxes_for(out: &[(PeerId, Vec<u8>)], peer: PeerId) -> Option<Vec<CardboardBox>> {
        out.iter()
            .filter(|(p, _)| *p == peer)
            .filter_map(|(_, b)| match ServerMsg::decode(b) {
                Some(ServerMsg::Boxes { boxes, .. }) => Some(boxes),
                _ => None,
            })
            .last()
    }

    fn walk_to(server: &mut Server, peer: PeerId, pos: (f32, f32, f32)) {
        let msg = ClientMsg::Position { position: pos, yaw: 0.0, state_flags: None, event_flags: None, pitch: None };
        server.handle_packet(peer, &msg.encode());
    }

    fn id_of(server: &Server, peer: PeerId) -> u32 {
        server.players[&peer].id
    }

    #[test]
    fn joiner_gets_the_lobby_box() {
        let mut server = Server::new();
        let a = PeerId(1);
        server.handle_packet(a, &ClientMsg::join("alice").encode());
        server.tick();
        let boxes = boxes_for(&server.drain_outbox(), a).expect("no box list");
        assert_eq!(boxes.len(), 1);
        assert_eq!(boxes[0].carrier, 0);
    }

    #[test]
    fn enter_exit_and_someone_else_cant_take_it() {
        let mut server = Server::new();
        let (a, b) = (PeerId(1), PeerId(2));
        server.handle_packet(a, &ClientMsg::join("alice").encode());
        server.handle_packet(b, &ClientMsg::join("bob").encode());
        server.tick();
        let id = boxes_for(&server.drain_outbox(), a).unwrap()[0].id;

        // too far: refused, and alice gets the list again so her tween cancels
        walk_to(&mut server, a, (20.0, 0.0, 20.0));
        server.handle_packet(a, &ClientMsg::BoxEnter { id }.encode());
        server.tick();
        assert_eq!(boxes_for(&server.drain_outbox(), a).unwrap()[0].carrier, 0);

        walk_to(&mut server, a, (-4.5, 0.0, -4.0));
        walk_to(&mut server, b, (-3.5, 0.0, -4.0));
        server.handle_packet(a, &ClientMsg::BoxEnter { id }.encode());
        server.handle_packet(b, &ClientMsg::BoxEnter { id }.encode());
        server.tick();
        let out = server.drain_outbox();
        assert_eq!(boxes_for(&out, a).unwrap()[0].carrier, id_of(&server, a));
        assert_eq!(boxes_for(&out, b).unwrap()[0].carrier, id_of(&server, a));

        // walks off with it, gets out, it stays there
        walk_to(&mut server, a, (-1.0, 0.0, 2.0));
        server.handle_packet(a, &ClientMsg::BoxExit.encode());
        server.tick();
        let left = boxes_for(&server.drain_outbox(), b).unwrap()[0];
        assert_eq!((left.carrier, left.position), (0, (-1.0, 0.0, 2.0)));
    }

    #[test]
    fn shot_box_is_gone_and_the_lobby_one_comes_back() {
        let mut server = Server::new();
        let a = PeerId(1);
        server.handle_packet(a, &ClientMsg::join("alice").encode());
        server.tick();
        let id = boxes_for(&server.drain_outbox(), a).unwrap()[0].id;

        server.handle_packet(a, &ClientMsg::BoxShot { id }.encode());
        server.tick();
        assert!(boxes_for(&server.drain_outbox(), a).unwrap().is_empty());

        // skip the 5 s wait
        server.boxes.lobby_respawn_at = Some(Instant::now());
        server.tick();
        let back = boxes_for(&server.drain_outbox(), a).unwrap();
        assert_eq!(back.len(), 1);
        assert_ne!(back[0].id, id, "a respawn gets a fresh id");
    }

    #[test]
    fn carrier_losing_health_or_dying_loses_the_box() {
        let mut server = Server::new();
        let a = PeerId(1);
        server.handle_packet(a, &ClientMsg::join("alice").encode());
        server.tick();
        let id = boxes_for(&server.drain_outbox(), a).unwrap()[0].id;
        walk_to(&mut server, a, (-4.0, 0.0, -4.5));
        server.handle_packet(a, &ClientMsg::BoxEnter { id }.encode());
        server.tick();
        assert!(server.boxes.is_carrying(a));

        server.players.get_mut(&a).unwrap().health -= 10;
        server.tick();
        assert!(!server.boxes.is_carrying(a));
        assert!(boxes_for(&server.drain_outbox(), a).unwrap().is_empty());
    }

    #[test]
    fn a_hit_on_someone_in_a_box_does_nothing() {
        let mut server = Server::new();
        let (a, b) = (PeerId(1), PeerId(2));
        server.handle_packet(a, &ClientMsg::join("alice").encode());
        server.handle_packet(b, &ClientMsg::join("bob").encode());
        server.tick();
        let id = boxes_for(&server.drain_outbox(), a).unwrap()[0].id;
        walk_to(&mut server, a, (-4.0, 0.0, -4.5));
        server.handle_packet(a, &ClientMsg::BoxEnter { id }.encode());
        server.tick();
        // the lobby only lets bots and humans hurt each other, bob stands in for a bot
        server.players.get_mut(&b).unwrap().is_bot = true;
        let hp = server.players[&a].health;
        let hit = ClientMsg::Hit { victim_id: id_of(&server, a), damage: 30.0, weapon: None, backstab: None, headshot: None };
        server.handle_packet(b, &hit.encode());
        assert_eq!(server.players[&a].health, hp, "the box took it");
        assert!(server.boxes.is_carrying(a));
    }

    #[test]
    fn whoever_is_in_a_box_is_flagged_for_bots() {
        let mut server = Server::new();
        let a = PeerId(1);
        server.handle_packet(a, &ClientMsg::join("alice").encode());
        server.tick();
        let id = boxes_for(&server.drain_outbox(), a).unwrap()[0].id;
        assert!(!server.players[&a].in_box);
        walk_to(&mut server, a, (-4.0, 0.0, -4.5));
        server.handle_packet(a, &ClientMsg::BoxEnter { id }.encode());
        server.tick();
        assert!(server.players[&a].in_box);
        server.handle_packet(a, &ClientMsg::BoxExit.encode());
        server.tick();
        assert!(!server.players[&a].in_box);
    }
}
