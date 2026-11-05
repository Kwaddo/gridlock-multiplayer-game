use std::io::{self, ErrorKind};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};

use crate::{Link, LinkStatus};

/// Non-blocking UDP socket on an ephemeral port that talks to one server.
///
/// The socket is deliberately not `connect`ed: a server bound to 0.0.0.0 may
/// answer from a different local address than the one we dialled.
pub struct UdpLink {
    socket: UdpSocket,
    server: SocketAddr,
    buf: Box<[u8]>,
}

impl UdpLink {
    pub fn open(addr: &str) -> io::Result<Self> {
        let server = addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(ErrorKind::InvalidInput, "address did not resolve"))?;
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_nonblocking(true)?;
        Ok(UdpLink { socket, server, buf: vec![0; 8192].into_boxed_slice() })
    }
}

impl Link for UdpLink {
    fn send(&mut self, packet: &[u8]) {
        let _ = self.socket.send_to(packet, self.server);
    }

    fn recv(&mut self) -> Vec<Box<[u8]>> {
        let mut out = Vec::new();
        // Stop on WouldBlock, or on any error such as an ICMP "connection
        // refused" from a server that is not running yet.
        while let Ok((n, _)) = self.socket.recv_from(&mut self.buf) {
            out.push(self.buf[..n].into());
        }
        out
    }

    fn status(&mut self) -> LinkStatus {
        // UDP has no connection; liveness is judged by the game's own timeouts.
        LinkStatus::Open
    }
}
