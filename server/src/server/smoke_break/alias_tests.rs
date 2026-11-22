// tests for alias.rs: what a hunter's game gets must never say which box is a human

use std::collections::HashSet;

use super::*;
use crate::player::WorldId;
use crate::server::boxes::Boxes;
use crate::server::smoke_break::is_decoy;
use crate::server::Server;

// three people in a smoke break match, round 1 started: 1 hunter, 2 hiders, 36 decoys
fn round_of_three() -> (Server, PeerId, Vec<PeerId>) {
    let mut server = Server::new();
    for (i, name) in ["alice", "bob", "cara"].iter().enumerate() {
        server.handle_packet(PeerId(i as u32 + 1), &ClientMsg::join(*name).encode());
    }
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
    let mut hiders: Vec<PeerId> = smoke.hiders.iter().copied().collect();
    hiders.sort();
    (server, hunter, hiders)
}

fn last_state(out: &[(PeerId, Vec<u8>)], peer: PeerId) -> Vec<PlayerState> {
    out.iter()
        .filter(|(p, _)| *p == peer)
        .filter_map(|(_, m)| match ServerMsg::decode(m) {
            Some(ServerMsg::State { players, .. }) => Some(players),
            _ => None,
        })
        .last()
        .expect("no state")
}

fn last_boxes(out: &[(PeerId, Vec<u8>)], peer: PeerId) -> Vec<CardboardBox> {
    out.iter()
        .filter(|(p, _)| *p == peer)
        .filter_map(|(_, m)| match ServerMsg::decode(m) {
            Some(ServerMsg::Boxes { boxes, .. }) => Some(boxes),
            _ => None,
        })
        .last()
        .expect("no box list")
}

fn infos(out: &[(PeerId, Vec<u8>)], peer: PeerId) -> Vec<PlayerInfo> {
    out.iter()
        .filter(|(p, _)| *p == peer)
        .filter_map(|(_, m)| match ServerMsg::decode(m) {
            Some(ServerMsg::PlayerInfo(list)) => Some(list),
            _ => None,
        })
        .flatten()
        .collect()
}

#[test]
fn a_hunter_never_gets_a_hiders_real_id_next_to_a_box() {
    let (mut server, hunter, hiders) = round_of_three();
    let out = server.drain_outbox();
    let real: HashSet<u32> = hiders.iter().map(|h| server.players[h].id).collect();
    let decoy_ids: HashSet<u32> = server.players.iter().filter(|(p, _)| is_decoy(**p)).map(|(_, p)| p.id).collect();

    let state = last_state(&out, hunter);
    let boxes = last_boxes(&out, hunter);
    for e in &state {
        assert!(!real.contains(&e.id), "a hider's real id in the hunter's state");
        assert!(!decoy_ids.contains(&e.id), "a decoy's real id in the hunter's state");
        assert_eq!(e.state_flags & FLAG_IS_BOT, 0, "no bot flag on anyone");
    }
    for b in &boxes {
        assert!(!real.contains(&b.carrier) && !decoy_ids.contains(&b.carrier), "a real carrier id");
    }
    // every box is carried by an alias that's also in the state
    let aliases: HashSet<u32> = state.iter().map(|e| e.id).filter(|id| *id >= SMOKE_ALIAS_BASE).collect();
    assert_eq!(aliases.len(), 2 + super::super::round_start::SMOKE_DECOYS);
    assert!(boxes.iter().all(|b| aliases.contains(&b.carrier)));
    // their info says nothing either, and a decoy's real id never goes out
    let info = infos(&out, hunter);
    assert!(info.iter().all(|i| !decoy_ids.contains(&i.id)));
    for alias in &aliases {
        assert_eq!(info.iter().find(|i| i.id == *alias), Some(&masked_info(*alias)));
    }
}

#[test]
fn a_hider_and_a_decoy_in_the_same_spot_look_the_same() {
    let (mut server, hunter, hiders) = round_of_three();
    let hider = hiders[0];
    let decoy = *server.players.keys().find(|p| is_decoy(**p)).unwrap();
    // the hider's game sends all sorts, the decoy has none of it
    let (at, yaw) = ((12.0, 0.0, 9.0), 1.0);
    {
        let h = server.players.get_mut(&hider).unwrap();
        h.position = (at.0, 0.4, at.2);
        h.yaw = yaw + TAU * 3.0;
        h.pitch = 0.6;
        h.is_sprinting = true;
        h.is_crouched = true;
        h.is_ads = true;
        h.is_airborne = true;
        h.evt_jump_start = true;
        // up, a drop would break the box
        h.health = 200;
    }
    let d = server.players.get_mut(&decoy).unwrap();
    d.position = at;
    d.yaw = yaw;
    server.drain_outbox();
    crate::server::broadcast::broadcast_state_with_events(&server.outbox, &server.players);
    for peer in server.players.keys().copied().collect::<Vec<_>>() {
        server.boxes.resend_to(peer);
    }
    server.sync_boxes();
    let out = server.drain_outbox();

    let alias = |p: PeerId| server.players[&p].smoke_alias.unwrap();
    let state = last_state(&out, hunter);
    let entry = |id: u32| PlayerState { id: 0, ..*state.iter().find(|e| e.id == id).unwrap() };
    assert_eq!(entry(alias(hider)), entry(alias(decoy)));
    let boxes = last_boxes(&out, hunter);
    let of = |id: u32| {
        let b = boxes.iter().find(|b| b.carrier == id).unwrap();
        CardboardBox { id: 0, carrier: 0, ..*b }
    };
    assert_eq!(of(alias(hider)), of(alias(decoy)));
}

#[test]
fn a_hider_still_sees_itself_under_its_real_id() {
    let (mut server, _, hiders) = round_of_three();
    let out = server.drain_outbox();
    let (me, other) = (hiders[0], hiders[1]);
    let (my_id, other_id) = (server.players[&me].id, server.players[&other].id);
    let state = last_state(&out, me);
    assert!(state.iter().any(|e| e.id == my_id), "my own entry");
    assert!(!state.iter().any(|e| e.id == other_id), "the other hider is an alias to me too");
    let boxes = last_boxes(&out, me);
    assert_eq!(boxes.iter().filter(|b| b.carrier == my_id).count(), 1, "my box, under my id");
    assert!(!boxes.iter().any(|b| b.carrier == other_id));
}

#[test]
fn a_found_hider_shows_up_under_their_real_id() {
    let (mut server, hunter, hiders) = round_of_three();
    server.smoke_state.as_mut().unwrap().tick(super::super::SMOKE_BLIND_TIME);
    let id = server.boxes.box_of(hiders[0]).unwrap();
    server.handle_packet(hunter, &ClientMsg::BoxShot { id }.encode());
    server.tick();
    server.drain_outbox();
    crate::server::broadcast::broadcast_state(&server.outbox, &server.players);
    let out = server.drain_outbox();
    let real = server.players[&hiders[0]].id;
    assert!(server.players[&hiders[0]].smoke_alias.is_none());
    assert!(last_state(&out, hunter).iter().any(|e| e.id == real), "found, out of the box");
}

#[test]
fn box_ids_say_nothing_about_who_is_inside() {
    // hider boxes used to be spawned first, so the lowest ids were the humans
    let mut players: std::collections::HashMap<PeerId, Player> = (1..=3)
        .map(|i| {
            let mut p = Player::new(i, format!("p{i}"));
            p.world = WorldId::Maze;
            (PeerId(i), p)
        })
        .collect();
    let (mut s, mut boxes) = (SmokeState::new(), Boxes::new());
    let mut hiders_not_lowest = false;
    for _ in 0..20 {
        s.start_round(&mut players, &mut boxes);
        let mut ids: Vec<u16> = players.keys().filter_map(|p| boxes.box_of(*p)).collect();
        ids.sort();
        let lowest = &ids[..s.hiders.len()];
        if s.hiders.iter().any(|h| !lowest.contains(&boxes.box_of(*h).unwrap())) {
            hiders_not_lowest = true;
        }
    }
    assert!(hiders_not_lowest, "hiders always had the lowest box ids");
}
