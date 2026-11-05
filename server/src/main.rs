use gridlock_server::udp_host::{UdpHost, DEFAULT_PORT};
use gridlock_server::{log, Server, EMBEDDED_MANIFEST};
use std::sync::atomic::AtomicBool;

/// Native dedicated server: UDP transport around the `Server` library.
fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);

    // prefer the on-disk manifest so it can be edited without a rebuild
    let manifest_src = std::fs::read_to_string("assets/manifest.json")
        .unwrap_or_else(|_| EMBEDDED_MANIFEST.to_string());
    let server = Server::new(serde_json::from_str(&manifest_src).ok());

    let mut host = UdpHost::bind(port, server)?;
    log!("UDP server started on 0.0.0.0:{}", host.port());
    host.run(&AtomicBool::new(false))
}
