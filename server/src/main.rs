use gridlock_server::udp_host::{UdpHost, DEFAULT_PORT};
use gridlock_server::{log, Server};
use std::sync::atomic::AtomicBool;

/// Native dedicated server: UDP transport around the `Server` library.
fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);

    let server = Server::new();

    let mut host = UdpHost::bind(port, server)?;
    log!("UDP server started on 0.0.0.0:{}", host.port());
    host.run(&AtomicBool::new(false))
}
