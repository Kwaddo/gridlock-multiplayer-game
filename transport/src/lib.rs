//! Packet links between a game client and its host.
//!
//! Native builds talk UDP to a dedicated server. Browser builds use a WebRTC
//! data channel (through the `gridlock_web` JS plugin) to a host that runs the
//! server in its own tab.
//! Either way the game only sees whole packets going to and from one host.

#[cfg(not(target_arch = "wasm32"))]
mod udp;
#[cfg(not(target_arch = "wasm32"))]
pub use udp::UdpLink;

#[cfg(target_arch = "wasm32")]
pub mod web;
#[cfg(target_arch = "wasm32")]
pub use web::RtcLink;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkStatus {
    /// Still negotiating; packets sent now are dropped.
    Connecting,
    Open,
    /// The host went away or signalling failed. The link will not recover.
    Closed,
}

pub trait Link {
    /// Send one packet to the host. Dropped silently if the link is not open.
    fn send(&mut self, packet: &[u8]);
    /// Drain every packet that has arrived since the last call.
    fn recv(&mut self) -> Vec<Box<[u8]>>;
    fn status(&mut self) -> LinkStatus;
}

/// Open a link to `target`: `host:port` over UDP on native builds,
/// a matchbox signalling room URL (`ws://` or `wss://`) in the browser.
pub fn open(target: &str) -> std::io::Result<Box<dyn Link>> {
    #[cfg(not(target_arch = "wasm32"))]
    return Ok(Box::new(UdpLink::open(target)?));

    #[cfg(target_arch = "wasm32")]
    return Ok(Box::new(RtcLink::open(target)));
}

/// A link that first replays packets read before it was handed over
/// (e.g. everything that arrived in the same batch as the join reply).
pub struct Backlogged {
    pending: Vec<Box<[u8]>>,
    inner: Box<dyn Link>,
}

impl Backlogged {
    pub fn new(inner: Box<dyn Link>, pending: Vec<Box<[u8]>>) -> Self {
        Backlogged { pending, inner }
    }
}

impl Link for Backlogged {
    fn send(&mut self, packet: &[u8]) {
        self.inner.send(packet);
    }

    fn recv(&mut self) -> Vec<Box<[u8]>> {
        let mut out = std::mem::take(&mut self.pending);
        out.extend(self.inner.recv());
        out
    }

    fn status(&mut self) -> LinkStatus {
        self.inner.status()
    }
}
