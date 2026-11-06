/// Native dedicated server: UDP transport around the `Server` library.
#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::io::Result<()> {
    use gridlock_server::udp_host::{UdpHost, DEFAULT_PORT};
    use gridlock_server::{log, Server};
    use std::sync::atomic::AtomicBool;

    let port = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);

    let server = Server::new();

    let mut host = UdpHost::bind(port, server)?;
    log!("UDP server started on 0.0.0.0:{}", host.port());
    host.run(&AtomicBool::new(false))
}

/// No dedicated server in the browser: a game tab hosts with the `Server`
/// library over WebRTC instead (client/src/net/hosting/web.rs).
#[cfg(target_arch = "wasm32")]
fn main() {}
