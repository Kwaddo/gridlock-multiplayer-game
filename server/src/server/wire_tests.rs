//! The version check, `MSG_PLAYER_INFO` sync and batched bot shots.

use super::*;
use crate::net::BOT_PEER_BASE;
use crate::player::Team;

fn to(out: &[(PeerId, Vec<u8>)], peer: PeerId) -> Vec<ServerMsg> {
    out.iter().filter(|(p, _)| *p == peer).filter_map(|(_, b)| ServerMsg::decode(b)).collect()
}

fn infos(msgs: &[ServerMsg]) -> Vec<PlayerInfo> {
    msgs.iter()
        .filter_map(|m| match m {
            ServerMsg::PlayerInfo(e) => Some(e.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

fn old_join(name: &str) -> Vec<u8> {
    let mut join = vec![MSG_JOIN];
    join.extend_from_slice(name.as_bytes());
    join
}

#[test]
fn an_old_client_is_told_then_sent_back_to_its_menu() {
    let mut server = Server::new();
    let (host, old) = (PeerId(1), PeerId(2));
    server.handle_packet(host, &ClientMsg::join("alice").encode());
    server.tick();
    server.drain_outbox();

    server.handle_packet(old, &old_join("carol"));
    server.tick();
    let out = server.drain_outbox();
    let got = to(&out, old);
    // a Welcome its connecting screen accepts, and the reason in its chat
    assert!(got.contains(&ServerMsg::Welcome { id: 0, username: "carol".into(), version: Some(PROTOCOL_VERSION) }));
    let notice = ServerMsg::Chat { from_id: 0, name: String::new(), text: VERSION_MISMATCH_NOTICE.into() };
    assert!(got.contains(&notice));
    // nothing it would misread, and nobody else hears of it
    assert!(!got.iter().any(|m| matches!(m, ServerMsg::State { .. } | ServerMsg::PlayerInfo(_))));
    assert!(!to(&out, host).iter().any(|m| matches!(m, ServerMsg::PlayerJoined { .. })));
    assert!(!server.has_peer(old) && server.is_rejected(old) && server.player_count() == 1);

    // its positions only bring the notice back now and then
    let pos = ClientMsg::Position { position: (0.0, 0.0, 0.0), yaw: 0.0, state_flags: None, event_flags: None, pitch: None };
    server.handle_packet(old, &pos.encode());
    assert!(server.drain_outbox().is_empty());
    server.age_rejection(old, Duration::from_secs(4));
    server.handle_packet(old, &pos.encode());
    assert_eq!(to(&server.drain_outbox(), old), vec![notice]);

    // later on, a player list without it: old builds return to their menu
    server.age_rejection(old, version::KICK_AFTER);
    server.handle_packet(old, &pos.encode());
    assert_eq!(to(&server.drain_outbox(), old), vec![ServerMsg::PlayerList(Vec::new())]);
    assert!(!server.has_peer(old));

    // updated, the same address gets in
    server.handle_packet(old, &ClientMsg::join("carol").encode());
    assert!(server.has_peer(old) && !server.is_rejected(old));
}

#[test]
fn another_new_version_is_turned_away_and_told_ours() {
    let mut server = Server::new();
    let peer = PeerId(5);
    for version in [Some(PROTOCOL_VERSION + 1), Some(1), Some(0)] {
        server.handle_packet(peer, &ClientMsg::Join { username: "zed".into(), version }.encode());
        let got = to(&server.drain_outbox(), peer);
        assert!(got.contains(&ServerMsg::Welcome { id: 0, username: "zed".into(), version: Some(PROTOCOL_VERSION) }));
        assert!(!server.has_peer(peer));
    }
    // leaving clears it
    server.handle_packet(peer, &ClientMsg::Disconnect.encode());
    assert!(!server.is_rejected(peer));
}

#[test]
fn info_goes_out_on_join_on_change_and_every_few_seconds() {
    let mut server = Server::new();
    let (a, b) = (PeerId(1), PeerId(2));
    server.handle_packet(a, &ClientMsg::join("alice").encode());
    server.tick();
    server.drain_outbox();
    server.handle_packet(b, &ClientMsg::join("bob").encode());
    server.tick();
    let out = server.drain_outbox();
    // the joiner gets everyone; the others get the joiner
    let mut names: Vec<String> = infos(&to(&out, b)).into_iter().map(|i| i.name).collect();
    names.sort();
    assert_eq!(names, ["alice", "bob"]);
    assert!(infos(&to(&out, a)).iter().any(|i| i.name == "bob" && i.weapon == "Knife" && i.max_health == 125));

    // nothing changed: nothing sent
    server.tick();
    assert!(!server.drain_outbox().iter().any(|(_, b)| b[0] == MSG_PLAYER_INFO));

    // every field that moved out of the state packet
    let alice = server.players.get_mut(&a).unwrap();
    alice.kills = 4;
    alice.gun_level = 3;
    alice.max_health = 150;
    alice.team = Team::Blue;
    alice.username = "alicia".into();
    alice.current_weapon = "MP5".into();
    server.tick();
    let out = server.drain_outbox();
    let want = PlayerInfo {
        id: 1,
        name: "alicia".into(),
        team: Team::Blue.as_u8(),
        weapon: "MP5".into(),
        kills: 4,
        gun_level: 3,
        max_health: 150,
    };
    // (the lobby rebalances teams, so bob may come along)
    for peer in [a, b] {
        let got = infos(&to(&out, peer));
        assert!(got.contains(&want), "{got:?}");
        assert!(got.iter().all(|i| i.id == 1 || i.team != 0));
    }

    // a lost packet is made good by the periodic resend of everyone
    server.info_sync = player_info::InfoSync::default();
    server.tick();
    assert_eq!(infos(&to(&server.drain_outbox(), a)).len(), 2);
}

#[test]
fn bot_shots_of_one_tick_share_a_packet() {
    let outbox = Outbox::default();
    let mut players = HashMap::new();
    let (human, other_human) = (PeerId(1), PeerId(2));
    players.insert(human, Player::new(1, "h".into()));
    players.insert(other_human, Player::new(2, "o".into()));
    let mut bots = Vec::new();
    for i in 0..3u32 {
        let addr = PeerId(BOT_PEER_BASE + i);
        let mut bot = Player::new(9000 + i, format!("bot{i}"));
        bot.is_bot = true;
        bot.position = (10.0 + i as f32, 0.0, 70.0);
        players.insert(addr, bot);
        bots.push(addr);
    }
    let fired: Vec<(PeerId, u32)> = bots.iter().map(|b| (*b, 1)).collect();
    crate::bot::manager::send_bot_shots(&outbox, &players, &fired);
    let out = outbox.drain();
    assert_eq!(out.len(), 2, "one packet per human, none for bots");
    for peer in [human, other_human] {
        let msgs = to(&out, peer);
        let [ServerMsg::BotShots(shots)] = msgs.as_slice() else { panic!("not one bot-shot batch: {msgs:?}") };
        assert_eq!(shots.len(), 3);
        assert_eq!(shots[2].shooter_id, 9002);
        assert_eq!(shots[2].start, (12.0, round_to_grid(1.2), 70.0));
        assert_eq!(shots[2].end, (0.0, 1.0, 0.0));
    }
    crate::bot::manager::send_bot_shots(&outbox, &players, &[]);
    assert!(outbox.drain().is_empty());
}

fn round_to_grid(v: f32) -> f32 {
    protocol::codec::dequantize_coord(protocol::codec::quantize_coord(v))
}
