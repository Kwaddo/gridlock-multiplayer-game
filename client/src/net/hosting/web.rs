//! Browser hosting: the server runs inside this tab. Remote players reach it
//! over WebRTC through a signalling room on the registry's Worker; the hosting
//! player talks to it through memory.
//!
//! The server keeps its own clock: a ticker in `web/gridlock_web/tickers.js`
//! (a worker's timer, not the animation frame) calls [`gridlock_host_tick`]
//! every few milliseconds, and the server ticks at a steady 60 Hz from those
//! calls ([`TickClock`]) whatever the host's frame rate, and through frames
//! that stall (a big asset loading, a shader compiling). Each frame steps it
//! too ([`LocalHost::update`]), which also keeps it going where the ticker
//! cannot run (no workers).
//!
//! Both callers are JS tasks of their own, and wasm runs one at a time: the
//! ticker never runs inside a frame, nor a frame inside a tick. The server
//! sits behind an `Rc<RefCell>` that neither keeps borrowed across a call out
//! to JS that could call back in; a tick that found it borrowed anyway skips
//! rather than re-enter it.

use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use gridlock_server::{Packet, PeerId, Server, MAX_PLAYERS};
use platform::Instant;
use rand::Rng;
use transport::web::{RtcEvent, RtcSocket};
use transport::{Link, LinkStatus};

use super::pace::{trim_backlog, TickClock};
use crate::net::registry::{self, Announce, Endpoint, Listing};

/// The hosting player's seat. WebRTC peers are numbered from 1, so 0 is free.
const LOCAL_PEER: PeerId = PeerId(0);
const ROOM_CODE_LEN: usize = 6;
/// No 0/O or 1/I/L, so codes survive being read out loud.
const ROOM_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

extern "C" {
    /// Start the ticker that calls [`gridlock_host_tick`] (web/gridlock_web/tickers.js).
    /// It stops by itself once that reports no game hosted.
    fn gridlock_host_ticker_start();
}

thread_local! {
    /// The game this tab hosts, for [`gridlock_host_tick`].
    static HOSTED: RefCell<Weak<RefCell<Core>>> = const { RefCell::new(Weak::new()) };
}

/// Called by the ticker in web/gridlock_web/tickers.js, between frames: step
/// the hosted server. Returns 0 when nothing is hosted (the ticker then stops).
#[no_mangle]
pub extern "C" fn gridlock_host_tick() -> u32 {
    let Some(core) = HOSTED.with(|h| h.borrow().upgrade()) else {
        return 0;
    };
    // Borrowed only if this were somehow called from inside a step: skip
    // this one rather than step re-entrantly.
    if let Ok(mut core) = core.try_borrow_mut() {
        core.step();
    }
    1
}

pub struct LocalHost {
    core: Rc<RefCell<Core>>,
    code: String,
}

/// Everything a step touches, shared with the ticker.
struct Core {
    // Field order is drop order: withdraw the listing, close the room, then stop.
    listing: Option<Listing>,
    room: Option<RtcSocket>,
    server: Server,
    local: Rc<RefCell<Loopback>>,
    players: Arc<AtomicUsize>,
    bots: Arc<AtomicUsize>,
    notice: Option<String>,
    clock: TickClock,
    /// Packets taken from the server for sending; kept so its capacity is reused.
    sending: Vec<(PeerId, Packet)>,
    /// The WebRTC peers one packet is going to (likewise kept).
    sending_to: Vec<u32>,
}

/// Packets between the hosting player and the in-tab server.
#[derive(Default)]
struct Loopback {
    /// What the hosting player sent, back to back (no allocation per
    /// packet), and where each packet ends in it.
    to_server: Vec<u8>,
    to_server_ends: Vec<usize>,
    to_player: Vec<Packet>,
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
        let bots = Arc::new(AtomicUsize::new(0));
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
                    bots: bots.clone(),
                },
            )),
            _ => None,
        };
        let notice = Some(match room {
            Some(_) => format!("Hosting room {code}. Friends join from DIRECT with this code."),
            None => "Online play is not configured in this build: only you can join.".to_string(),
        });

        let core = Rc::new(RefCell::new(Core {
            listing,
            room,
            server,
            local: Default::default(),
            players,
            bots,
            notice,
            clock: TickClock::new(Instant::now()),
            sending: Vec::new(),
            sending_to: Vec::new(),
        }));
        // The newest host is the one ticked (an older one still being
        // dropped is not).
        HOSTED.with(|h| *h.borrow_mut() = Rc::downgrade(&core));
        unsafe { gridlock_host_ticker_start() };
        Ok(LocalHost { core, code })
    }

    /// What the hosting player is connected to; keys the asset cache.
    pub fn address(&self) -> String {
        format!("room:{}", self.code)
    }

    /// The hosting player's own connection: memory, no network involved.
    pub fn self_link(&mut self) -> Result<Box<dyn Link>, &'static str> {
        let local = self.core.borrow().local.clone();
        Ok(Box::new(LoopbackLink(local)))
    }

    /// Step the server from the frame: take in what the hosting player just
    /// sent, tick if one is due, and deliver what it sent. Call once per
    /// frame; the ticker steps it between frames as well.
    pub fn update(&mut self) {
        // Frames and the ticker's steps are separate tasks, so no step is
        // under way here: still borrowed means one stopped half way (a
        // panic, which aborts on wasm). Fail this frame too, as a crash
        // did before the ticker, rather than draw on with a server that
        // never steps again.
        let Ok(mut core) = self.core.try_borrow_mut() else {
            panic!("the hosted server failed in an earlier step");
        };
        core.step();
    }

    pub fn take_notice(&mut self) -> Option<String> {
        self.core.try_borrow_mut().ok()?.notice.take()
    }
}

impl Core {
    /// Take in packets from both kinds of player, tick if one is due, and
    /// deliver what the server sent (replies go out at once, as on a
    /// dedicated server, not at the next tick).
    fn step(&mut self) {
        if let Some(room) = &mut self.room {
            let (server, notice) = (&mut self.server, &mut self.notice);
            room.poll(|event| match event {
                RtcEvent::Packet(peer, data) => server.handle_packet(PeerId(peer), data),
                RtcEvent::PeerLeft(peer) => server.peer_disconnected(PeerId(peer)),
                RtcEvent::PeerJoined(_) => {} // they introduce themselves with Join
                RtcEvent::Closed => {
                    *notice = Some("Lost the signalling server: no one else can join.".into());
                }
            });
        }
        {
            let mut local = self.local.borrow_mut();
            let Loopback { to_server, to_server_ends, .. } = &mut *local;
            let mut start = 0;
            for &end in to_server_ends.iter() {
                self.server.handle_packet(LOCAL_PEER, &to_server[start..end]);
                start = end;
            }
            to_server.clear();
            to_server_ends.clear();
        }

        if self.clock.due(Instant::now()) {
            self.server.tick();
        }

        self.server.drain_outbox_into(&mut self.sending);
        let mut local = self.local.borrow_mut();
        // A packet queued for several players (one broadcast) comes as a run
        // of the same bytes: one call into JS sends it to all its WebRTC
        // peers, and the hosting player gets the bytes themselves.
        for run in self.sending.chunk_by(|(_, a), (_, b)| Packet::ptr_eq(a, b)) {
            let packet = &run[0].1;
            self.sending_to.clear();
            for (peer, _) in run {
                if *peer == LOCAL_PEER {
                    local.to_player.push(Packet::clone(packet));
                } else {
                    self.sending_to.push(peer.0);
                }
            }
            if let (Some(room), false) = (&mut self.room, self.sending_to.is_empty()) {
                room.send_many(&self.sending_to, packet);
            }
        }
        self.sending.clear();
        trim_backlog(&mut local.to_player);
        drop(local);

        self.players.store(self.server.player_count(), Ordering::Relaxed);
        self.bots.store(self.server.bot_count(), Ordering::Relaxed);
        if let Some(listing) = &mut self.listing {
            listing.tick();
        }
    }
}

impl Drop for LocalHost {
    fn drop(&mut self) {
        // The ticker finds nothing hosted and stops (unless a newer host
        // took its place already).
        HOSTED.with(|h| {
            let mut hosted = h.borrow_mut();
            if Weak::ptr_eq(&hosted, &Rc::downgrade(&self.core)) {
                *hosted = Weak::new();
            }
        });
    }
}

struct LoopbackLink(Rc<RefCell<Loopback>>);

impl Link for LoopbackLink {
    fn send(&mut self, packet: &[u8]) {
        let mut local = self.0.borrow_mut();
        local.to_server.extend_from_slice(packet);
        let end = local.to_server.len();
        local.to_server_ends.push(end);
    }

    fn recv(&mut self) -> Vec<Packet> {
        std::mem::take(&mut self.0.borrow_mut().to_player)
    }

    fn status(&mut self) -> LinkStatus {
        LinkStatus::Open
    }
}
