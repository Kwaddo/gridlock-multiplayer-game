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
                .any(|p| matches!(ServerMsg::decode(p), Some(ServerMsg::State { .. })));
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

/// Poll `session` until `done` holds for its player-info cache and the last
/// state, as the game's own sync does.
fn poll_until(
    session: &mut Session,
    cache: &mut client::net::PlayerInfoCache,
    mut done: impl FnMut(&client::net::PlayerInfoCache, Option<&client::net::StateSnapshot>) -> bool,
) -> bool {
    let mut last = None;
    for _ in 0..200 {
        let update = client::net::poll_updates(session.link.as_mut(), session.player_id, cache);
        if update.players.is_some() {
            last = update.players;
        }
        if done(cache, last.as_ref()) {
            return true;
        }
        thread::sleep(Duration::from_millis(10));
    }
    false
}

/// Names, teams and weapons no longer ride in every state packet: a joiner
/// still learns them, and a change still reaches the others.
#[test]
fn players_learn_names_teams_and_weapons_and_their_changes() {
    use client::net::protocols::ClientMsg;
    let host = HostHandle::spawn(0, Server::new()).unwrap();
    let addr = format!("127.0.0.1:{}", host.port());
    let mut alice = connect(&addr, "alice");
    let mut bob = connect(&addr, "bob");
    let (alice_id, bob_id) = (alice.player_id, bob.player_id);

    let mut bobs = client::net::PlayerInfoCache::new();
    let learned = poll_until(&mut bob, &mut bobs, |cache, state| {
        let named = |id: u32, name: &str| cache.get(&id).is_some_and(|i| i.name == name && i.team != 0);
        named(alice_id, "alice")
            && named(bob_id, "bob")
            && state.is_some_and(|s| s.players.iter().any(|p| p.id == alice_id))
    });
    assert!(learned, "bob never learned who is in the game: {bobs:?}");
    assert_eq!(bobs[&alice_id].weapon, "Knife");

    alice.link.send(&ClientMsg::WeaponChange { name: "AK-47".into() }.encode());
    let changed = poll_until(&mut bob, &mut bobs, |cache, _| cache[&alice_id].weapon == "AK-47");
    assert!(changed, "alice's new weapon never reached bob");
    // alice hears of it too, and of her own entry
    let mut alices = client::net::PlayerInfoCache::new();
    assert!(poll_until(&mut alice, &mut alices, |c, _| c.get(&alice_id).is_some_and(|i| i.weapon == "AK-47")));
}

/// A build from before the version check reaching a current host gets a
/// Welcome it understands and the reason, and never joins.
#[test]
fn an_old_build_is_turned_away_by_a_current_host() {
    use client::net::protocols::{MSG_JOIN, VERSION_MISMATCH_NOTICE};
    use std::net::UdpSocket;
    let host = HostHandle::spawn(0, Server::new()).unwrap();
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
    let mut join = vec![MSG_JOIN];
    join.extend_from_slice(b"oldie");
    let (mut welcomed, mut told) = (false, false);
    let mut buf = [0u8; 1500];
    for _ in 0..20 {
        socket.send_to(&join, ("127.0.0.1", host.port())).unwrap();
        while let Ok(n) = socket.recv(&mut buf) {
            match ServerMsg::decode(&buf[..n]) {
                Some(ServerMsg::Welcome { id, .. }) => welcomed |= id == 0,
                Some(ServerMsg::Chat { from_id: 0, text, .. }) => told |= text == VERSION_MISMATCH_NOTICE,
                Some(ServerMsg::State { .. }) | Some(ServerMsg::PlayerInfo(_)) => panic!("an old build got game state"),
                _ => {}
            }
        }
        if welcomed && told {
            break;
        }
    }
    assert!(welcomed && told);
    thread::sleep(Duration::from_millis(250));
    assert_eq!(host.players().load(std::sync::atomic::Ordering::Relaxed), 0);
}
