//! Native UDP transport around [`Server`]: used by the dedicated server binary
//! and by game clients that host a match themselves.

use std::collections::HashMap;
use std::io::{self, ErrorKind};
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::{PeerId, Server};

pub const DEFAULT_PORT: u16 = 7878;

const PRUNE_INTERVAL: Duration = Duration::from_secs(5);
const PEER_IDLE_TTL: Duration = Duration::from_secs(10);
/// Upper bound on how long a recv waits before the server ticks again.
const TICK_WAIT: Duration = Duration::from_millis(100);

/// A [`Server`] bound to a UDP socket, mapping socket addresses to peer ids.
pub struct UdpHost {
    socket: UdpSocket,
    server: Server,
    peer_of: HashMap<SocketAddr, PeerId>,
    addr_of: HashMap<PeerId, SocketAddr>,
    last_seen: HashMap<PeerId, Instant>,
    next_peer: u32,
    last_prune: Instant,
}

impl UdpHost {
    /// Bind `0.0.0.0:port` (0 picks a free port).
    pub fn bind(port: u16, server: Server) -> io::Result<Self> {
        let socket = UdpSocket::bind(("0.0.0.0", port))?;
        socket.set_read_timeout(Some(TICK_WAIT))?;
        Ok(UdpHost {
            socket,
            server,
            peer_of: HashMap::new(),
            addr_of: HashMap::new(),
            last_seen: HashMap::new(),
            next_peer: 1,
            last_prune: Instant::now(),
        })
    }

    pub fn port(&self) -> u16 {
        self.socket.local_addr().map(|a| a.port()).unwrap_or(0)
    }

    pub fn server(&self) -> &Server {
        &self.server
    }

    /// Serve until `stop` is set.
    pub fn run(&mut self, stop: &AtomicBool) -> io::Result<()> {
        let mut buffer = [0u8; 8192];
        while !stop.load(Ordering::Relaxed) {
            self.step(&mut buffer)?;
        }
        Ok(())
    }

    /// Wait for at most one packet, then tick and flush the outbox.
    fn step(&mut self, buffer: &mut [u8]) -> io::Result<()> {
        match self.socket.recv_from(buffer) {
            Ok((0, _)) => {}
            Ok((n, src)) => {
                let peer = self.peer_for(src);
                self.last_seen.insert(peer, Instant::now());
                self.server.handle_packet(peer, &buffer[..n]);
            }
            Err(e)
                if matches!(
                    e.kind(),
                    ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::ConnectionReset
                ) => {}
            Err(e) => return Err(e),
        }

        self.server.tick();
        for (peer, bytes) in self.server.drain_outbox() {
            if let Some(addr) = self.addr_of.get(&peer) {
                let _ = self.socket.send_to(&bytes, addr);
            }
        }

        if self.last_prune.elapsed() >= PRUNE_INTERVAL {
            self.last_prune = Instant::now();
            self.prune();
        }
        Ok(())
    }

    fn peer_for(&mut self, src: SocketAddr) -> PeerId {
        if let Some(&peer) = self.peer_of.get(&src) {
            return peer;
        }
        let peer = PeerId(self.next_peer);
        self.next_peer += 1;
        self.peer_of.insert(src, peer);
        self.addr_of.insert(peer, src);
        peer
    }

    /// Forget addresses with no player (discovery pings, disconnects, timeouts).
    fn prune(&mut self) {
        let server = &self.server;
        let (peer_of, addr_of) = (&mut self.peer_of, &mut self.addr_of);
        self.last_seen.retain(|peer, seen| {
            let keep = server.has_peer(*peer) || seen.elapsed() < PEER_IDLE_TTL;
            if !keep {
                if let Some(addr) = addr_of.remove(peer) {
                    peer_of.remove(&addr);
                }
            }
            keep
        });
    }
}

/// A [`UdpHost`] running on a background thread. Dropping it stops the server.
pub struct HostHandle {
    port: u16,
    players: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl HostHandle {
    /// Bind first (so a busy port is reported here), then serve on a thread.
    pub fn spawn(port: u16, server: Server) -> io::Result<Self> {
        let mut host = UdpHost::bind(port, server)?;
        let port = host.port();
        let players = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let (players, stop) = (players.clone(), stop.clone());
            thread::Builder::new().name("gridlock-host".into()).spawn(move || {
                let mut buffer = [0u8; 8192];
                while !stop.load(Ordering::Relaxed) {
                    if let Err(e) = host.step(&mut buffer) {
                        log!("Host stopped: {}", e);
                        break;
                    }
                    players.store(host.server().player_count(), Ordering::Relaxed);
                }
            })?
        };
        Ok(HostHandle { port, players, stop, thread: Some(thread) })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Live player count, shareable with e.g. an online listing heartbeat.
    pub fn players(&self) -> Arc<AtomicUsize> {
        self.players.clone()
    }
}

impl Drop for HostHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
