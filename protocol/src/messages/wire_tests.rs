//! Quantization, packet sizes and the version handshake's wire tricks.

use super::*;
use crate::codec::*;
use crate::*;

fn state(id: u32, position: Vec3, yaw: f32) -> PlayerState {
    PlayerState {
        id,
        position,
        yaw,
        state_flags: FLAG_SPRINTING | FLAG_SILENT,
        event_flags: EVT_JUMP_START,
        health: 125,
        alive: true,
        freeze_secs: 0,
    }
}

fn decode_state(m: &ServerMsg) -> Vec<PlayerState> {
    match ServerMsg::decode(&m.encode()) {
        Some(ServerMsg::State { players, .. }) => players,
        other => panic!("not a state: {other:?}"),
    }
}

/// Smallest angle between two yaws, radians.
fn yaw_error(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(std::f32::consts::TAU);
    d.min(std::f32::consts::TAU - d)
}

#[test]
fn positions_come_back_within_a_centimetre_everywhere() {
    // the biggest maze is 43 x 1.8 m; the lobby and range reach about +-41 m;
    // sweep well past both, and up and down too
    let mut worst = 0.0f32;
    let mut v = -120.0f32;
    while v <= 120.0 {
        let got = dequantize_coord(quantize_coord(v));
        worst = worst.max((got - v).abs());
        v += 0.0137;
    }
    assert!(worst <= 0.5 / POS_UNITS_PER_M + 1e-5, "worst error {worst} m");
    assert!(worst <= 0.01);
    // the largest maze a u8 side could ever describe still fits
    assert!(POS_LIMIT_M > 255.0 * 1.8);
}

#[test]
fn extreme_coordinates_saturate_instead_of_wrapping() {
    let edge = dequantize_coord(i16::MAX);
    assert_eq!(dequantize_coord(quantize_coord(POS_LIMIT_M)), edge);
    assert_eq!(dequantize_coord(quantize_coord(1.0e9)), edge);
    assert_eq!(dequantize_coord(quantize_coord(f32::INFINITY)), edge);
    assert_eq!(quantize_coord(-1.0e9), i16::MIN);
    assert_eq!(quantize_coord(f32::NEG_INFINITY), i16::MIN);
    assert_eq!(quantize_coord(f32::NAN), 0);

    let far = ServerMsg::State {
        world: 1,
        players: vec![
            state(1, (77.4, 2.4, 77.4), 0.0),
            state(2, (-41.0, -0.05, 41.0), 0.0),
            state(3, (511.0, -511.0, 0.0), 0.0),
            state(4, (9.0e6, -9.0e6, f32::NAN), 0.0),
        ],
    };
    let got = decode_state(&far);
    assert!((got[0].position.0 - 77.4).abs() <= 0.01 && (got[0].position.1 - 2.4).abs() <= 0.01);
    assert!((got[1].position.0 + 41.0).abs() <= 0.01 && (got[1].position.1 + 0.05).abs() <= 0.01);
    assert_eq!(got[2].position, (511.0, -511.0, 0.0));
    assert_eq!(got[3].position, (edge, dequantize_coord(i16::MIN), 0.0));
}

#[test]
fn yaw_comes_back_within_a_hundredth_of_a_degree() {
    let step = std::f32::consts::TAU / 65536.0;
    let mut worst = 0.0f32;
    // any angle, wound up or not
    let mut yaw = -20.0f32;
    while yaw <= 20.0 {
        worst = worst.max(yaw_error(dequantize_yaw(quantize_yaw(yaw)), yaw));
        yaw += 0.000_731;
    }
    assert!(worst <= step * 0.5 + 1e-5, "worst {} deg", worst.to_degrees());
    assert!(worst.to_degrees() < 0.006);
    // a full turn and just under it wrap to the same place
    assert_eq!(quantize_yaw(std::f32::consts::TAU), 0);
    assert_eq!(quantize_yaw(std::f32::consts::TAU - step * 0.25), 0);
    assert_eq!(quantize_yaw(f32::NAN), 0);
    assert_eq!(quantize_yaw(f32::INFINITY), 0);
    let back = dequantize_yaw(quantize_yaw(3.0));
    assert!((-std::f32::consts::PI..std::f32::consts::PI).contains(&back));
}

#[test]
fn state_entries_are_18_bytes() {
    let one = ServerMsg::State { world: 1, players: vec![state(1, (1.0, 0.0, 1.0), 0.3)] };
    let sixteen = ServerMsg::State {
        world: 1,
        players: (0..16).map(|i| state(i, (i as f32, 0.0, 2.0 * i as f32), i as f32)).collect(),
    };
    let header = 1 + 1 + 2;
    assert_eq!(one.encode().len(), header + PlayerState::WIRE_BYTES);
    assert_eq!(sixteen.encode().len(), header + 16 * PlayerState::WIRE_BYTES);
    assert_eq!(PlayerState::WIRE_BYTES, 18);
    // health saturates in u16 on the server side; flags and freeze pass through
    let got = decode_state(&sixteen);
    assert_eq!(got.len(), 16);
    assert!(got.iter().all(|p| p.state_flags == FLAG_SPRINTING | FLAG_SILENT && p.health == 125));
}

#[test]
fn truncated_new_messages_are_rejected() {
    let packets = [
        ServerMsg::State { world: 0, players: vec![state(1, (1.0, 2.0, 3.0), 1.0)] }.encode(),
        ServerMsg::BotShots(vec![BotShot { shooter_id: 1, victim_id: 2, start: (0.0, 0.0, 0.0), end: (1.0, 1.0, 1.0) }])
            .encode(),
        ServerMsg::PlayerInfo(vec![PlayerInfo { id: 3, name: "a".into(), ..Default::default() }]).encode(),
    ];
    for full in packets {
        for cut in 1..full.len() {
            assert_eq!(ServerMsg::decode(&full[..cut]), None, "type {} cut at {cut}", full[0]);
        }
    }
    // a huge count with no entries behind it neither decodes nor allocates
    assert_eq!(ServerMsg::decode(&[MSG_STATE, 0, 0xFF, 0xFF]), None);
}

/// What a server from before the version check makes of a `Join`.
fn old_server_name(packet: &[u8]) -> String {
    String::from_utf8_lossy(&packet[1..]).trim().to_string()
}

#[test]
fn old_servers_read_the_plain_name_from_a_versioned_join() {
    for name in ["alice", "Bob Smith", "x", ""] {
        let packet = ClientMsg::join(name).encode();
        assert_eq!(old_server_name(&packet), name);
        assert_eq!(
            ClientMsg::decode(&packet),
            Some(ClientMsg::Join { username: name.into(), version: Some(PROTOCOL_VERSION) })
        );
    }
    // a join from before the check has no version
    let mut old = vec![MSG_JOIN];
    old.extend_from_slice(b"carol");
    assert_eq!(ClientMsg::decode(&old), Some(ClientMsg::Join { username: "carol".into(), version: None }));
    // whitespace that is not a version stays part of the (trimmed) name
    let mut odd = vec![MSG_JOIN];
    odd.extend_from_slice(b"dave\r\n   \t");
    assert_eq!(ClientMsg::decode(&odd), Some(ClientMsg::Join { username: "dave".into(), version: None }));
}

#[test]
fn old_clients_read_a_versioned_welcome_and_discover_reply() {
    // builds from before the check stop reading after the name
    let welcome = ServerMsg::Welcome { id: 4, username: "alice".into(), version: Some(PROTOCOL_VERSION) }.encode();
    let mut r = Reader::new(&welcome[1..]);
    assert_eq!((r.u32(), r.str8()), (Some(4), Some("alice".to_string())));
    let discover =
        ServerMsg::Discover { players: 3, max_players: 16, name: "g".into(), version: Some(PROTOCOL_VERSION) }
            .encode();
    let mut r = Reader::new(&discover[1..]);
    assert_eq!((r.u8(), r.u8(), r.str8()), (Some(3), Some(16), Some("g".to_string())));
    // and what they send back has no version
    let mut old_welcome = Writer::new(MSG_WELCOME);
    old_welcome.u32(4).str8("alice");
    assert_eq!(
        ServerMsg::decode(&old_welcome.finish()),
        Some(ServerMsg::Welcome { id: 4, username: "alice".into(), version: None })
    );
}
