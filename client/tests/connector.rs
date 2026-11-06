// Real sockets and threads: native only.
#![cfg(not(target_arch = "wasm32"))]

use std::net::UdpSocket;
use std::thread;
use std::time::Duration;

use client::net::client::{ConnectPoll, Connector};
use client::net::protocols::{ClientMsg, ServerMsg};

/// Fake server: ignores the first Join (as if it were lost), then answers the
/// retry with Welcome followed immediately by a State packet.
fn spawn_fake_server() -> String {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let addr = socket.local_addr().unwrap().to_string();
    thread::spawn(move || {
        let mut buf = [0u8; 512];
        let mut joins = 0;
        loop {
            let (n, from) = socket.recv_from(&mut buf).unwrap();
            if !matches!(ClientMsg::decode(&buf[..n]), Some(ClientMsg::Join { .. })) {
                continue;
            }
            joins += 1;
            if joins == 1 {
                continue;
            }
            let welcome = ServerMsg::Welcome { id: 7, username: "tester".into() };
            socket.send_to(&welcome.encode(), from).unwrap();
            socket.send_to(&ServerMsg::State(Vec::new()).encode(), from).unwrap();
            return;
        }
    });
    addr
}

#[test]
fn connector_retries_join_and_keeps_packets_after_welcome() {
    let addr = spawn_fake_server();
    let mut connector = Connector::join(&addr, "tester").unwrap();

    let session = loop {
        match connector.poll() {
            ConnectPoll::Pending(c) => connector = c,
            ConnectPoll::Connected(session) => break session,
            ConnectPoll::Failed(reason) => panic!("connect failed: {reason}"),
        }
        thread::sleep(Duration::from_millis(20));
    };
    assert_eq!((session.player_id, session.username.as_str()), (7, "tester"));
    let mut link = session.link;

    // The State packet may arrive with the Welcome or just after it.
    let mut packets = Vec::new();
    for _ in 0..50 {
        packets.extend(link.recv());
        if !packets.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(matches!(ServerMsg::decode(&packets[0]), Some(ServerMsg::State(_))));
}

#[test]
fn connector_gives_up_when_nobody_answers() {
    // Bound but silent, so Join packets vanish.
    let silent = UdpSocket::bind("127.0.0.1:0").unwrap();
    let addr = silent.local_addr().unwrap().to_string();
    let mut connector = Connector::join(&addr, "tester").unwrap();
    let started = std::time::Instant::now();
    loop {
        match connector.poll() {
            ConnectPoll::Pending(c) => connector = c,
            ConnectPoll::Connected(_) => panic!("nobody should have answered"),
            ConnectPoll::Failed(_) => break,
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert!(started.elapsed() >= Duration::from_secs(5));
}
