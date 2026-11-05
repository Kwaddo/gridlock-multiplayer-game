use std::borrow::Borrow;
use std::cell::RefCell;
use std::fmt;

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

/// Queue of outgoing packets. Mirrors `UdpSocket::send_to` so game code keeps
/// its shape; the host drains it after each packet/tick and does the real send.
#[derive(Default)]
pub struct Outbox {
    queue: RefCell<Vec<(PeerId, Vec<u8>)>>,
}

impl Outbox {
    pub fn send_to(&self, buf: &[u8], to: impl Borrow<PeerId>) -> std::io::Result<usize> {
        let to = *to.borrow();
        if !to.is_bot() {
            self.queue.borrow_mut().push((to, buf.to_vec()));
        }
        Ok(buf.len())
    }

    pub fn drain(&self) -> Vec<(PeerId, Vec<u8>)> {
        std::mem::take(&mut *self.queue.borrow_mut())
    }
}
