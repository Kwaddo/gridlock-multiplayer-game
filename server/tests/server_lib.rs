use gridlock_server::{PeerId, Server};

fn sent_types(out: &[(PeerId, Vec<u8>)], to: PeerId) -> Vec<u8> {
    out.iter().filter(|(p, _)| *p == to).map(|(_, b)| b[0]).collect()
}

#[test]
fn join_tick_disconnect_without_sockets() {
    let mut server = Server::new();
    let (a, b) = (PeerId(1), PeerId(2));

    server.handle_packet(a, &protocol::ClientMsg::join("alice").encode());
    server.tick();
    let out = server.drain_outbox();
    let types = sent_types(&out, a);
    assert!(types.contains(&protocol::MSG_WELCOME));
    assert!(types.contains(&protocol::MSG_STATE));

    server.handle_packet(b, &protocol::ClientMsg::join("bob").encode());
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
    let mut server = Server::new();
    let (a, b) = (PeerId(1), PeerId(2));
    let mut sent = Vec::new();
    let mut step = |server: &mut Server, peer: PeerId, msg: ClientMsg| {
        server.handle_packet(peer, &msg.encode());
        server.tick();
        sent.extend(server.drain_outbox());
    };

    step(&mut server, a, ClientMsg::join("alice"));
    step(&mut server, b, ClientMsg::join("bob"));
    step(&mut server, a, ClientMsg::Discover);
    step(&mut server, a, ClientMsg::SetTeam { team: 1 });
    step(&mut server, a, ClientMsg::WeaponChange { name: "AK".into() });
    step(&mut server, a, ClientMsg::BotSpawn { behavior: 255, level: 1 });
    step(&mut server, a, ClientMsg::Position {
        position: (1.0, 0.0, 1.0),
        yaw: 0.0,
        state_flags: Some(0),
        event_flags: Some(0),
        pitch: Some(0.2),
    });
    step(&mut server, a, ClientMsg::Hit {
        victim_id: 2,
        damage: 10.0,
        weapon: Some("AK".into()),
        backstab: Some(false),
    });
    // non-host start request must be denied
    step(&mut server, b, ClientMsg::StartGame { mode: Some(2), duration: Some(0), autofill: None, blue_bots: None });
    step(&mut server, a, ClientMsg::StartGame { mode: Some(2), duration: Some(0), autofill: None, blue_bots: None });

    assert!(!sent.is_empty());
    for (_, bytes) in &sent {
        assert!(ServerMsg::decode(bytes).is_some(), "undecodable packet type {}", bytes[0]);
    }
    let types: std::collections::HashSet<u8> = sent.iter().map(|(_, b)| b[0]).collect();
    for t in [
        protocol::MSG_WELCOME,
        protocol::MSG_STATE,
        protocol::MSG_PLAYER_LIST,
        protocol::MSG_PLAYER_INFO,
        protocol::MSG_LOBBY_LEADER,
        protocol::MSG_DISCOVER,
        protocol::MSG_START_DENIED,
        protocol::MSG_MATCH_COUNTDOWN,
    ] {
        assert!(types.contains(&t), "server never sent type {t}; got {types:?}");
    }
}

fn decoded(out: &[(PeerId, Vec<u8>)], to: PeerId) -> Vec<protocol::ServerMsg> {
    out.iter()
        .filter(|(p, _)| *p == to)
        .filter_map(|(_, b)| protocol::ServerMsg::decode(b))
        .collect()
}

fn joined_pair() -> (Server, PeerId, PeerId) {
    use protocol::ClientMsg;
    let mut server = Server::new();
    let (a, b) = (PeerId(1), PeerId(2));
    server.handle_packet(a, &ClientMsg::join("alice").encode());
    server.handle_packet(b, &ClientMsg::join("bob").encode());
    server.tick();
    (server, a, b)
}

#[test]
fn join_and_leave_are_announced() {
    use protocol::{ServerMsg, LEAVE_QUIT};
    let mut server = Server::new();
    let (a, b) = (PeerId(1), PeerId(2));
    server.handle_packet(a, &protocol::ClientMsg::join("alice").encode());
    server.drain_outbox();
    server.handle_packet(b, &protocol::ClientMsg::join("bob").encode());
    let out = server.drain_outbox();
    assert!(decoded(&out, a).contains(&ServerMsg::PlayerJoined { id: 2, name: "bob".into() }));
    assert!(!decoded(&out, b).iter().any(|m| matches!(m, ServerMsg::PlayerJoined { .. })));

    server.handle_packet(b, &protocol::ClientMsg::Disconnect.encode());
    let out = server.drain_outbox();
    assert!(decoded(&out, a)
        .contains(&ServerMsg::PlayerLeft { id: 2, name: "bob".into(), reason: LEAVE_QUIT }));
}

#[test]
fn chat_reaches_everyone_cleaned_and_rate_limited() {
    use protocol::{ClientMsg, ServerMsg};
    let (mut server, a, b) = joined_pair();
    server.drain_outbox();

    server.handle_packet(a, &ClientMsg::Chat { text: "  gg\n wp ".into() }.encode());
    let out = server.drain_outbox();
    let line = ServerMsg::Chat { from_id: 1, name: "alice".into(), text: "gg wp".into() };
    assert!(decoded(&out, a).contains(&line));
    assert!(decoded(&out, b).contains(&line));

    // straight away again: dropped
    server.handle_packet(a, &ClientMsg::Chat { text: "spam".into() }.encode());
    // blank: dropped
    server.handle_packet(b, &ClientMsg::Chat { text: "   ".into() }.encode());
    let out = server.drain_outbox();
    assert!(!out.iter().any(|(_, b)| b[0] == protocol::MSG_CHAT));
}

fn infos_to(out: &[(PeerId, Vec<u8>)], to: PeerId) -> Vec<protocol::PlayerInfo> {
    decoded(out, to)
        .into_iter()
        .filter_map(|m| match m {
            protocol::ServerMsg::PlayerInfo(e) => Some(e),
            _ => None,
        })
        .flatten()
        .collect()
}

/// Names, teams and weapons left the state packet: a joiner still learns
/// them, and changes still reach everyone.
#[test]
fn names_teams_and_weapons_reach_players_through_player_info() {
    use protocol::{ClientMsg, ServerMsg};
    let (mut server, a, b) = joined_pair();
    let out = server.drain_outbox();
    let to_b = infos_to(&out, b);
    for name in ["alice", "bob"] {
        let info = to_b.iter().find(|i| i.name == name).unwrap_or_else(|| panic!("bob never heard of {name}"));
        assert_ne!(info.team, 0, "lobby players are on a team");
        assert_eq!(info.weapon, "Knife");
    }
    // the state packet carries both, without names
    let state = decoded(&out, b).into_iter().rev().find_map(|m| match m {
        ServerMsg::State { players, .. } => Some(players),
        _ => None,
    });
    assert_eq!(state.map(|p| p.len()), Some(2));

    // a weapon change goes out as it happens, without waiting for a tick
    server.handle_packet(a, &ClientMsg::WeaponChange { name: "AK-47".into() }.encode());
    let out = server.drain_outbox();
    assert!(infos_to(&out, b).iter().any(|i| i.id == 1 && i.weapon == "AK-47"));
    assert!(infos_to(&out, a).iter().any(|i| i.id == 1 && i.weapon == "AK-47"));

    // a team change, with the next tick: with three in the lobby the host
    // may leave the bigger team
    let c = PeerId(3);
    server.handle_packet(c, &ClientMsg::join("cat").encode());
    server.tick();
    let teams = infos_to(&server.drain_outbox(), c);
    let team_of = |id: u32| teams.iter().find(|i| i.id == id).unwrap().team;
    let alice_team = team_of(1);
    assert_eq!(teams.iter().filter(|i| i.team == alice_team).count(), 2, "{teams:?}");
    let other = if alice_team == 1 { 2 } else { 1 };
    server.handle_packet(a, &ClientMsg::SetTeam { team: other }.encode());
    server.tick();
    let out = server.drain_outbox();
    for peer in [a, b, c] {
        assert!(infos_to(&out, peer).iter().any(|i| i.id == 1 && i.team == other));
    }
}

/// A build from before the version check, or any other version, never joins.
#[test]
fn other_versions_are_turned_away() {
    use protocol::{ClientMsg, ServerMsg, PROTOCOL_VERSION};
    let mut server = Server::new();
    let (old, newer) = (PeerId(1), PeerId(2));
    let mut join = vec![protocol::MSG_JOIN];
    join.extend_from_slice(b"oldie");
    server.handle_packet(old, &join);
    server.handle_packet(newer, &ClientMsg::Join { username: "newbie".into(), version: Some(PROTOCOL_VERSION + 1) }.encode());
    server.tick();
    let out = server.drain_outbox();
    for (peer, name) in [(old, "oldie"), (newer, "newbie")] {
        let got = decoded(&out, peer);
        assert!(got.contains(&ServerMsg::Welcome { id: 0, username: name.into(), version: Some(PROTOCOL_VERSION) }));
        assert!(got.iter().any(|m| matches!(m, ServerMsg::Chat { from_id: 0, .. })));
        assert!(!got.iter().any(|m| matches!(m, ServerMsg::State { .. })));
        assert!(!server.has_peer(peer));
    }
    assert_eq!(server.player_count(), 0);
}
