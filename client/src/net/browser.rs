//! Finds joinable games: a LAN broadcast scan (native builds) and the online
//! host list, merged into one list. Nothing here blocks the frame.

use crate::net::registry::{self, ListFetch, Listed};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Lan,
    Online,
}

pub struct ServerEntry {
    pub name: String,
    /// What to hand to `Connector::join`: `host:port`, or a room URL in the browser.
    pub target: String,
    /// What to show under the name.
    pub detail: String,
    pub players: u8,
    pub max_players: u8,
    pub origin: Origin,
}

pub struct ServerBrowser {
    entries: Vec<ServerEntry>,
    #[cfg(not(target_arch = "wasm32"))]
    lan: Option<lan::Scan>,
    online: Option<ListFetch>,
    online_error: Option<String>,
}

impl ServerBrowser {
    /// Starts a first search straight away.
    pub fn new() -> Self {
        let mut browser = ServerBrowser {
            entries: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            lan: None,
            online: None,
            online_error: None,
        };
        browser.refresh();
        browser
    }

    pub fn refresh(&mut self) {
        self.entries.clear();
        self.online_error = None;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.lan = lan::Scan::start();
        }
        self.online = registry::registry_url().map(|url| ListFetch::start(&url));
    }

    /// Collect whatever replies arrived since the last frame.
    pub fn poll(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(scan) = &mut self.lan {
            let found = scan.poll();
            if scan.finished() {
                self.lan = None;
            }
            found.into_iter().for_each(|entry| self.add(entry));
        }

        if let Some(result) = self.online.as_mut().and_then(ListFetch::poll) {
            self.online = None;
            match result {
                Ok(listed) => listed.into_iter().filter_map(online_entry).for_each(|e| self.add(e)),
                Err(e) => self.online_error = Some(e),
            }
        }
    }

    fn add(&mut self, entry: ServerEntry) {
        // A game can show up twice (LAN and online); the LAN address is the one that works here.
        match self.entries.iter_mut().find(|e| e.target == entry.target) {
            Some(existing) if entry.origin == Origin::Lan => *existing = entry,
            Some(_) => {}
            None => self.entries.push(entry),
        }
        // Games on this network first.
        self.entries.sort_by_key(|e| e.origin != Origin::Lan);
    }

    pub fn entries(&self) -> &[ServerEntry] {
        &self.entries
    }

    pub fn is_searching(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        if self.lan.is_some() {
            return true;
        }
        self.online.is_some()
    }

    /// Why the online half of the list is missing, if it is.
    pub fn online_note(&self) -> Option<&str> {
        if registry::registry_url().is_none() {
            Some("Online list not configured")
        } else if self.online_error.is_some() {
            Some("Online list unreachable")
        } else {
            None
        }
    }
}

/// Native builds join UDP hosts; the browser build joins WebRTC rooms. The two
/// cannot reach each other, so each keeps only the listings it can join.
fn online_entry(listed: Listed) -> Option<ServerEntry> {
    let Listed { name, addr, room, players, max_players, lan } = listed;
    // Native builds find their LAN games with the broadcast scan (by LAN
    // address); a listing's public address is the online way in.
    #[cfg(not(target_arch = "wasm32"))]
    let (target, detail, origin) = {
        let _ = (room, lan);
        let addr = addr?;
        (addr.clone(), addr, Origin::Online)
    };
    // Browsers cannot scan the network, so the registry says which rooms come
    // from the player's own network; WebRTC then connects inside it.
    #[cfg(target_arch = "wasm32")]
    let (target, detail, origin) = {
        let _ = addr;
        let room = room?;
        let origin = if lan { Origin::Lan } else { Origin::Online };
        (registry::room_url(&registry::registry_url()?, &room), format!("room {room}"), origin)
    };
    Some(ServerEntry { name, target, detail, players, max_players, origin })
}

#[cfg(not(target_arch = "wasm32"))]
mod lan {
    use std::net::{Ipv4Addr, UdpSocket};
    use std::time::{Duration, Instant};

    use super::{Origin, ServerEntry};
    use crate::net::{ClientMsg, ServerMsg};

    /// How long to wait for LAN replies after a broadcast.
    const SCAN_TIME: Duration = Duration::from_secs(2);
    const DISCOVERY_PORT: u16 = gridlock_server::udp_host::DEFAULT_PORT;

    pub struct Scan {
        socket: UdpSocket,
        started: Instant,
    }

    impl Scan {
        /// Broadcast a discovery ping to the LAN on a fresh non-blocking socket.
        /// Sends to both 255.255.255.255 and the subnet-directed broadcast (x.x.x.255)
        /// because limited broadcast doesn't always reach other hosts on Linux.
        pub fn start() -> Option<Self> {
            let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
            socket.set_broadcast(true).ok()?;
            socket.set_nonblocking(true).ok()?;

            let discover = ClientMsg::Discover.encode();
            let _ = socket.send_to(&discover, ("255.255.255.255", DISCOVERY_PORT));
            if let Some(bcast) = local_broadcast_addr() {
                let _ = socket.send_to(&discover, (bcast, DISCOVERY_PORT));
            }
            Some(Scan { socket, started: Instant::now() })
        }

        pub fn poll(&mut self) -> Vec<ServerEntry> {
            let mut found = Vec::new();
            let mut buf = [0u8; 512];
            while let Ok((n, from)) = self.socket.recv_from(&mut buf) {
                if let Some(ServerMsg::Discover { players, max_players, name }) = ServerMsg::decode(&buf[..n]) {
                    let addr = from.to_string();
                    found.push(ServerEntry {
                        name,
                        target: addr.clone(),
                        detail: addr,
                        players,
                        max_players,
                        origin: Origin::Lan,
                    });
                }
            }
            found
        }

        pub fn finished(&self) -> bool {
            self.started.elapsed() >= SCAN_TIME
        }
    }

    /// Determine the local subnet broadcast address by connecting a throwaway
    /// UDP socket to a public IP (no data is sent). Assumes /24 subnet.
    fn local_broadcast_addr() -> Option<Ipv4Addr> {
        let probe = UdpSocket::bind("0.0.0.0:0").ok()?;
        // connect() on a UDP socket just sets the default destination;
        // it doesn't actually transmit anything.
        probe.connect("8.8.8.8:80").ok()?;
        match probe.local_addr().ok()?.ip() {
            std::net::IpAddr::V4(v4) => {
                let o = v4.octets();
                Some(Ipv4Addr::new(o[0], o[1], o[2], 255))
            }
            std::net::IpAddr::V6(_) => None,
        }
    }
}
