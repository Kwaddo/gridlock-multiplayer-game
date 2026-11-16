use super::*;
use crate::net::BOT_PEER_BASE;

const DT: f32 = 1.0 / 60.0;
const SITE: (f32, f32, f32) = (10.0, 0.0, 10.0);
const RED_SPAWN: (f32, f32, f32) = (1.0, 0.0, 1.0);
const BLUE_SPAWN: (f32, f32, f32) = (20.0, 0.0, 20.0);

fn player(id: u32, team: Team, is_bot: bool) -> Player {
    let mut p = Player::new(id, format!("p{id}"));
    p.world = WorldId::Maze;
    p.team = team;
    p.is_bot = is_bot;
    p
}

fn peer(id: u32, is_bot: bool) -> PeerId {
    if is_bot { PeerId(BOT_PEER_BASE + id) } else { PeerId(id) }
}

fn add(players: &mut HashMap<PeerId, Player>, id: u32, team: Team, is_bot: bool) -> PeerId {
    let addr = peer(id, is_bot);
    players.insert(addr, player(id, team, is_bot));
    addr
}

/// Round 1 (Red attacks) with the given players.
fn round_one(players: &mut HashMap<PeerId, Player>) -> (SndState, Outbox) {
    let mut snd = SndState::new(SITE, RED_SPAWN, BLUE_SPAWN);
    let socket = Outbox::default();
    assert!(snd.start_round(&socket, players));
    assert_eq!(snd.attacking_team, Team::Red);
    socket.drain();
    (snd, socket)
}

fn last_round_state(socket: &Outbox) -> Option<SndRound> {
    socket.drain().into_iter().rev().find_map(|(_, b)| match ServerMsg::decode(&b) {
        Some(ServerMsg::SndRoundState(s)) => Some(s),
        _ => None,
    })
}

#[test]
fn carrier_death_drops_the_bomb_where_they_died() {
    let mut players = HashMap::new();
    let red = add(&mut players, 1, Team::Red, false);
    add(&mut players, 2, Team::Blue, false);
    let (mut snd, socket) = round_one(&mut players);
    assert_eq!(snd.bomb_carrier, Some(1));
    assert_eq!(snd.bomb_dropped_pos, None);

    let p = players.get_mut(&red).unwrap();
    p.position = (5.0, 1.7, 6.0);
    p.is_alive = false;
    snd.tick(DT, &socket, &mut players);

    assert_eq!(snd.bomb_carrier, None);
    // snapped to the floor
    assert_eq!(snd.bomb_dropped_pos, Some((5.0, 0.0, 6.0)));
    let state = last_round_state(&socket).expect("the drop is broadcast at once");
    assert!(state.bomb_dropped);
    assert_eq!((state.bomb_carrier_id, state.bomb_pos), (0, (5.0, 0.0, 6.0)));
}

#[test]
fn carrier_disconnect_drops_the_bomb_where_they_were_last_seen() {
    let mut players = HashMap::new();
    let red = add(&mut players, 1, Team::Red, false);
    add(&mut players, 2, Team::Blue, false);
    let (mut snd, socket) = round_one(&mut players);

    players.get_mut(&red).unwrap().position = (7.0, 0.5, 3.0);
    snd.tick(DT, &socket, &mut players);
    assert_eq!(snd.bomb_carrier, Some(1));
    players.remove(&red);
    snd.tick(DT, &socket, &mut players);

    assert_eq!(snd.bomb_carrier, None);
    assert_eq!(snd.bomb_dropped_pos, Some((7.0, 0.0, 3.0)));
}

#[test]
fn an_attacker_picks_up_the_dropped_bomb_and_a_defender_does_not() {
    let mut players = HashMap::new();
    let red = add(&mut players, 1, Team::Red, false);
    let blue = add(&mut players, 2, Team::Blue, false);
    let mate = add(&mut players, 3, Team::Red, false);
    let bot = add(&mut players, 4, Team::Red, true);
    let (mut snd, socket) = round_one(&mut players);
    // out of reach while the carrier dies
    for addr in [blue, mate, bot] {
        players.get_mut(&addr).unwrap().position = (30.0, 0.0, 30.0);
    }
    let p = players.get_mut(&red).unwrap();
    p.position = (5.0, 0.0, 5.0);
    p.is_alive = false;
    snd.tick(DT, &socket, &mut players);
    assert_eq!(snd.bomb_dropped_pos, Some((5.0, 0.0, 5.0)));

    // a defender and an attacking bot stand on it: it stays on the floor
    players.get_mut(&blue).unwrap().position = (5.0, 0.0, 5.0);
    players.get_mut(&bot).unwrap().position = (5.0, 0.0, 5.0);
    snd.tick(DT, &socket, &mut players);
    assert!(!snd.try_pickup_bomb(blue, &players));
    assert!(!snd.try_pickup_bomb(bot, &players));
    assert_eq!(snd.bomb_carrier, None);
    assert_eq!(snd.bomb_dropped_pos, Some((5.0, 0.0, 5.0)));

    // a human attacker walks within range and carries it
    players.get_mut(&mate).unwrap().position = (6.0, 0.0, 5.5);
    snd.tick(DT, &socket, &mut players);
    assert_eq!(snd.bomb_carrier, Some(3));
    assert_eq!(snd.bomb_dropped_pos, None);
    let state = last_round_state(&socket).expect("the pickup is broadcast at once");
    assert!(!state.bomb_dropped);
    assert_eq!(state.bomb_carrier_id, 3);
}

#[test]
fn start_round_never_gives_the_bomb_to_a_bot() {
    let mut players = HashMap::new();
    // bots with lower ids than the human, so map order or "first" can't hide it
    add(&mut players, 1, Team::Red, true);
    add(&mut players, 2, Team::Red, true);
    add(&mut players, 9, Team::Red, false);
    add(&mut players, 3, Team::Blue, false);
    let (snd, _) = round_one(&mut players);
    assert_eq!(snd.bomb_carrier, Some(9));
}

#[test]
fn no_human_attackers_means_no_bomb() {
    let mut players = HashMap::new();
    let bot = add(&mut players, 1, Team::Red, true);
    add(&mut players, 2, Team::Blue, false);
    let (mut snd, socket) = round_one(&mut players);
    assert_eq!(snd.bomb_carrier, None);
    assert_eq!(snd.bomb_dropped_pos, None);

    snd.tick(DT, &socket, &mut players);
    let state = last_round_state(&socket).expect("first round state");
    assert_eq!(state.bomb_carrier_id, 0);
    assert!(!state.bomb_dropped, "no phantom bomb");
    assert!(!snd.try_pickup_bomb(bot, &players));
}
