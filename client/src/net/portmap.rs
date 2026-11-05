//! Asks the home router to forward the game port, so players outside the LAN
//! can reach a hosted game without manual port forwarding.
//!
//! Tries UPnP IGD first, then NAT-PMP (Apple and some other routers). Runs on
//! a background thread, renews the lease while hosting and removes the mapping
//! when dropped. Neither helps behind carrier-grade NAT, which is detected and
//! reported instead.

use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use igd_next::{AddPortError, PortMappingProtocol, SearchOptions};
use natpmp::{Natpmp, Protocol, Response};

const DESCRIPTION: &str = "Gridlock game host";
/// Requested lease; renewed at half-life so a crashed host's mapping lapses.
const LEASE: Duration = Duration::from_secs(3600);
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortStatus {
    /// Still talking to the router.
    Pending,
    /// The router forwards `external` to this machine.
    Open { external: SocketAddrV4, method: &'static str },
    /// No mapping; the reason is shown to the host.
    Unavailable(String),
}

/// Keeps a router port mapping alive until dropped. Dropping signals the
/// worker thread, which removes the mapping without blocking the game.
pub struct PortMapping {
    status: Arc<Mutex<PortStatus>>,
    stop: Arc<AtomicBool>,
}

impl PortMapping {
    pub fn start(local_port: u16) -> Self {
        let status = Arc::new(Mutex::new(PortStatus::Pending));
        let stop = Arc::new(AtomicBool::new(false));
        let (status_w, stop_w) = (status.clone(), stop.clone());
        thread::spawn(move || run(local_port, &status_w, &stop_w));
        PortMapping { status, stop }
    }

    pub fn status(&self) -> PortStatus {
        read(&self.status)
    }

    /// A handle another thread can poll for the status.
    pub fn watch(&self) -> impl Fn() -> PortStatus + Send + 'static {
        let status = self.status.clone();
        move || read(&status)
    }
}

fn read(status: &Mutex<PortStatus>) -> PortStatus {
    status.lock().map(|s| s.clone()).unwrap_or(PortStatus::Pending)
}

impl Drop for PortMapping {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn run(local_port: u16, status: &Mutex<PortStatus>, stop: &AtomicBool) {
    let set = |s: PortStatus| {
        if let Ok(mut guard) = status.lock() {
            *guard = s;
        }
    };

    let mut router = match open_upnp(local_port) {
        Ok(router) => router,
        Err(upnp_err) => match open_natpmp(local_port) {
            Ok(router) => router,
            Err(_) => return set(PortStatus::Unavailable(upnp_err)),
        },
    };

    // A private "external" address means a second NAT sits upstream (carrier-grade
    // NAT or a second router), so the mapping cannot be reached from the internet.
    let external = router.external();
    if is_private(*external.ip()) {
        router.close();
        return set(PortStatus::Unavailable(
            "your internet provider shares your public address (carrier-grade NAT)".into(),
        ));
    }
    set(PortStatus::Open { external, method: router.method() });

    let mut renewed = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(200));
        if router.lease_secs() > 0 && renewed.elapsed() >= LEASE / 2 {
            renewed = Instant::now();
            if router.renew().is_err() {
                set(PortStatus::Unavailable("the router dropped the port mapping".into()));
            }
        }
    }
    router.close();
}

enum Router {
    Upnp { gateway: igd_next::Gateway, external: SocketAddrV4, local: SocketAddr, lease: u32 },
    NatPmp { client: Natpmp, external: SocketAddrV4, local_port: u16, lease: u32 },
}

impl Router {
    fn external(&self) -> SocketAddrV4 {
        match self {
            Router::Upnp { external, .. } | Router::NatPmp { external, .. } => *external,
        }
    }

    fn method(&self) -> &'static str {
        match self {
            Router::Upnp { .. } => "UPnP",
            Router::NatPmp { .. } => "NAT-PMP",
        }
    }

    fn lease_secs(&self) -> u32 {
        match self {
            Router::Upnp { lease, .. } | Router::NatPmp { lease, .. } => *lease,
        }
    }

    fn renew(&mut self) -> Result<(), String> {
        match self {
            Router::Upnp { gateway, external, local, lease } => gateway
                .add_port(PortMappingProtocol::UDP, external.port(), *local, *lease, DESCRIPTION)
                .map_err(|e| e.to_string()),
            Router::NatPmp { client, external, local_port, lease } => {
                natpmp_map(client, *local_port, external.port(), *lease).map(|_| ())
            }
        }
    }

    fn close(self) {
        match self {
            Router::Upnp { gateway, external, .. } => {
                let _ = gateway.remove_port(PortMappingProtocol::UDP, external.port());
            }
            // NAT-PMP deletes a mapping when asked for a zero lifetime.
            Router::NatPmp { mut client, local_port, .. } => {
                let _ = natpmp_map(&mut client, local_port, 0, 0);
            }
        }
    }
}

fn open_upnp(local_port: u16) -> Result<Router, String> {
    let mut options = SearchOptions::default();
    options.timeout = Some(DISCOVERY_TIMEOUT);
    let gateway = igd_next::search_gateway(options).map_err(|_| "no UPnP router found".to_string())?;
    let local = SocketAddr::new(local_ip_towards(gateway.addr)?, local_port);
    let external_ip = match gateway.get_external_ip().map_err(|e| e.to_string())? {
        IpAddr::V4(ip) => ip,
        IpAddr::V6(_) => return Err("router reported an IPv6 address".into()),
    };

    // Prefer the same external port so the address is predictable; fall back to
    // any free one if another device already holds it.
    let mut lease = LEASE.as_secs() as u32;
    let add = |lease: u32| gateway.add_port(PortMappingProtocol::UDP, local_port, local, lease, DESCRIPTION);
    let external_port = match add(lease) {
        Ok(()) => local_port,
        Err(AddPortError::OnlyPermanentLeasesSupported) => {
            lease = 0;
            add(0).map(|()| local_port).map_err(|e| e.to_string())?
        }
        Err(_) => gateway
            .add_any_port(PortMappingProtocol::UDP, local, lease, DESCRIPTION)
            .map_err(|e| format!("router refused the mapping ({e})"))?,
    };
    let external = SocketAddrV4::new(external_ip, external_port);
    Ok(Router::Upnp { gateway, external, local, lease })
}

fn open_natpmp(local_port: u16) -> Result<Router, String> {
    let mut client = Natpmp::new().map_err(|e| e.to_string())?;
    client.send_public_address_request().map_err(|e| e.to_string())?;
    let external_ip = match natpmp_response(&mut client)? {
        Response::Gateway(g) => *g.public_address(),
        _ => return Err("unexpected NAT-PMP reply".into()),
    };
    let lease = LEASE.as_secs() as u32;
    let external_port = natpmp_map(&mut client, local_port, local_port, lease)?;
    let external = SocketAddrV4::new(external_ip, external_port);
    Ok(Router::NatPmp { client, external, local_port, lease })
}

/// Request a UDP mapping; returns the public port the router granted.
fn natpmp_map(client: &mut Natpmp, private: u16, public: u16, lease: u32) -> Result<u16, String> {
    client
        .send_port_mapping_request(Protocol::UDP, private, public, lease)
        .map_err(|e| e.to_string())?;
    match natpmp_response(client)? {
        Response::UDP(m) => Ok(m.public_port()),
        _ => Err("unexpected NAT-PMP reply".into()),
    }
}

/// natpmp's own retry schedule runs for about a minute; cap it.
fn natpmp_response(client: &mut Natpmp) -> Result<Response, String> {
    let deadline = Instant::now() + DISCOVERY_TIMEOUT;
    loop {
        match client.read_response_or_retry() {
            Ok(response) => return Ok(response),
            Err(natpmp::Error::NATPMP_TRYAGAIN) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
}

/// The LAN address the router should forward to: whichever interface routes to it.
fn local_ip_towards(gateway: SocketAddr) -> Result<IpAddr, String> {
    let probe = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    probe.connect(gateway).map_err(|e| e.to_string())?;
    probe.local_addr().map(|a| a.ip()).map_err(|e| e.to_string())
}

/// RFC 1918 private ranges plus RFC 6598 shared address space (carrier-grade NAT).
fn is_private(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    ip.is_private() || ip.is_loopback() || ip.is_unspecified() || (a == 100 && (64..128).contains(&b))
}

#[cfg(test)]
mod tests {
    use super::is_private;

    #[test]
    fn detects_addresses_behind_another_nat() {
        for ip in ["10.0.0.1", "192.168.1.1", "172.16.5.4", "100.64.0.1", "100.127.255.255", "0.0.0.0"] {
            assert!(is_private(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["8.8.8.8", "100.63.0.1", "100.128.0.1", "203.0.113.5"] {
            assert!(!is_private(ip.parse().unwrap()), "{ip}");
        }
    }
}

/// Talks to the real router on this network, so it only runs on request:
/// `cargo test -p client --lib live_router -- --ignored --nocapture`
#[cfg(test)]
mod live {
    use super::{PortMapping, PortStatus};

    #[test]
    #[ignore]
    fn live_router() {
        let mapping = PortMapping::start(7878);
        for _ in 0..100 {
            if mapping.status() != PortStatus::Pending {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        println!("{:?}", mapping.status());
        drop(mapping);
        // give the worker a moment to remove the mapping again
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
