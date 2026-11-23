use super::*;
use crate::net::Outbox;
use crate::player::WorldId;
use crate::protocols::{ClientMsg, ServerMsg};

/// `handle_shoot` into a queue, flushed at once as the next bot tick would.
fn shoot(outbox: &Outbox, players: &mut HashMap<PeerId, Player>, src: PeerId, end: (f32, f32, f32), now: Instant) {
    let mut queue = ShotQueue::default();
    handle_shoot(&mut queue, players, src, end, now);
    queue.flush(outbox, players);
}

/// What the server's queued shots send, as its next bot tick would.
fn flush(server: &mut crate::server::Server) -> Vec<(PeerId, crate::net::Packet)> {
    server.shot_queue.flush(&server.outbox, &server.players);
    server.drain_outbox()
}

fn shots_to(out: &[(PeerId, crate::net::Packet)], peer: PeerId) -> Vec<Shot> {
    out.iter()
        .filter(|(p, _)| *p == peer)
        .filter_map(|(_, b)| match ServerMsg::decode(b) {
            Some(ServerMsg::Shots(shots)) => Some(shots),
            _ => None,
        })
        .flatten()
        .collect()
}

/// A and B in the maze, C in the lobby, and a bot in the maze.
fn three_players() -> (HashMap<PeerId, Player>, [PeerId; 4]) {
    let ids = [PeerId(1), PeerId(2), PeerId(3), PeerId(crate::net::BOT_PEER_BASE)];
    let mut players = HashMap::new();
    for (i, addr) in ids.iter().enumerate() {
        let mut p = Player::new(i as u32 + 1, format!("p{i}"));
        p.world = if i == 2 { WorldId::Lobby } else { WorldId::Maze };
        p.is_bot = i == 3;
        p.current_weapon = "M4A1".into();
        players.insert(*addr, p);
    }
    players.get_mut(&ids[0]).unwrap().position = (10.0, 0.0, 20.0);
    (players, ids)
}

#[test]
fn a_shot_reaches_the_humans_in_the_shooters_world_only() {
    let outbox = Outbox::default();
    let (mut players, [a, b, c, bot]) = three_players();
    // the client claims nothing about the start: it comes from A's position
    shoot(&outbox, &mut players, a, (10.0, 1.0, 40.0), Instant::now());
    let out = outbox.drain();
    let [shot] = shots_to(&out, b)[..] else { panic!("B got no single shot: {out:?}") };
    assert_eq!(shot.shooter_id, 1);
    assert_eq!(shot.victim_id, 0);
    let gun = protocol::codec::dequantize_coord(protocol::codec::quantize_coord(SHOT_START_HEIGHT));
    assert_eq!(shot.start, (10.0, gun, 20.0));
    assert_eq!(shot.end, (10.0, 1.0, 40.0));
    // A gets the same shared packet (its client drops its own shot)
    assert_eq!(shots_to(&out, a), vec![shot]);
    for peer in [c, bot] {
        assert!(out.iter().all(|(p, _)| *p != peer), "{peer:?} got the shot");
    }
}

#[test]
fn shots_are_dropped_faster_than_the_weapon_fires() {
    let outbox = Outbox::default();
    let (mut players, [a, b, ..]) = three_players();
    let t0 = Instant::now();
    let interval = min_shot_interval("M4A1");
    shoot(&outbox, &mut players, a, (0.0, 1.0, 0.0), t0);
    shoot(&outbox, &mut players, a, (0.0, 1.0, 0.0), t0 + interval / 2);
    assert_eq!(shots_to(&outbox.drain(), b).len(), 1, "the second shot came too soon");
    shoot(&outbox, &mut players, a, (0.0, 1.0, 0.0), t0 + interval);
    assert_eq!(shots_to(&outbox.drain(), b).len(), 1, "a shot at the fire rate is relayed");

    // a minigun may fire faster than a sniper rifle
    assert!(min_shot_interval("Minigun") < min_shot_interval("Bolt Action"));
    assert!(min_shot_interval("Minigun") < Duration::from_millis(50));
}

#[test]
fn the_dead_and_bad_end_points_shoot_nothing_and_far_ends_are_capped() {
    let outbox = Outbox::default();
    let (mut players, [a, b, ..]) = three_players();
    players.get_mut(&a).unwrap().is_alive = false;
    shoot(&outbox, &mut players, a, (0.0, 1.0, 0.0), Instant::now());
    assert!(outbox.drain().is_empty(), "a dead player fired");

    players.get_mut(&a).unwrap().is_alive = true;
    shoot(&outbox, &mut players, a, (f32::NAN, 1.0, 0.0), Instant::now());
    assert!(outbox.drain().is_empty(), "a NaN end was relayed");

    shoot(&outbox, &mut players, a, (10.0, 1.2, 20.0 + 10_000.0), Instant::now());
    let [shot] = shots_to(&outbox.drain(), b)[..] else { panic!("no capped shot") };
    assert!((shot.end.2 - (20.0 + MAX_SHOT_DISTANCE)).abs() < 0.05, "{:?}", shot.end);
}

#[test]
fn a_shoot_packet_is_dispatched_to_the_relay() {
    let mut server = crate::server::Server::new();
    let (a, b) = (PeerId(1), PeerId(2));
    server.handle_packet(a, &ClientMsg::join("alice").encode());
    server.handle_packet(b, &ClientMsg::join("bob").encode());
    server.drain_outbox();
    server.handle_packet(a, &ClientMsg::Shoot { end: (1.0, 1.0, 5.0) }.encode());
    assert!(shots_to(&server.drain_outbox(), b).is_empty(), "a shot went out before the bot tick");
    let out = flush(&mut server);
    assert_eq!(shots_to(&out, b).len(), 1, "bob (same lobby) saw no shot");
}

#[test]
fn a_box_under_a_smoke_alias_never_shows_its_shot() {
    let outbox = Outbox::default();
    let (mut players, [a, ..]) = three_players();
    players.get_mut(&a).unwrap().smoke_alias = Some(0x4000_0000);
    shoot(&outbox, &mut players, a, (0.0, 1.0, 0.0), Instant::now());
    assert!(outbox.drain().is_empty(), "an aliased box's real id went out with a shot");
}

// in Smoke Break only a hunter who may fire is seen firing: never a hider, never while blind
#[test]
fn in_smoke_break_only_a_released_hunters_shot_is_relayed() {
    use crate::protocols::MODE_SMOKE_BREAK;
    use crate::server::smoke_break::{SmokeState, SMOKE_BLIND_TIME};
    let mut server = crate::server::Server::new();
    let (a, b) = (PeerId(1), PeerId(2));
    server.handle_packet(a, &ClientMsg::join("alice").encode());
    server.handle_packet(b, &ClientMsg::join("bob").encode());
    for p in server.players.values_mut() {
        p.world = WorldId::Maze;
    }
    server.match_mode = MODE_SMOKE_BREAK;
    server.match_running = true;
    server.maze = Some(crate::maze::MazeData::smoke_break_arena());
    server.smoke_state = Some(SmokeState::new());
    server.tick();
    let smoke = server.smoke_state.as_ref().unwrap();
    let hunter = *smoke.hunters.iter().next().unwrap();
    let hider = if hunter == a { b } else { a };
    server.drain_outbox();
    let shoot = ClientMsg::Shoot { end: (1.0, 1.0, 5.0) }.encode();

    server.handle_packet(hunter, &shoot);
    assert!(shots_to(&flush(&mut server), hider).is_empty(), "a blind hunter was seen firing");

    server.smoke_state.as_mut().unwrap().tick(SMOKE_BLIND_TIME);
    server.handle_packet(hider, &shoot);
    let out = flush(&mut server);
    assert!(shots_to(&out, hunter).is_empty(), "a hider's shot gave their box away");

    server.handle_packet(hunter, &shoot);
    assert_eq!(shots_to(&flush(&mut server), hider).len(), 1, "the hider never saw the hunter fire");
}

/// The `MSG_SHOTS` packets in `out` for `peer`, each as its shot count.
fn shot_packets_to(out: &[(PeerId, crate::net::Packet)], peer: PeerId) -> Vec<usize> {
    out.iter()
        .filter(|(p, _)| *p == peer)
        .filter_map(|(_, b)| match ServerMsg::decode(b) {
            Some(ServerMsg::Shots(shots)) => Some(shots.len()),
            _ => None,
        })
        .collect()
}

// players' shots wait for the bot tick and go out together: one packet per world per tick
#[test]
fn players_shots_go_out_once_per_tick_in_one_packet_per_world() {
    let mut server = crate::server::Server::new();
    let (a, b, c) = (PeerId(1), PeerId(2), PeerId(3));
    for (peer, name) in [(a, "alice"), (b, "bob"), (c, "carol")] {
        server.handle_packet(peer, &ClientMsg::join(name).encode());
    }
    for peer in [a, b] {
        server.players.get_mut(&peer).unwrap().world = WorldId::Maze;
    }
    server.drain_outbox();
    let shoot = ClientMsg::Shoot { end: (1.0, 1.0, 5.0) }.encode();
    for peer in [a, b, c] {
        server.handle_packet(peer, &shoot);
    }
    assert!(server.drain_outbox().is_empty(), "a shot went out before the tick");
    // the first bot tick is always due
    server.tick();
    let out = server.drain_outbox();
    assert_eq!(shot_packets_to(&out, a), vec![2], "the maze's two shots, in one packet");
    assert_eq!(shot_packets_to(&out, b), vec![2]);
    assert_eq!(shot_packets_to(&out, c), vec![1], "the lobby's own shot, apart");
}

#[test]
fn a_full_queue_is_split_at_the_packet_limit() {
    use crate::server::broadcast::MAX_SHOTS_PER_PACKET;
    let outbox = Outbox::default();
    let (players, [a, b, ..]) = three_players();
    let mut queue = ShotQueue::default();
    let shot = Shot { shooter_id: 1, victim_id: 0, start: (0.0, 1.2, 0.0), end: (0.0, 1.0, 5.0) };
    for _ in 0..MAX_SHOTS_PER_PACKET + 3 {
        queue.push(WorldId::Maze, shot);
    }
    queue.flush(&outbox, &players);
    let out = outbox.drain();
    assert_eq!(shot_packets_to(&out, b), vec![MAX_SHOTS_PER_PACKET, 3]);
    assert_eq!(shot_packets_to(&out, a), vec![MAX_SHOTS_PER_PACKET, 3]);
    queue.flush(&outbox, &players);
    assert!(outbox.drain().is_empty(), "a flushed queue sent again");
}
