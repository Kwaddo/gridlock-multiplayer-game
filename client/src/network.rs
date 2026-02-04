use std::net::UdpSocket;
use crate::protocols::*;

pub fn connect(ip: &str, username: &str) -> Option<(UdpSocket, u32)> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?; // Bind to any available por

    // Send join message
    let mut join_msg = vec![MSG_JOIN];
    join_msg.extend_from_slice(username.as_bytes());
    socket.send_to(&join_msg, ip).ok()?;

    socket.set_read_timeout(Some(std::time::Duration::from_secs(3))).ok()?;

    let mut buffer: [u8; 512] = [0; 512];
    let (_, _) = socket.recv_from(&mut buffer).ok()?;
    let id = u32::from_be_bytes([buffer[1], buffer[2], buffer[3], buffer[4]]);

    socket.set_nonblocking(true).ok()?;

    Some((socket, id))
}
