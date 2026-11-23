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

fn messages(out: &[(PeerId, crate::net::Packet)], peer: PeerId) -> Vec<ServerMsg> {
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

// the server list counts people and bots, never the decoy crowd
#[test]
fn decoys_are_not_counted_as_players_or_bots() {
    let (mut server, _, _) = match_of_two();
    server.tick();
    assert!(server.players.keys().any(|p| is_decoy(*p)), "the crowd is out");
    assert_eq!((server.player_count(), server.bot_count()), (2, 0));
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
fn on_the_winner_screen_box_packets_still_follow_smoke_rules() {
    let (mut server, a, _) = match_of_two();
    server.tick();
    {
        let smoke = server.smoke_state.as_mut().unwrap();
        smoke.tick(SMOKE_BLIND_TIME);
    }
    let smoke = server.smoke_state.as_ref().unwrap();
    let (hunter, hider) = (*smoke.hunters.iter().next().unwrap(), *smoke.hiders.iter().next().unwrap());
    server.handle_packet(a, &ClientMsg::EndMatch.encode());
    assert!(!server.match_running && server.smoke_state.is_some(), "the winner countdown");

    // the hider can't climb out, the hunter can't find them, nobody gets hurt
    server.handle_packet(hider, &ClientMsg::BoxExit.encode());
    server.tick();
    let id = server.boxes.box_of(hider).expect("still boxed");
    let points = server.smoke_state.as_ref().unwrap().points.clone();
    server.handle_packet(hunter, &ClientMsg::BoxShot { id }.encode());
    let hp = server.players[&hider].health;
    let hit = ClientMsg::Hit { victim_id: server.players[&hider].id, damage: 50.0, weapon: None, backstab: None, headshot: None };
    server.handle_packet(hunter, &hit.encode());
    server.tick();
    assert_eq!(server.boxes.box_of(hider), Some(id), "the box stays");
    let smoke = server.smoke_state.as_ref().unwrap();
    assert!(smoke.hiders.contains(&hider) && smoke.points == points, "no find after the match is over");
    assert_eq!(server.players[&hider].health, hp);
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

#[test]
fn a_refused_box_shot_sends_no_player_list() {
    let (mut server, _, _) = match_of_two();
    server.tick();
    let hider = *server.smoke_state.as_ref().unwrap().hiders.iter().next().unwrap();
    let id = server.boxes.box_of(hider).unwrap();
    server.drain_outbox();
    // hiders can't shoot, nothing changed
    server.handle_packet(hider, &ClientMsg::BoxShot { id }.encode());
    let out = server.drain_outbox();
    assert!(!out.iter().any(|(_, m)| matches!(ServerMsg::decode(m), Some(ServerMsg::PlayerList(_)))));
}

// released, a decoy put where `at` says and its box moved with it, the hunter and that box
fn hunter_and_decoy_at(server: &mut Server, hunter_at: (f32, f32), decoy_at: (f32, f32)) -> (PeerId, u16) {
    server.smoke_state.as_mut().unwrap().tick(SMOKE_BLIND_TIME);
    let hunter = *server.smoke_state.as_ref().unwrap().hunters.iter().next().unwrap();
    let decoy = *server.players.keys().find(|p| is_decoy(**p)).unwrap();
    server.players.get_mut(&hunter).unwrap().position = (hunter_at.0, 0.0, hunter_at.1);
    server.players.get_mut(&decoy).unwrap().position = (decoy_at.0, 0.0, decoy_at.1);
    server.sync_boxes();
    (hunter, server.boxes.box_of(decoy).unwrap())
}

fn shoot(server: &mut Server, hunter: PeerId, id: u16) -> bool {
    server.handle_packet(hunter, &ClientMsg::BoxShot { id }.encode());
    server.boxes.carrier_of(id).is_none()
}

#[test]
fn a_box_shot_has_to_be_one_the_hunter_could_make() {
    let (mut server, _, _) = match_of_two();
    server.tick();
    let maze = server.maze.as_ref().unwrap();
    let (cells, width, height) = (maze.cells.clone(), maze.width as usize, maze.height as usize);
    let size = crate::server::game::MAZE_CELL_SIZE;
    let open = |x: usize, z: usize| cells[z * width + x] != 0;
    // open floor, then a wall, then open floor again further along the same row, well in range
    let (x0, x1, z) = (0..height)
        .flat_map(|z| (0..width - 1).map(move |x| (x, z)))
        .filter(|&(x, z)| open(x, z) && !open(x + 1, z))
        .find_map(|(x, z)| (x + 2..width).find(|&x1| open(x1, z)).map(|x1| (x, x1, z)))
        .filter(|&(x0, x1, _)| (x1 - x0) as f32 * size < 40.0)
        .expect("no wall to hide behind");
    let mid = |x: usize| (x as f32 + 0.5) * size;
    let (left, right) = ((mid(x0), mid(z)), (mid(x1), mid(z)));
    // where the round put the hunter, the middle of a room
    let hunter = *server.smoke_state.as_ref().unwrap().hunters.iter().next().unwrap();
    let hunter_at = server.players[&hunter].position;
    let (hunter, id) = hunter_and_decoy_at(&mut server, left, right);
    assert!(!shoot(&mut server, hunter, id), "through a wall");

    // in the open, close by: fine, once, but not again straight away
    let near = (hunter_at.0 + 1.5, hunter_at.2);
    let decoy = server.players.iter().find(|(p, _)| is_decoy(**p) && server.boxes.box_of(**p) != Some(id)).map(|(p, _)| *p).unwrap();
    server.players.get_mut(&decoy).unwrap().position = (near.0, 0.0, near.1);
    // too far first
    server.players.get_mut(&hunter).unwrap().position = (near.0 - 60.0, 0.0, near.1);
    server.sync_boxes();
    let near_id = server.boxes.box_of(decoy).unwrap();
    assert!(!shoot(&mut server, hunter, near_id), "out of the deagle's range");
    server.players.get_mut(&hunter).unwrap().position = (near.0 - 1.5, 0.0, near.1);
    // not with anything else
    server.players.get_mut(&hunter).unwrap().current_weapon = "M4A1".into();
    assert!(!shoot(&mut server, hunter, near_id), "only the deagle");
    server.players.get_mut(&hunter).unwrap().current_weapon = HUNTER_WEAPON.into();
    assert!(shoot(&mut server, hunter, near_id), "a fair shot");

    let other = server.players.iter().find(|(p, _)| is_decoy(**p) && server.boxes.box_of(**p).is_some()).map(|(p, _)| *p).unwrap();
    server.players.get_mut(&other).unwrap().position = (near.0, 0.0, near.1);
    server.sync_boxes();
    let other_id = server.boxes.box_of(other).unwrap();
    assert!(!shoot(&mut server, hunter, other_id), "faster than the deagle fires");
}

#[test]
fn a_boxed_hider_cant_teleport_and_gets_told_where_it_is() {
    let (mut server, _, _) = match_of_two();
    server.tick();
    let hider = *server.smoke_state.as_ref().unwrap().hiders.iter().next().unwrap();
    let start = server.players[&hider].position;
    server.drain_outbox();
    let far = (start.0 + 20.0, 0.0, start.2 + 20.0);
    let msg = ClientMsg::Position { position: far, yaw: 0.0, state_flags: None, event_flags: None, pitch: None };
    server.handle_packet(hider, &msg.encode());
    assert_eq!(server.players[&hider].position, start, "the box stayed");
    let told = messages(&server.drain_outbox(), hider)
        .into_iter()
        .any(|m| matches!(m, ServerMsg::Respawn { position, .. } if position == start));
    assert!(told, "their game is put back");
}
