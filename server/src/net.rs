use std::borrow::Borrow;
use std::cell::RefCell;
use std::fmt;
use std::sync::Arc;

/// Transport-agnostic peer handle. The host (UDP binary or WebRTC tab) maps
/// its real connections to these ids; bots use ids from `BOT_PEER_BASE` up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PeerId(pub u32);

pub const BOT_PEER_BASE: u32 = 0xFFFF_0000;

impl PeerId {
    pub fn is_bot(self) -> bool {
        self.0 >= BOT_PEER_BASE
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "peer#{}", self.0)
    }
}

/// Bytes of one queued packet. A broadcast queues the same allocation for
/// every recipient instead of a copy each (`Arc`: the server runs on its own
/// thread when a native client hosts).
pub type Packet = Arc<[u8]>;

/// Queue of outgoing packets. Mirrors `UdpSocket::send_to` so game code keeps
/// its shape; the host drains it after each packet/tick and does the real send.
#[derive(Default)]
pub struct Outbox {
    queue: RefCell<Vec<(PeerId, Packet)>>,
}

impl Outbox {
    pub fn send_to(&self, buf: &[u8], to: impl Borrow<PeerId>) -> std::io::Result<usize> {
        let to = *to.borrow();
        if !to.is_bot() {
            self.queue.borrow_mut().push((to, Packet::from(buf)));
        }
        Ok(buf.len())
    }

    /// [`send_to`](Self::send_to) for a packet going to several peers: queues
    /// `packet` itself rather than a copy of its bytes.
    pub fn send_shared(&self, packet: &Packet, to: impl Borrow<PeerId>) {
        let to = *to.borrow();
        if !to.is_bot() {
            self.queue.borrow_mut().push((to, Packet::clone(packet)));
        }
    }

    pub fn drain(&self) -> Vec<(PeerId, Packet)> {
        std::mem::take(&mut *self.queue.borrow_mut())
    }

    /// Move the queued packets onto the end of `out`. Unlike [`drain`](Self::drain)
    /// the queue keeps its capacity, so a host that reuses `out` stops
    /// regrowing both every tick.
    pub fn drain_into(&self, out: &mut Vec<(PeerId, Packet)>) {
        out.append(&mut self.queue.borrow_mut());
    }
}
