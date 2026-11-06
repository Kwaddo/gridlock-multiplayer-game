// Real sockets and threads: native only.
#![cfg(not(target_arch = "wasm32"))]

use std::thread;
use std::time::Duration;

use client::net::client::{ConnectPoll, Connector, Session};
use client::net::protocols::ServerMsg;
use gridlock_server::udp_host::HostHandle;
use gridlock_server::Server;

fn connect(addr: &str, name: &str) -> Session {
    let mut connector = Connector::join(addr, name).unwrap();
    loop {
        match connector.poll() {
            ConnectPoll::Pending(c) => connector = c,
            ConnectPoll::Connected(session) => return session,
            ConnectPoll::Failed(reason) => panic!("{name}: {reason}"),
        }
        thread::sleep(Duration::from_millis(20));
    }
}

/// A host running inside the client process takes two players, and both
/// receive game state from it.
#[test]
fn in_process_host_accepts_players() {
    let mut server = Server::new();
    server.set_name("test game");
    let host = HostHandle::spawn(0, server).unwrap();
    let addr = format!("127.0.0.1:{}", host.port());

    let mut alice = connect(&addr, "alice");
    let mut bob = connect(&addr, "bob");
    assert_ne!(alice.player_id, bob.player_id);

    for session in [&mut alice, &mut bob] {
        let mut got_state = false;
        for _ in 0..100 {
            got_state |= session
                .link
                .recv()
                .iter()
                .any(|p| matches!(ServerMsg::decode(p), Some(ServerMsg::State(_))));
            if got_state {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(got_state, "{} never got state", session.username);
    }

    // the host thread keeps the shared player count current
    thread::sleep(Duration::from_millis(250));
    assert_eq!(host.players().load(std::sync::atomic::Ordering::Relaxed), 2);
}

#[test]
fn dropping_the_host_frees_its_port() {
    let host = HostHandle::spawn(0, Server::new()).unwrap();
    let port = host.port();
    drop(host);
    // the thread has joined and the socket is closed, so the port binds again
    HostHandle::spawn(port, Server::new()).unwrap();
}
