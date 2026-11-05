//! Browser hosting: the server runs inside this tab and is stepped once per
//! frame. Remote players reach it over WebRTC through a signalling room on the
//! registry's Worker; the hosting player talks to it through memory.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use gridlock_server::{PeerId, Server, MAX_PLAYERS};
use rand::Rng;
use transport::web::{RtcEvent, RtcSocket};
use transport::{Link, LinkStatus};

use crate::net::registry::{self, Announce, Endpoint, Listing};

/// The hosting player's seat. WebRTC peers are numbered from 1, so 0 is free.
const LOCAL_PEER: PeerId = PeerId(0);
const ROOM_CODE_LEN: usize = 6;
/// No 0/O or 1/I/L, so codes survive being read out loud.
const ROOM_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

pub struct LocalHost {
    // Field order is drop order: withdraw the listing, close the room, then stop.
    listing: Option<Listing>,
    room: Option<RtcSocket>,
    server: Server,
    code: String,
    local: Rc<RefCell<Loopback>>,
    players: Arc<AtomicUsize>,
    notice: Option<String>,
}

/// Packets between the hosting player and the in-tab server.
#[derive(Default)]
struct Loopback {
    to_server: Vec<Vec<u8>>,
    to_player: Vec<Box<[u8]>>,
}

impl LocalHost {
    pub fn start(name: &str, list_online: bool) -> Result<Self, String> {
        let mut server = Server::new();
        server.set_name(name);

        let mut rng = rand::thread_rng();
        let code: String = (0..ROOM_CODE_LEN)
            .map(|_| ROOM_ALPHABET[rng.gen_range(0..ROOM_ALPHABET.len())] as char)
            .collect();

        let players = Arc::new(AtomicUsize::new(0));
        let base = registry::registry_url();
        let room = base.as_ref().map(|base| RtcSocket::host(&registry::room_url(base, &code)));
        let listing = match (list_online, &base) {
            (true, Some(base)) => Some(Listing::start(
                base.clone(),
                Announce {
                    name: name.to_string(),
                    max_players: MAX_PLAYERS.min(255) as u8,
                    endpoint: Endpoint::Room(code.clone()),
                    players: players.clone(),
                },
            )),
            _ => None,
        };
        let notice = Some(match room {
            Some(_) => format!("Hosting room {code}. Friends join from DIRECT with this code."),
            None => "Online play is not configured in this build: only you can join.".to_string(),
        });

        Ok(LocalHost { listing, room, server, code, local: Default::default(), players, notice })
    }

    /// What the hosting player is connected to; keys the asset cache.
    pub fn address(&self) -> String {
        format!("room:{}", self.code)
    }

    /// The hosting player's own connection: memory, no network involved.
    pub fn self_link(&mut self) -> Result<Box<dyn Link>, &'static str> {
        Ok(Box::new(LoopbackLink(self.local.clone())))
    }

    /// Step the server: take in packets from both kinds of player, tick, and
    /// deliver what it sent. Call once per frame.
    pub fn update(&mut self) {
        if let Some(room) = &mut self.room {
            for event in room.poll() {
                match event {
                    RtcEvent::Packet(peer, data) => self.server.handle_packet(PeerId(peer), &data),
                    RtcEvent::PeerLeft(peer) => self.server.peer_disconnected(PeerId(peer)),
                    RtcEvent::PeerJoined(_) => {} // they introduce themselves with Join
                    RtcEvent::Closed => {
                        self.notice = Some("Lost the signalling server: no one else can join.".into());
                    }
                }
            }
        }
        let from_player = std::mem::take(&mut self.local.borrow_mut().to_server);
        for packet in from_player {
            self.server.handle_packet(LOCAL_PEER, &packet);
        }

        self.server.tick();

        for (peer, bytes) in self.server.drain_outbox() {
            if peer == LOCAL_PEER {
                self.local.borrow_mut().to_player.push(bytes.into());
            } else if let Some(room) = &mut self.room {
                room.send(peer.0, &bytes);
            }
        }

        self.players.store(self.server.player_count(), Ordering::Relaxed);
        if let Some(listing) = &mut self.listing {
            listing.tick();
        }
    }

    pub fn take_notice(&mut self) -> Option<String> {
        self.notice.take()
    }
}

struct LoopbackLink(Rc<RefCell<Loopback>>);

impl Link for LoopbackLink {
    fn send(&mut self, packet: &[u8]) {
        self.0.borrow_mut().to_server.push(packet.to_vec());
    }

    fn recv(&mut self) -> Vec<Box<[u8]>> {
        std::mem::take(&mut self.0.borrow_mut().to_player)
    }

    fn status(&mut self) -> LinkStatus {
        LinkStatus::Open
    }
}
