// tests for hooks.rs, the round plugged into the server

use super::*;
use crate::server::smoke_break::{is_decoy, SmokeState, SMOKE_BLIND_TIME, SMOKE_RELEASED_TIME};

// two people joined and standing in a smoke break match, first tick not run yet
fn match_of_two() -> (Server, PeerId, PeerId) {
    let mut server = Server::new();
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
    (server, a, b)
}

fn messages(out: &[(PeerId, Vec<u8>)], peer: PeerId) -> Vec<ServerMsg> {
    out.iter().filter(|(p, _)| *p == peer).filter_map(|(_, m)| ServerMsg::decode(m)).collect()
}

#[test]
fn the_first_tick_starts_round_1_and_moves_everyone() {
    let (mut server, a, b) = match_of_two();
    server.tick();
    let smoke = server.smoke_state.as_ref().unwrap();
    assert_eq!(smoke.round, 1);
    assert!(server.players.keys().any(|p| is_decoy(*p)), "the crowd is out");
    let out = server.drain_outbox();
    for peer in [a, b] {
        let got = messages(&out, peer);
        assert!(got.iter().any(|m| matches!(m, ServerMsg::Respawn { .. })), "{peer} wasn't moved");
        assert!(got.iter().any(|m| matches!(m, ServerMsg::SmokeRound(_))), "{peer} got no round");
    }
}

#[test]
fn coins_only_ever_reach_hiders() {
    let (mut server, a, b) = match_of_two();
    server.tick();
    {
        let smoke = server.smoke_state.as_mut().unwrap();
        smoke.tick(SMOKE_BLIND_TIME);
        smoke.tick(SMOKE_RELEASED_TIME);
    }
    server.smoke_last_send = None;
    server.drain_outbox();
    server.tick();
    let out = server.drain_outbox();
    let smoke = server.smoke_state.as_ref().unwrap();
    let hunter = *smoke.hunters.iter().next().unwrap();
    let hider = if hunter == a { b } else { a };
    let coins = |peer| {
        messages(&out, peer)
            .into_iter()
            .filter_map(|m| if let ServerMsg::SmokeCoins(c) = m { Some(c) } else { None })
            .last()
            .expect("no coin message")
    };
    assert!(coins(hunter).is_empty());
    // the hider may have just grabbed one they were standing on, so compare with what's actually out
    assert!(!smoke.coins.is_empty());
    assert_eq!(coins(hider).len(), smoke.coins.len());
}

#[test]
fn alone_the_match_ends_with_a_notice() {
    let mut server = Server::new();
    server.handle_packet(PeerId(1), &ClientMsg::join("alice").encode());
    server.players.get_mut(&PeerId(1)).unwrap().world = WorldId::Maze;
    server.match_mode = MODE_SMOKE_BREAK;
    server.match_running = true;
    server.smoke_state = Some(SmokeState::new());
    server.tick();
    assert!(!server.match_running);
    assert!(server.match_end_pending.is_some());
}

#[test]
fn one_left_and_the_match_ends_with_a_winner() {
    let (mut server, _, _) = match_of_two();
    server.tick();
    let hider = *server.smoke_state.as_ref().unwrap().hiders.iter().next().unwrap();
    server.players.remove(&hider);
    server.tick();
    assert!(!server.match_running);
    assert!(server.match_end_pending.is_some(), "the winner screen");
}

#[test]
fn every_hunter_leaving_gives_the_hiders_the_round() {
    let (mut server, _, _) = match_of_two();
    server.handle_packet(PeerId(3), &ClientMsg::join("cara").encode());
    server.players.get_mut(&PeerId(3)).unwrap().world = WorldId::Maze;
    server.tick();
    let hunter = *server.smoke_state.as_ref().unwrap().hunters.iter().next().unwrap();
    server.players.remove(&hunter);
    server.tick();
    let smoke = server.smoke_state.as_ref().unwrap();
    assert_eq!((smoke.phase, smoke.winner), (SmokePhase::Results, SMOKE_SIDE_HIDER));
}

#[test]
fn after_the_results_the_next_round_starts_on_its_own() {
    let (mut server, _, _) = match_of_two();
    server.tick();
    {
        let smoke = server.smoke_state.as_mut().unwrap();
        smoke.end_round(SMOKE_SIDE_HIDER);
        smoke.phase_timer = 0.0;
    }
    server.tick();
    let smoke = server.smoke_state.as_ref().unwrap();
    assert_eq!((smoke.round, smoke.phase), (2, SmokePhase::Blind));
}

#[test]
fn the_last_round_ends_the_match() {
    let (mut server, _, _) = match_of_two();
    server.tick();
    {
        let smoke = server.smoke_state.as_mut().unwrap();
        smoke.max_rounds = 1;
        smoke.end_round(SMOKE_SIDE_HIDER);
        smoke.phase_timer = 0.0;
    }
    server.tick();
    assert!(server.match_end_pending.is_some());
}

#[test]
fn the_first_round_takes_the_lobbys_round_count() {
    let (mut server, _, _) = match_of_two();
    server.smoke_rounds = 10;
    server.tick();
    assert_eq!(server.smoke_state.as_ref().unwrap().max_rounds, 10);
}

#[test]
fn the_host_ending_it_shows_the_winner_first() {
    let (mut server, a, _) = match_of_two();
    server.tick();
    server.handle_packet(a, &ClientMsg::EndMatch.encode());
    assert!(server.match_end_pending.is_some(), "the winner screen, not straight home");
}

#[test]
fn nobody_takes_damage_and_nobody_climbs_out() {
    let (mut server, a, b) = match_of_two();
    server.tick();
    let hp = server.players[&b].health;
    let hit = ClientMsg::Hit { victim_id: server.players[&b].id, damage: 50.0, weapon: None, backstab: None, headshot: None };
    server.handle_packet(a, &hit.encode());
    assert_eq!(server.players[&b].health, hp, "no damage in smoke break");
    let hider = *server.smoke_state.as_ref().unwrap().hiders.iter().next().unwrap();
    server.handle_packet(hider, &ClientMsg::BoxExit.encode());
    server.tick();
    assert!(server.boxes.is_carrying(hider), "hiders can't leave their box");
}

#[test]
fn when_the_match_ends_the_crowd_never_reaches_the_lobby() {
    let (mut server, _, _) = match_of_two();
    server.tick();
    server.handle_packet(PeerId(1), &ClientMsg::EndMatch.encode());
    // the winner screen's countdown runs out
    if let Some(end) = server.match_end_pending.as_mut() {
        end.0 = Instant::now();
    }
    // one tick sends everyone home, the next clears the smoke break away
    server.tick();
    server.tick();
    assert!(server.players.keys().all(|p| !is_decoy(*p)), "the decoys are gone");
    assert!(server.smoke_state.is_none());
}
