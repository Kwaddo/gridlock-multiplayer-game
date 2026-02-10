use crate::net::{MSG_JOIN, MSG_WELCOME, MSG_DISCOVER};
use std::net::UdpSocket;

pub struct DiscoveredServer {
    pub addr: String,
    pub name: String,
    pub players: u8,
    pub max_players: u8,
}

pub fn connect(ip: &str, username: &str) -> Option<(UdpSocket, u32, String)> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?; // Bind to any available por

    // Send join message
    let mut join_msg = vec![MSG_JOIN];
    join_msg.extend_from_slice(username.as_bytes());
    socket.send_to(&join_msg, ip).ok()?;

    socket
        .set_read_timeout(Some(std::time::Duration::from_secs(3)))
        .ok()?;

    let mut buffer: [u8; 512] = [0; 512];
    let (bytes_rcv, _) = socket.recv_from(&mut buffer).ok()?;

    // Verify it's a welcome message
    if bytes_rcv < 6 || buffer[0] != MSG_WELCOME {
        return None;
    }

    let id = u32::from_be_bytes([buffer[1], buffer[2], buffer[3], buffer[4]]);
    let name_len = buffer[5] as usize;
    if bytes_rcv < 6 + name_len {
        return None;
    }
    let assigned_username = String::from_utf8_lossy(&buffer[6..6 + name_len]).to_string();

    socket.set_nonblocking(true).ok()?;

    Some((socket, id, assigned_username))
}

/// Create a broadcast socket and send a discovery packet to the LAN.
/// Sends to both 255.255.255.255 and the subnet-directed broadcast (x.x.x.255)
/// because limited broadcast doesn't always reach other hosts on Linux.
pub fn start_discovery() -> Option<UdpSocket> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.set_broadcast(true).ok()?;
    socket.set_nonblocking(true).ok()?;

    // limited broadcast (may be dropped by some routers/OSes)
    let _ = socket.send_to(&[MSG_DISCOVER], "255.255.255.255:7878");

    // subnet-directed broadcast: figure out our local IP and broadcast to x.x.x.255
    if let Some(bcast) = local_broadcast_addr() {
        let _ = socket.send_to(&[MSG_DISCOVER], &bcast);
    }

    Some(socket)
}

/// Determine the local subnet broadcast address by connecting a throwaway
/// UDP socket to a public IP (no data is sent). Assumes /24 subnet.
fn local_broadcast_addr() -> Option<String> {
    let probe = UdpSocket::bind("0.0.0.0:0").ok()?;
    // connect() on a UDP socket just sets the default destination;
    // it doesn't actually transmit anything.
    probe.connect("8.8.8.8:80").ok()?;
    let local = probe.local_addr().ok()?;
    if let std::net::IpAddr::V4(v4) = local.ip() {
        let o = v4.octets();
        Some(format!("{}.{}.{}.255:7878", o[0], o[1], o[2]))
    } else {
        None
    }
}

/// Poll a discovery socket for server responses (non-blocking).
pub fn poll_discovery(socket: &UdpSocket) -> Vec<DiscoveredServer> {
    let mut servers = Vec::new();
    let mut buf = [0u8; 512];
    while let Ok((size, addr)) = socket.recv_from(&mut buf) {
        if size >= 4 && buf[0] == MSG_DISCOVER {
            let players = buf[1];
            let max_players = buf[2];
            let name_len = buf[3] as usize;
            if size >= 4 + name_len {
                let name = String::from_utf8_lossy(&buf[4..4 + name_len]).to_string();
                servers.push(DiscoveredServer {
                    addr: addr.to_string(),
                    name,
                    players,
                    max_players,
                });
            }
        }
    }
    servers
}
