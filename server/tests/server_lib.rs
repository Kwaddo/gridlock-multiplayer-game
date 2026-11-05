use gridlock_server::{PeerId, Server, EMBEDDED_MANIFEST};

const MSG_JOIN: u8 = protocol::MSG_JOIN;

fn sent_types(out: &[(PeerId, Vec<u8>)], to: PeerId) -> Vec<u8> {
    out.iter().filter(|(p, _)| *p == to).map(|(_, b)| b[0]).collect()
}

#[test]
fn join_tick_disconnect_without_sockets() {
    let mut server = Server::new(serde_json::from_str(EMBEDDED_MANIFEST).ok());
    let (a, b) = (PeerId(1), PeerId(2));

    let mut join = vec![MSG_JOIN];
    join.extend_from_slice(b"alice");
    server.handle_packet(a, &join);
    server.tick();
    let out = server.drain_outbox();
    let types = sent_types(&out, a);
    assert!(types.contains(&protocol::MSG_WELCOME));
    assert!(types.contains(&protocol::MSG_ASSETS));
    assert!(types.contains(&protocol::MSG_STATE));

    let mut join = vec![MSG_JOIN];
    join.extend_from_slice(b"bob");
    server.handle_packet(b, &join);
    server.tick();
    let out = server.drain_outbox();
    assert!(sent_types(&out, a).contains(&protocol::MSG_STATE), "existing peer gets state");

    server.peer_disconnected(b);
    server.tick();
    assert!(server.drain_outbox().iter().all(|(p, _)| *p != b));
}

#[test]
fn every_server_packet_decodes() {
    use protocol::{ClientMsg, ServerMsg};
    let mut server = Server::new(serde_json::from_str(EMBEDDED_MANIFEST).ok());
    let (a, b) = (PeerId(1), PeerId(2));
    let mut sent = Vec::new();
    let mut step = |server: &mut Server, peer: PeerId, msg: ClientMsg| {
        server.handle_packet(peer, &msg.encode());
        server.tick();
        sent.extend(server.drain_outbox());
    };

    step(&mut server, a, ClientMsg::Join { username: "alice".into() });
    step(&mut server, b, ClientMsg::Join { username: "bob".into() });
    step(&mut server, a, ClientMsg::Discover);
    step(&mut server, a, ClientMsg::SetTeam { team: 1 });
    step(&mut server, a, ClientMsg::WeaponChange { name: "AK".into() });
    step(&mut server, a, ClientMsg::BotSpawn { behavior: 255, level: 1 });
    step(&mut server, a, ClientMsg::Position {
        position: (1.0, 0.0, 1.0),
        yaw: 0.0,
        state_flags: Some(0),
        event_flags: Some(0),
    });
    step(&mut server, a, ClientMsg::Hit {
        victim_id: 2,
        damage: 10.0,
        weapon: Some("AK".into()),
        backstab: Some(false),
    });
    // non-host start request must be denied
    step(&mut server, b, ClientMsg::StartGame { mode: Some(2), duration: Some(0), autofill: None });
    step(&mut server, a, ClientMsg::StartGame { mode: Some(2), duration: Some(0), autofill: None });

    assert!(!sent.is_empty());
    for (_, bytes) in &sent {
        assert!(ServerMsg::decode(bytes).is_some(), "undecodable packet type {}", bytes[0]);
    }
    let types: std::collections::HashSet<u8> = sent.iter().map(|(_, b)| b[0]).collect();
    for t in [
        protocol::MSG_WELCOME,
        protocol::MSG_ASSETS,
        protocol::MSG_STATE,
        protocol::MSG_PLAYER_LIST,
        protocol::MSG_LOBBY_LEADER,
        protocol::MSG_DISCOVER,
        protocol::MSG_START_DENIED,
        protocol::MSG_MATCH_COUNTDOWN,
    ] {
        assert!(types.contains(&t), "server never sent type {t}; got {types:?}");
    }
}
