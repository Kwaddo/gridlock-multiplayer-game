use super::*;
use crate::*;

fn rt_server(m: ServerMsg) {
    assert_eq!(ServerMsg::decode(&m.encode()), Some(m));
}

fn rt_client(m: ClientMsg) {
    assert_eq!(ClientMsg::decode(&m.encode()), Some(m));
}

#[test]
fn server_messages_round_trip() {
    // on the fixed-point grid (1/64 m, 1/65536 turn), so it comes back exact
    let ps = PlayerState {
        id: 7,
        position: (1.0, 2.5, -3.25),
        yaw: -std::f32::consts::FRAC_PI_2,
        pitch: -(30.0f32).to_radians(), // whole degrees, so it comes back exact
        state_flags: FLAG_ADS | FLAG_IS_BOT,
        event_flags: EVT_LANDED,
        health: 80,
        alive: true,
        freeze_secs: 2,
    };
    rt_server(ServerMsg::State { world: 1, players: vec![ps, PlayerState { id: 8, ..ps }] });
    rt_server(ServerMsg::State { world: 0, players: Vec::new() });
    rt_server(ServerMsg::Welcome { id: 1, username: "bob".into(), version: None });
    rt_server(ServerMsg::Welcome { id: 1, username: "bob".into(), version: Some(PROTOCOL_VERSION) });
    rt_server(ServerMsg::Maze(MazeLayout {
        width: 2,
        height: 2,
        teleporter: (1, 1),
        rooms: vec![MazeRoom { x: 0, y: 0, w: 1, h: 1 }],
        props: vec![MazeProp { kind: 3, x: 1, y: 0 }],
        lights: vec![MazeLight { x: 0, y: 1, variant: 2 }],
        cells: vec![0, 1, 1, 0],
    }));
    for (mode, freeze_secs, match_timer) in
        [(None, None, None), (Some(2), Some(3), None), (Some(2), Some(3), Some(900))]
    {
        rt_server(ServerMsg::WorldChange { world: 1, spawn: (1.0, 0.0, 2.0), mode, freeze_secs, match_timer });
    }
    rt_server(ServerMsg::Assets { world: 255, entries: vec![("k".into(), "a/b.glb".into())] });
    rt_server(ServerMsg::Discover { players: 2, max_players: 16, name: "Gridlock".into(), version: None, bots: None });
    let version = Some(PROTOCOL_VERSION);
    rt_server(ServerMsg::Discover { players: 2, max_players: 16, name: "G".into(), version, bots: None });
    rt_server(ServerMsg::Discover { players: 9, max_players: 16, name: "G".into(), version, bots: Some(7) });
    rt_server(ServerMsg::Respawn { position: (1.0, 2.0, 3.0), kind: Some(1) });
    rt_server(ServerMsg::Damage { amount: 12.5, attacker_id: 4, stun: true });
    rt_server(ServerMsg::KillFeed { killer_id: 3, victim_id: 9, weapon: "AK".into(), headshot: false });
    rt_server(ServerMsg::KillFeed { killer_id: 3, victim_id: 9, weapon: "AK".into(), headshot: true });
    rt_server(ServerMsg::StartDenied { reason: None });
    rt_server(ServerMsg::StartDenied { reason: Some(1) });
    rt_server(ServerMsg::MatchEnd { winner_id: 3, winner_name: "x".into(), countdown: Some(5) });
    rt_server(ServerMsg::PlayerList(vec![PlayerListEntry {
        id: 1,
        name: "a".into(),
        kills: 1,
        deaths: 2,
        world: 0,
        alive: false,
        gun_level: 1,
        team: 2,
        weapon: "Punch".into(),
        is_bot: true,
    }]));
    rt_server(ServerMsg::MatchTimer { secs: 65535 });
    let shot = Shot { shooter_id: 1, victim_id: 2, start: (0.0, 1.25, 0.0), end: (1.0, 1.0, 76.5) };
    rt_server(ServerMsg::Shots(vec![shot, Shot { shooter_id: 3, ..shot }]));
    rt_server(ServerMsg::Shots(Vec::new()));
    // on the fixed-point grid again (1/64 m, quarter turn) so it comes back exact
    let cbox = CardboardBox { id: 7, position: (2.0, 0.0, -9.5), yaw: std::f32::consts::FRAC_PI_2, carrier: 0 };
    rt_server(ServerMsg::Boxes { world: 0, boxes: vec![cbox, CardboardBox { id: 65535, carrier: 42, ..cbox }] });
    rt_server(ServerMsg::Boxes { world: 1, boxes: Vec::new() });
    let info = PlayerInfo {
        id: 9001,
        name: "Viper".into(),
        team: 2,
        weapon: "AK-47".into(),
        kills: 70_000,
        gun_level: 9,
        max_health: 500,
    };
    rt_server(ServerMsg::PlayerInfo(vec![info.clone(), PlayerInfo { id: 1, weapon: String::new(), ..info }]));
    rt_server(ServerMsg::PlayerInfo(Vec::new()));
    rt_server(ServerMsg::Chat { from_id: 3, name: "alice".into(), text: "gg".into() });
    rt_server(ServerMsg::Chat { from_id: 0, name: String::new(), text: "Match ended".into() });
    rt_server(ServerMsg::PlayerJoined { id: 4, name: "bob".into() });
    rt_server(ServerMsg::MatchStatus { status: MATCH_STATUS_RUNNING });
    rt_server(ServerMsg::TeamScore { red: 42, blue: 7, to_win: 100 });
    let settings = LobbySettings { mode: 2, duration: 900, red_bots: 3, blue_bots: 1, diff_mask: 0b10101 };
    rt_server(ServerMsg::LobbySettings(settings));
    rt_client(ClientMsg::LobbySettings(settings));
    rt_server(ServerMsg::PlayerLeft { id: 4, name: "bob".into(), reason: LEAVE_TIMED_OUT });
}

fn snd_round(bomb_dropped: bool) -> SndRound {
    SndRound {
        round: 1,
        max_rounds: 7,
        red_wins: 0,
        blue_wins: 1,
        phase: 2,
        attacking_team: 1,
        bomb_carrier_id: 0,
        bomb_pos: (1.0, 0.0, 1.0),
        bomb_site: (5.0, 0.0, 5.0),
        bomb_planted: false,
        plant_progress: 0.25,
        defuse_progress: 0.0,
        phase_timer: 30.0,
        bomb_dropped,
    }
}

#[test]
fn snd_round_state_is_49_bytes() {
    let m = ServerMsg::SndRoundState(snd_round(true));
    assert_eq!(m.encode().len(), 49);
    rt_server(m);
    rt_server(ServerMsg::SndRoundState(snd_round(false)));
}

#[test]
fn snd_round_state_from_an_older_server_has_no_dropped_bomb() {
    // older servers send 48 bytes, without the trailing bomb_dropped byte
    let mut bytes = ServerMsg::SndRoundState(snd_round(true)).encode();
    bytes.pop();
    assert_eq!(ServerMsg::decode(&bytes), Some(ServerMsg::SndRoundState(snd_round(false))));
}

#[test]
fn client_messages_round_trip() {
    rt_client(ClientMsg::Join { username: "alice".into(), version: None });
    rt_client(ClientMsg::join("alice"));
    rt_client(ClientMsg::Join { username: "bob".into(), version: Some(0) });
    rt_client(ClientMsg::Join { username: "bob".into(), version: Some(u16::MAX) });
    rt_client(ClientMsg::Position { position: (1.0, 2.0, 3.0), yaw: 0.1, state_flags: Some(1), event_flags: Some(2), pitch: None });
    // pitch in whole degrees comes back exact
    rt_client(ClientMsg::Position {
        position: (1.0, 2.0, 3.0),
        yaw: 0.1,
        state_flags: Some(1),
        event_flags: Some(2),
        pitch: Some((45.0f32).to_radians()),
    });
    rt_client(ClientMsg::Position { position: (1.0, 2.0, 3.0), yaw: 0.1, state_flags: None, event_flags: None, pitch: None });
    rt_client(ClientMsg::Hit { victim_id: 2, damage: 30.0, weapon: Some("AK".into()), backstab: Some(false), headshot: None });
    rt_client(ClientMsg::Hit { victim_id: 2, damage: 30.0, weapon: Some("AK".into()), backstab: Some(false), headshot: Some(true) });
    rt_client(ClientMsg::StartGame { mode: Some(3), duration: Some(0), autofill: Some((4, 0b11111)), blue_bots: None });
    rt_client(ClientMsg::StartGame { mode: Some(2), duration: Some(0), autofill: Some((1, 0b00100)), blue_bots: Some(5) });
    rt_client(ClientMsg::StartGame { mode: None, duration: None, autofill: None, blue_bots: None });
    rt_client(ClientMsg::WeaponChange { name: "MP5".into() });
    rt_client(ClientMsg::BotSpawn { behavior: 255, level: 2 });
    rt_client(ClientMsg::Chat { text: "hello there".into() });
    rt_client(ClientMsg::EndMatch);
    rt_client(ClientMsg::BoxEnter { id: 300 });
    rt_client(ClientMsg::BoxExit);
    rt_client(ClientMsg::BoxShot { id: 7 });
}

// a box is 14 bytes on the wire, so a list is type + world + count + 14 per box
#[test]
fn boxes_message_size() {
    let b = CardboardBox { id: 1, position: (0.0, 0.0, 0.0), yaw: 0.0, carrier: 0 };
    assert_eq!(ServerMsg::Boxes { world: 0, boxes: vec![b; 3] }.encode().len(), 3 + 3 * 14);
}

#[test]
fn shoot_round_trips_with_its_wire_layout() {
    let end = (12.5, -1.25, 300.75);
    let m = ClientMsg::Shoot { end };
    let bytes = m.encode();
    // [MSG_SHOOT][end: 3 x f32]
    assert_eq!(bytes.len(), 13);
    assert_eq!(bytes[0], MSG_SHOOT);
    rt_client(m);
    // a truncated end point is malformed, not a shot at the origin
    assert_eq!(ClientMsg::decode(&bytes[..12]), None);
}

// a kill feed from before the headshot byte still reads, as not a headshot
#[test]
fn old_kill_feed_without_headshot_still_decodes() {
    let new = ServerMsg::KillFeed { killer_id: 1, victim_id: 2, weapon: "AK".into(), headshot: true }.encode();
    let old = &new[..new.len() - 1]; // drop the trailing headshot byte
    assert_eq!(
        ServerMsg::decode(old),
        Some(ServerMsg::KillFeed { killer_id: 1, victim_id: 2, weapon: "AK".into(), headshot: false })
    );
}

#[test]
fn truncated_packets_are_rejected() {
    let full = ServerMsg::Damage { amount: 1.0, attacker_id: 2, stun: false }.encode();
    assert_eq!(ServerMsg::decode(&full[..full.len() - 1]), None);
    assert_eq!(ClientMsg::decode(&[MSG_POSITION, 0, 0]), None);
    assert_eq!(ServerMsg::decode(&[]), None);
    assert_eq!(ServerMsg::decode(&[200]), None);
}

#[test]
fn long_strings_are_clamped() {
    let long = "x".repeat(300);
    let ServerMsg::Welcome { username, .. } =
        ServerMsg::decode(&ServerMsg::Welcome { id: 1, username: long, version: None }.encode()).unwrap()
    else {
        panic!()
    };
    assert_eq!(username.len(), 255);
}

#[test]
fn smoke_break_messages_round_trip() {
    let round = SmokeRound { round: 3, phase: SMOKE_PHASE_RESULTS, phase_timer: 7.5, coins_left: 12, side: SMOKE_SIDE_HUNTER, max_rounds: 5, golden_ready: true, winner: SMOKE_SIDE_HIDER, found: true, gun_lock: 3.5 };
    rt_server(ServerMsg::SmokeRound(round));
    rt_server(ServerMsg::SmokeCoins(vec![(1.0, 0.0, -2.5), (10.0, 0.0, 4.0)]));
    rt_server(ServerMsg::SmokeCoins(Vec::new()));
    rt_server(ServerMsg::SmokeScores(vec![SmokeScore { id: 1, points: 30, side: SMOKE_SIDE_HIDER, round_points: -20, mult_tenths: 10 }, SmokeScore { id: 9001, points: 120, side: SMOKE_SIDE_HUNTER, round_points: 75, mult_tenths: 14 }]));
    rt_server(ServerMsg::SmokeCoinTaken { at: (3.0, 0.0, 3.0) });
}

#[test]
fn smoke_break_message_sizes() {
    let round = SmokeRound { round: 1, phase: 0, phase_timer: 15.0, coins_left: 20, side: 0, max_rounds: 3, golden_ready: false, winner: SMOKE_NO_WINNER, found: false, gun_lock: 0.0 };
    assert_eq!(ServerMsg::SmokeRound(round).encode().len(), 1 + 18);
    let row = SmokeScore { id: 1, points: 0, side: 0, round_points: 0, mult_tenths: 10 };
    assert_eq!(ServerMsg::SmokeScores(vec![row; 4]).encode().len(), 2 + 4 * 12);
    assert_eq!(ServerMsg::SmokeFeed { shooter: 1, target: 0, red_handed: false, points: -10 }.encode().len(), 1 + 11);
    assert_eq!(ServerMsg::SmokeCoins(vec![(0.0, 0.0, 0.0); 20]).encode().len(), 2 + 20 * 6);
}

#[test]
fn smoke_break_golden_messages_round_trip() {
    rt_server(ServerMsg::SmokeGoldCoin(Some(((4.0, 0.0, 9.0), 27.5))));
    rt_server(ServerMsg::SmokeGoldCoin(None));
    rt_server(ServerMsg::SmokeGolden(vec![3, 9001]));
    rt_server(ServerMsg::SmokeGolden(Vec::new()));
    rt_server(ServerMsg::SmokeDots(vec![((1.0, 0.0, 2.0), true), ((30.0, 0.0, 8.0), false)]));
    rt_client(ClientMsg::SmokeGoGolden);
    rt_server(ServerMsg::SmokeFeed { shooter: 7, target: 9001, red_handed: true, points: -100 });
    rt_server(ServerMsg::SmokeFeed { shooter: 7, target: 0, red_handed: false, points: -10 });
}
