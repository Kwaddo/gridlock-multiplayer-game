use super::*;
use crate::player::Team;
use crate::player::WorldId;

fn to(out: &[(PeerId, Vec<u8>)], peer: PeerId) -> Vec<ServerMsg> {
    out.iter().filter(|(p, _)| *p == peer).filter_map(|(_, b)| ServerMsg::decode(b)).collect()
}

fn start_match(server: &mut Server, host: PeerId) {
    let start = ClientMsg::StartGame { mode: Some(MODE_TEAM_DEATHMATCH), duration: Some(0), autofill: None, blue_bots: None };
    server.handle_packet(host, &start.encode());
    // skip the warmup countdown
    server.match_warmup_end = Some(Instant::now());
    server.tick();
    assert!(server.match_running);
}

#[test]
fn host_ends_match_and_everyone_stays_connected() {
    let mut server = Server::new();
    let (a, b) = (PeerId(1), PeerId(2));
    server.handle_packet(a, &ClientMsg::join("alice").encode());
    server.handle_packet(b, &ClientMsg::join("bob").encode());
    start_match(&mut server, a);
    server.drain_outbox();

    // only the host may end it
    server.handle_packet(b, &ClientMsg::EndMatch.encode());
    assert!(server.match_running);
    assert!(!to(&server.drain_outbox(), b).iter().any(|m| matches!(m, ServerMsg::WorldChange { .. })));

    server.handle_packet(a, &ClientMsg::EndMatch.encode());
    let out = server.drain_outbox();
    for peer in [a, b] {
        let msgs = to(&out, peer);
        assert!(msgs.iter().any(|m| matches!(m, ServerMsg::WorldChange { world: 0, .. })));
        assert!(msgs.iter().any(|m| matches!(m, ServerMsg::Chat { from_id: 0, .. })));
    }
    assert!(!server.match_running && server.maze.is_none());
    assert!(server.has_peer(a) && server.has_peer(b));

    // the lobby is still up: a new match can start straight away
    start_match(&mut server, a);
}

#[test]
fn match_ends_once_every_human_has_left_it() {
    let mut server = Server::new();
    let (a, b) = (PeerId(1), PeerId(2));
    server.handle_packet(a, &ClientMsg::join("alice").encode());
    server.handle_packet(b, &ClientMsg::join("bob").encode());
    start_match(&mut server, a);
    server.drain_outbox();

    let lobby = ClientMsg::WorldChange { target: WorldId::Lobby.as_u8() };
    server.handle_packet(b, &lobby.encode());
    server.tick();
    assert!(server.match_running, "alice is still playing");
    let notice = ServerMsg::Chat { from_id: 0, name: String::new(), text: "bob left the match".into() };
    assert!(to(&server.drain_outbox(), a).contains(&notice));

    server.handle_packet(a, &lobby.encode());
    server.tick();
    assert!(!server.match_running && server.maze.is_none());
}

fn statuses(out: &[(PeerId, Vec<u8>)], peer: PeerId) -> Vec<u8> {
    to(out, peer)
        .into_iter()
        .filter_map(|m| match m {
            ServerMsg::MatchStatus { status } => Some(status),
            _ => None,
        })
        .collect()
}

fn two_players() -> (Server, PeerId, PeerId) {
    let mut server = Server::new();
    let (a, b) = (PeerId(1), PeerId(2));
    server.handle_packet(a, &ClientMsg::join("alice").encode());
    server.handle_packet(b, &ClientMsg::join("bob").encode());
    (server, a, b)
}

#[test]
fn host_stays_host_while_in_the_match() {
    let (mut server, a, b) = two_players();
    start_match(&mut server, a);
    // bob goes back to the lobby; alice, in the maze, is still the host
    server.handle_packet(b, &ClientMsg::WorldChange { target: WorldId::Lobby.as_u8() }.encode());
    let leaders: Vec<u32> = to(&server.drain_outbox(), b)
        .into_iter()
        .filter_map(|m| match m {
            ServerMsg::LobbyLeader { id } => Some(id),
            _ => None,
        })
        .collect();
    assert!(!leaders.is_empty());
    assert!(leaders.iter().all(|id| *id == 1), "host changed: {leaders:?}");
}

#[test]
fn anyone_in_the_lobby_can_join_a_running_match_without_changing_it() {
    let (mut server, a, b) = two_players();
    start_match(&mut server, a);
    let (mode, duration) = (server.match_mode, server.match_duration);
    server.handle_packet(b, &ClientMsg::WorldChange { target: WorldId::Lobby.as_u8() }.encode());
    server.drain_outbox();

    // bob (not the host) rejoins, asking for other settings
    let start = ClientMsg::StartGame {
        mode: Some(MODE_GUN_GAME),
        duration: Some(30),
        autofill: Some((8, 0b1)),
        blue_bots: None,
    };
    server.handle_packet(b, &start.encode());
    let out = server.drain_outbox();
    assert!(to(&out, b).iter().any(|m| matches!(m, ServerMsg::WorldChange { world: 1, .. })));
    assert_eq!(server.players[&b].world, WorldId::Maze);
    assert_eq!((server.match_mode, server.match_duration), (mode, duration));
    assert_eq!((server.autofill_red_bots, server.autofill_blue_bots), (0, 0));
}

#[test]
fn start_requests_while_starting_or_ending_are_refused_with_a_reason() {
    let (mut server, a, b) = two_players();
    let start = ClientMsg::StartGame { mode: Some(MODE_TEAM_DEATHMATCH), duration: Some(0), autofill: None, blue_bots: None };
    server.handle_packet(a, &start.encode());
    server.drain_outbox();
    server.handle_packet(b, &start.encode());
    assert!(to(&server.drain_outbox(), b)
        .contains(&ServerMsg::StartDenied { reason: Some(START_DENIED_STARTING) }));

    server.match_warmup_end = Some(Instant::now());
    server.tick();
    server.match_end_pending = Some((Instant::now() + Duration::from_secs(60), 1, "alice".into()));
    server.handle_packet(b, &ClientMsg::WorldChange { target: WorldId::Lobby.as_u8() }.encode());
    server.drain_outbox();
    server.handle_packet(b, &start.encode());
    assert!(to(&server.drain_outbox(), b)
        .contains(&ServerMsg::StartDenied { reason: Some(START_DENIED_ENDING) }));
    assert_eq!(server.players[&b].world, WorldId::Lobby);
}

#[test]
fn match_status_follows_the_match() {
    let (mut server, a, b) = two_players();
    assert_eq!(statuses(&server.drain_outbox(), b).last(), Some(&MATCH_STATUS_NONE));

    let start = ClientMsg::StartGame { mode: Some(MODE_TEAM_DEATHMATCH), duration: Some(0), autofill: None, blue_bots: None };
    server.handle_packet(a, &start.encode());
    assert_eq!(statuses(&server.drain_outbox(), b).last(), Some(&MATCH_STATUS_STARTING));

    server.match_warmup_end = Some(Instant::now());
    server.tick();
    assert_eq!(statuses(&server.drain_outbox(), b).last(), Some(&MATCH_STATUS_RUNNING));

    server.handle_packet(a, &ClientMsg::EndMatch.encode());
    assert_eq!(statuses(&server.drain_outbox(), b).last(), Some(&MATCH_STATUS_NONE));
}

#[test]
fn team_score_counts_kills_as_they_happen_and_keeps_them() {
    let (mut server, a, b) = two_players();
    start_match(&mut server, a);
    let (alice, bob) = (server.players[&a].team, server.players[&b].team);
    assert_ne!(alice, bob, "TDM puts the two on opposite teams");
    server.players.get_mut(&a).unwrap().post_teleport_freeze_until = None;
    server.players.get_mut(&b).unwrap().post_teleport_freeze_until = None;
    server.drain_outbox();

    let kill = ClientMsg::Hit { victim_id: 2, damage: 500.0, weapon: Some("AK-47".into()), backstab: None };
    server.handle_packet(a, &kill.encode());
    server.tick();
    let expected = if alice == crate::player::Team::Red {
        game::TeamScore { red: 1, blue: 0 }
    } else {
        game::TeamScore { red: 0, blue: 1 }
    };
    assert_eq!(server.team_score, expected);
    let (red, blue) = (expected.red as u16, expected.blue as u16);
    let sent = ServerMsg::TeamScore { red, blue, to_win: game::MATCH_SCORE_TO_WIN as u16 };
    assert!(to(&server.drain_outbox(), b).contains(&sent));

    // alice leaving the match doesn't take her kill off her team
    server.handle_packet(a, &ClientMsg::WorldChange { target: WorldId::Lobby.as_u8() }.encode());
    server.tick();
    assert_eq!(server.team_score, expected);
}

#[test]
fn team_kills_score_nothing() {
    let mut score = game::TeamScore::default();
    score.record_kill(crate::player::Team::Red, crate::player::Team::Red);
    score.record_kill(crate::player::Team::Spectator, crate::player::Team::Blue);
    assert_eq!(score, game::TeamScore::default());
    score.record_kill(crate::player::Team::Blue, crate::player::Team::Red);
    assert_eq!(score, game::TeamScore { red: 0, blue: 1 });
}

fn teams(server: &Server) -> Vec<Team> {
    let mut players: Vec<&Player> = server.players.values().filter(|p| !p.is_bot).collect();
    players.sort_by_key(|p| p.id);
    players.iter().map(|p| p.team).collect()
}

#[test]
fn lobby_teams_stay_balanced_and_only_the_host_can_swap() {
    let mut server = Server::new();
    let (a, b, c) = (PeerId(1), PeerId(2), PeerId(3));
    server.handle_packet(a, &ClientMsg::join("alice").encode());
    server.handle_packet(b, &ClientMsg::join("bob").encode());
    assert_eq!(teams(&server), vec![Team::Red, Team::Blue]);

    // bob (not the host) cannot change sides
    server.handle_packet(b, &ClientMsg::SetTeam { team: 1 }.encode());
    assert_eq!(teams(&server), vec![Team::Red, Team::Blue]);
    // the host cannot make it 2 v 0 either
    server.handle_packet(a, &ClientMsg::SetTeam { team: 2 }.encode());
    assert_eq!(teams(&server), vec![Team::Red, Team::Blue]);

    // with a third player, the host may swap to the smaller side
    server.handle_packet(c, &ClientMsg::join("cat").encode());
    assert_eq!(teams(&server), vec![Team::Red, Team::Blue, Team::Red]);
    server.handle_packet(a, &ClientMsg::SetTeam { team: 2 }.encode());
    assert_eq!(teams(&server), vec![Team::Blue, Team::Blue, Team::Red]);

    // whoever leaves, the rest are evened out again
    server.handle_packet(c, &ClientMsg::Disconnect.encode());
    server.tick();
    let t = teams(&server);
    assert!(t.contains(&Team::Red) && t.contains(&Team::Blue), "{t:?}");
}

#[test]
fn match_teams_are_the_balanced_lobby_teams() {
    let mut server = Server::new();
    let (a, b) = (PeerId(1), PeerId(2));
    server.handle_packet(a, &ClientMsg::join("alice").encode());
    server.handle_packet(b, &ClientMsg::join("bob").encode());
    start_match(&mut server, a);
    assert_eq!(teams(&server), vec![Team::Red, Team::Blue]);
}

#[test]
fn only_the_host_sets_the_lobby_settings_and_everyone_hears() {
    let mut server = Server::new();
    let (a, b) = (PeerId(1), PeerId(2));
    server.handle_packet(a, &ClientMsg::join("alice").encode());
    server.handle_packet(b, &ClientMsg::join("bob").encode());
    server.drain_outbox();

    let theirs = LobbySettings { mode: MODE_GUN_GAME, duration: 600, red_bots: 4, blue_bots: 0, diff_mask: 1 };
    server.handle_packet(b, &ClientMsg::LobbySettings(theirs).encode());
    assert!(server.drain_outbox().is_empty());

    // the host's goes to everyone, clamped to what the server accepts
    let mine = LobbySettings { mode: MODE_TEAM_DEATHMATCH, duration: 900, red_bots: 2, blue_bots: 40, diff_mask: 0xff };
    server.handle_packet(a, &ClientMsg::LobbySettings(mine).encode());
    let out = server.drain_outbox();
    let expected = LobbySettings { blue_bots: game::MAX_BOTS_PER_TEAM, diff_mask: 0b11111, ..mine };
    for peer in [a, b] {
        assert!(to(&out, peer).contains(&ServerMsg::LobbySettings(expected)));
    }

    // a newcomer is told at once
    let c = PeerId(3);
    server.handle_packet(c, &ClientMsg::join("cat").encode());
    assert!(to(&server.drain_outbox(), c).contains(&ServerMsg::LobbySettings(expected)));
}

#[test]
fn an_endless_match_sends_its_timer_heartbeat_once_a_second() {
    let mut server = Server::new();
    let (a, b) = (PeerId(1), PeerId(2));
    server.handle_packet(a, &ClientMsg::join("alice").encode());
    server.handle_packet(b, &ClientMsg::join("bob").encode());
    start_match(&mut server, a);
    server.match_duration = None;
    let timers = |out: &[(PeerId, Vec<u8>)]| {
        to(out, b).iter().filter(|m| matches!(m, ServerMsg::MatchTimer { secs: u16::MAX })).count()
    };

    server.tick();
    server.drain_outbox();
    // ticking after every packet sends no timer within the second
    for _ in 0..50 {
        server.tick();
    }
    assert_eq!(timers(&server.drain_outbox()), 0);

    // a second on, exactly one heartbeat
    server.last_match_heartbeat = Some(Instant::now() - Duration::from_millis(1100));
    for _ in 0..50 {
        server.tick();
    }
    assert_eq!(timers(&server.drain_outbox()), 1);
}
