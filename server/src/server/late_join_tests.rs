//! Joining a match that is already running (issue #24).

use super::*;
use crate::player::{Team, WorldId};

fn to(out: &[(PeerId, crate::net::Packet)], peer: PeerId) -> Vec<ServerMsg> {
    out.iter().filter(|(p, _)| *p == peer).filter_map(|(_, b)| ServerMsg::decode(b)).collect()
}

fn notice(text: &str) -> ServerMsg {
    ServerMsg::Chat { from_id: 0, name: String::new(), text: text.into() }
}

/// alice (host) and bob start a `mode` match with `bots` per team (or in
/// all, in Gun Game); cat connects once it is running.
fn running(mode: u8, bots: u8) -> (Server, PeerId) {
    let mut server = Server::new();
    let (a, b, c) = (PeerId(1), PeerId(2), PeerId(3));
    server.handle_packet(a, &ClientMsg::join("alice").encode());
    server.handle_packet(b, &ClientMsg::join("bob").encode());
    let autofill = (bots > 0).then_some((bots, 0b11111));
    let start = ClientMsg::StartGame { mode: Some(mode), duration: Some(0), autofill, blue_bots: Some(bots) };
    server.handle_packet(a, &start.encode());
    server.match_warmup_end = Some(Instant::now());
    server.tick();
    assert!(server.match_running && server.match_mode == mode);
    server.handle_packet(c, &ClientMsg::join("cat").encode());
    server.drain_outbox();
    (server, c)
}

fn late_join(server: &mut Server, peer: PeerId) -> Vec<ServerMsg> {
    let start = ClientMsg::StartGame {
        mode: Some(MODE_GUN_GAME),
        duration: Some(30),
        autofill: Some((8, 0b1)),
        blue_bots: Some(8),
    };
    server.handle_packet(peer, &start.encode());
    to(&server.drain_outbox(), peer)
}

fn in_maze(server: &Server, team: Team, bots: bool) -> usize {
    server.players.values().filter(|p| p.world == WorldId::Maze && p.team == team && p.is_bot == bots).count()
}

#[test]
fn a_late_joiner_never_changes_the_running_mode_or_settings() {
    let (mut server, c) = running(MODE_TEAM_DEATHMATCH, 0);
    let duration = server.match_duration;
    let settings = server.lobby_settings;
    let msgs = late_join(&mut server, c);

    assert_eq!(server.match_mode, MODE_TEAM_DEATHMATCH);
    assert_eq!(server.match_duration, duration);
    assert_eq!(server.lobby_settings, settings);
    assert_eq!((server.autofill_red_bots, server.autofill_blue_bots), (0, 0));
    assert!(msgs.iter().any(|m| matches!(m, ServerMsg::WorldChange { world: 1, mode: Some(MODE_TEAM_DEATHMATCH), .. })));

    // 1 v 1: cat goes to Red and starts at Red's spawn
    let cat = &server.players[&c];
    assert_eq!((cat.world, cat.team), (WorldId::Maze, Team::Red));
    let spawns = server.maze.as_ref().unwrap().team_spawn_positions(game::MAZE_CELL_SIZE);
    assert_eq!(cat.position, spawns.0);
    assert!(cat.is_alive);
    assert!(msgs.contains(&notice("Joining Red team")));
}

#[test]
fn a_late_joiner_takes_a_bots_place_on_their_team() {
    let (mut server, c) = running(MODE_TEAM_DEATHMATCH, 2);
    let (red, blue) = (in_maze(&server, Team::Red, false), in_maze(&server, Team::Blue, false));
    let (red_bots, blue_bots) = (in_maze(&server, Team::Red, true), in_maze(&server, Team::Blue, true));
    assert_eq!(red + red_bots, blue + blue_bots, "the match starts even");
    assert!(red_bots > 0 && blue_bots > 0);
    let total = server.players.values().filter(|p| p.world == WorldId::Maze).count();
    let msgs = late_join(&mut server, c);

    let team = server.players[&c].team;
    assert_eq!(server.players[&c].world, WorldId::Maze);
    assert!(msgs.contains(&notice(if team == Team::Red { "Joining Red team" } else { "Joining Blue team" })));
    // one bot on cat's team made room: the team and the match are the same size
    let bots_before = if team == Team::Red { red_bots } else { blue_bots };
    assert_eq!(in_maze(&server, team, true), bots_before - 1);
    assert_eq!(in_maze(&server, Team::Red, false) + in_maze(&server, Team::Red, true), red + red_bots);
    assert_eq!(in_maze(&server, Team::Blue, false) + in_maze(&server, Team::Blue, true), blue + blue_bots);
    assert_eq!(server.players.values().filter(|p| p.world == WorldId::Maze).count(), total);
    // and the bot manager forgot it
    assert_eq!(server.bot_manager.bot_states_mut().len(), red_bots + blue_bots - 1);
}

#[test]
fn a_full_match_without_bots_turns_a_late_joiner_away() {
    let (mut server, c) = running(MODE_TEAM_DEATHMATCH, 0);
    // fill the match with humans
    for i in 0..game::MATCH_MAX_PLAYERS as u32 {
        let peer = PeerId(100 + i);
        server.handle_packet(peer, &ClientMsg::join(&format!("p{i}")).encode());
        server.handle_packet(peer, &ClientMsg::StartGame { mode: None, duration: None, autofill: None, blue_bots: None }.encode());
    }
    assert_eq!(server.players.values().filter(|p| p.world == WorldId::Maze).count(), game::MATCH_MAX_PLAYERS);
    let msgs = late_join(&mut server, c);
    assert!(msgs.contains(&ServerMsg::StartDenied { reason: None }));
    assert_eq!(server.players[&c].world, WorldId::Lobby);
}

#[test]
fn an_snd_late_joiner_spectates_until_the_next_round() {
    let (mut server, c) = running(MODE_SEARCH_AND_DESTROY, 0);
    let msgs = late_join(&mut server, c);
    assert_eq!(server.match_mode, MODE_SEARCH_AND_DESTROY);
    let cat = &server.players[&c];
    assert_eq!(cat.world, WorldId::Maze);
    assert!(cat.team == Team::Red || cat.team == Team::Blue);
    assert!(!cat.is_alive);
    assert!(msgs.contains(&notice("Spectating until next round")));

    // still out for the rest of this round
    server.tick();
    assert!(!server.players[&c].is_alive);

    let mut snd = server.snd_state.take().unwrap();
    assert!(snd.start_round(&server.outbox, &mut server.players));
    assert!(server.players[&c].is_alive);
}

#[test]
fn a_gun_game_late_joiner_starts_at_level_one() {
    let (mut server, c) = running(MODE_GUN_GAME, 3);
    server.players.get_mut(&c).unwrap().gun_level = 7;
    let bots = server.bot_manager.bot_states_mut().len();
    let msgs = late_join(&mut server, c);
    let cat = &server.players[&c];
    assert_eq!((cat.world, cat.team, cat.gun_level), (WorldId::Maze, Team::Spectator, 1));
    assert_eq!(cat.current_weapon, game::GUN_GAME_PROGRESSION[0]);
    assert!(cat.is_alive);
    assert!(msgs.contains(&notice("Joining at gun level 1")));
    // room to spare: no bot leaves
    assert_eq!(server.bot_manager.bot_states_mut().len(), bots);
}
