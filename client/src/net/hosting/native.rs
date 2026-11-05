//! Native hosting: the server library runs on a background thread behind UDP
//! and the hosting player joins it over loopback like anyone else. Listing it
//! online also asks the router to open the port.

use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr};

use gridlock_server::udp_host::{HostHandle, DEFAULT_PORT};
use gridlock_server::{Server, EMBEDDED_MANIFEST, MAX_PLAYERS};
use platform::{Duration, Instant};
use transport::Link;

use crate::net::portmap::{PortMapping, PortStatus};
use crate::net::registry::{self, Announce, Endpoint, Listing};

/// Stop waiting for the registry before telling the host how reachable they are.
const NOTICE_WAIT: Duration = Duration::from_secs(15);

/// A running hosted game. Dropping it takes the game off the online list,
/// closes the router port and shuts the server down, disconnecting everyone.
pub struct LocalHost {
    // Field order is drop order: withdraw the listing, close the port, then stop.
    online: Option<Online>,
    handle: HostHandle,
}

struct Online {
    listing: Listing,
    mapping: PortMapping,
    started: Instant,
    notified: bool,
}

impl LocalHost {
    /// Start a server on the standard port, so LAN discovery finds it.
    pub fn start(name: &str, list_online: bool) -> Result<Self, String> {
        let mut server = Server::new(serde_json::from_str(EMBEDDED_MANIFEST).ok());
        server.set_name(name);
        let handle = HostHandle::spawn(DEFAULT_PORT, server).map_err(|e| match e.kind() {
            ErrorKind::AddrInUse => format!("Port {DEFAULT_PORT} is already in use."),
            _ => format!("Could not start hosting: {e}"),
        })?;

        let online = match (list_online, registry::registry_url()) {
            (true, Some(url)) => Some(Online::start(url, name, &handle)),
            _ => None,
        };
        Ok(LocalHost { online, handle })
    }

    /// What the hosting player is connected to; keys the asset cache.
    pub fn address(&self) -> String {
        format!("127.0.0.1:{}", self.handle.port())
    }

    /// The hosting player's own connection to the game.
    pub fn self_link(&mut self) -> Result<Box<dyn Link>, &'static str> {
        transport::open(&self.address()).map_err(|_| "Could not connect to the hosted game.")
    }

    /// Per frame. The server and heartbeat run on their own threads here.
    pub fn update(&mut self) {
        if let Some(online) = &mut self.online {
            online.listing.tick();
        }
    }

    /// Once known, a one-time message telling the host whether players outside
    /// their network can reach the game.
    pub fn take_notice(&mut self) -> Option<String> {
        let port = self.handle.port();
        let online = self.online.as_mut().filter(|o| !o.notified)?;
        let notice = online.notice(port)?;
        online.notified = true;
        Some(notice)
    }
}

impl Online {
    fn start(url: String, name: &str, handle: &HostHandle) -> Self {
        let mapping = PortMapping::start(handle.port());
        let status = mapping.watch();
        let local_port = handle.port();
        // Advertise the port the router opened, or ours if it opened none.
        let endpoint = Endpoint::Udp(Box::new(move || match status() {
            PortStatus::Pending => None,
            PortStatus::Open { external, .. } => Some(external.port()),
            PortStatus::Unavailable(_) => Some(local_port),
        }));
        let listing = Listing::start(
            url,
            Announce {
                name: name.to_string(),
                max_players: MAX_PLAYERS.min(255) as u8,
                endpoint,
                players: handle.players(),
            },
        );
        Online { listing, mapping, started: Instant::now(), notified: false }
    }

    fn notice(&self, port: u16) -> Option<String> {
        let forward_hint = format!("Online players need UDP port {port} forwarded to this machine.");
        match self.mapping.status() {
            PortStatus::Pending => None,
            PortStatus::Unavailable(reason) => {
                Some(format!("Couldn't open a port automatically: {reason}. {forward_hint}"))
            }
            PortStatus::Open { external, method } => {
                // The registry sees the address the internet sees. If it differs
                // from the router's, another NAT sits in between.
                let seen = self.listing.listed_addr().and_then(|a| a.parse::<SocketAddr>().ok());
                match seen {
                    None if self.started.elapsed() < NOTICE_WAIT => None,
                    Some(seen) if seen.ip() != IpAddr::V4(*external.ip()) => Some(format!(
                        "Router opened a port, but your provider shares your public address. {forward_hint}"
                    )),
                    _ => Some(format!(
                        "Port {} opened on your router ({method}): online players can join.",
                        external.port()
                    )),
                }
            }
        }
    }
}
