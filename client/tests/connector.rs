// Real sockets and threads: native only.
#![cfg(not(target_arch = "wasm32"))]

use std::net::UdpSocket;
use std::thread;
use std::time::Duration;

use client::net::client::{ConnectPoll, Connector};
use client::net::protocols::{ClientMsg, ServerMsg, MSG_DISCONNECT, MSG_JOIN, MSG_WELCOME, PROTOCOL_VERSION};

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
            let welcome = ServerMsg::Welcome { id: 7, username: "tester".into(), version: Some(PROTOCOL_VERSION) };
            socket.send_to(&welcome.encode(), from).unwrap();
            socket.send_to(&ServerMsg::State { world: 0, players: Vec::new() }.encode(), from).unwrap();
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
    assert!(matches!(ServerMsg::decode(&packets[0]), Some(ServerMsg::State { .. })));
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

/// Fake host of another version: answers the first Join with a Welcome
/// carrying `version` (`None`: a host from before the version check, which
/// writes no version), and passes on what the client sends after it.
fn spawn_host_of_version(version: Option<u16>) -> (String, std::sync::mpsc::Receiver<Vec<u8>>) {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let addr = socket.local_addr().unwrap().to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let mut buf = [0u8; 512];
        let mut welcomed = false;
        socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        while let Ok((n, from)) = socket.recv_from(&mut buf) {
            if buf[0] == MSG_JOIN {
                // what a host from before the check makes of the name
                assert_eq!(String::from_utf8_lossy(&buf[1..n]).trim(), "tester");
                if !welcomed {
                    let mut welcome = vec![MSG_WELCOME];
                    welcome.extend_from_slice(&7u32.to_be_bytes());
                    welcome.push(6);
                    welcome.extend_from_slice(b"tester");
                    if let Some(v) = version {
                        welcome.extend_from_slice(&v.to_be_bytes());
                    }
                    socket.send_to(&welcome, from).unwrap();
                    welcomed = true;
                }
            } else if welcomed {
                let _ = tx.send(buf[..n].to_vec());
            }
        }
    });
    (addr, rx)
}

fn fail_reason(addr: &str) -> &'static str {
    let mut connector = Connector::join(addr, "tester").unwrap();
    loop {
        match connector.poll() {
            ConnectPoll::Pending(c) => connector = c,
            ConnectPoll::Connected(_) => panic!("joined a host of another version"),
            ConnectPoll::Failed(reason) => return reason,
        }
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn an_older_host_is_refused_with_a_clear_reason_and_told_we_left() {
    let (addr, after) = spawn_host_of_version(None);
    let reason = fail_reason(&addr);
    assert!(reason.contains("older version"), "{reason}");
    // the old host hears a Disconnect instead of timing us out
    let bye = after.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(bye, vec![MSG_DISCONNECT]);
}

#[test]
fn a_host_of_another_version_is_refused() {
    let (addr, _) = spawn_host_of_version(Some(PROTOCOL_VERSION + 1));
    assert!(fail_reason(&addr).contains("newer version"));
    let (addr, _) = spawn_host_of_version(Some(PROTOCOL_VERSION - 1));
    assert!(fail_reason(&addr).contains("older version"));
}
