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
    rt_server(ServerMsg::Discover { players: 2, max_players: 16, name: "Gridlock".into(), version: None });
    rt_server(ServerMsg::Discover { players: 2, max_players: 16, name: "G".into(), version: Some(PROTOCOL_VERSION) });
    rt_server(ServerMsg::Respawn { position: (1.0, 2.0, 3.0), kind: Some(1) });
    rt_server(ServerMsg::Damage { amount: 12.5, attacker_id: 4, stun: true });
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
    let shot = BotShot { shooter_id: 1, victim_id: 2, start: (0.0, 1.25, 0.0), end: (1.0, 1.0, 76.5) };
    rt_server(ServerMsg::BotShots(vec![shot, BotShot { shooter_id: 3, ..shot }]));
    rt_server(ServerMsg::BotShots(Vec::new()));
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

#[test]
fn snd_round_state_is_48_bytes() {
    let m = ServerMsg::SndRoundState(SndRound {
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
    });
    assert_eq!(m.encode().len(), 48);
    rt_server(m);
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
    rt_client(ClientMsg::Hit { victim_id: 2, damage: 30.0, weapon: Some("AK".into()), backstab: Some(false) });
    rt_client(ClientMsg::StartGame { mode: Some(3), duration: Some(0), autofill: Some((4, 0b11111)), blue_bots: None });
    rt_client(ClientMsg::StartGame { mode: Some(2), duration: Some(0), autofill: Some((1, 0b00100)), blue_bots: Some(5) });
    rt_client(ClientMsg::StartGame { mode: None, duration: None, autofill: None, blue_bots: None });
    rt_client(ClientMsg::WeaponChange { name: "MP5".into() });
    rt_client(ClientMsg::BotSpawn { behavior: 255, level: 2 });
    rt_client(ClientMsg::Chat { text: "hello there".into() });
    rt_client(ClientMsg::EndMatch);
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
