use crate::net::{MSG_JOIN, MSG_WELCOME};
use std::net::UdpSocket;

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
