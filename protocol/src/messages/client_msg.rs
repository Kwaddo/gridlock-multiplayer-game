use crate::codec::{dequantize_pitch, quantize_pitch, Reader, Vec3, Writer};
use crate::*;

/// Client -> server.
#[derive(Clone, Debug, PartialEq)]
pub enum ClientMsg {
    /// Username is the raw remainder of the packet (no length prefix), then
    /// `version` (`PROTOCOL_VERSION`; `None` from builds before the check)
    /// written as whitespace, see [`write_join_version`]: servers from before
    /// the check trim it off and still see the plain name.
    Join { username: String, version: Option<u16> },
    /// `pitch` (look up/down, + = up) trails the event flags and is only written when both flags are,
    /// before this pitch was never sent so nobody saw where you looked up or down
    Position { position: Vec3, yaw: f32, state_flags: Option<u8>, event_flags: Option<u8>, pitch: Option<f32> },
    Disconnect,
    WorldChange { target: u8 },
    Discover,
    WallBroken { x: u8, y: u8 },
    Hit { victim_id: u32, damage: f32, weapon: Option<String>, backstab: Option<bool> },
    RespawnRequest,
    StartGame {
        mode: Option<u8>,
        // seconds; 0 = server default, 65535 = unlimited
        duration: Option<u16>,
        // (bots, difficulty mask): bots is the Red team's bot count in team
        // modes and the total bot count in Gun Game
        autofill: Option<(u8, u8)>,
        // Blue team's bot count in team modes; `None` (older clients) means
        // the same as Red
        blue_bots: Option<u8>,
    },
    SetTeam { team: u8 },
    LobbySettings(LobbySettings),
    SndBombPickup,
    SndPlantStart,
    SndPlantStop,
    SndDefuseStart,
    SndDefuseStop,
    WeaponChange { name: String },
    BotSpawn { behavior: u8, level: u8 },
    BotDespawnAll,
    Chat { text: String },
    EndMatch,
}

/// Marks the version at the end of a `Join`; 16 bits follow, see [`write_join_version`].
const JOIN_VERSION_MARK: &[u8] = b"\r\n";
const JOIN_VERSION_LEN: usize = 2 + 16;

/// The version as `\r\n` plus 16 bits, most significant first, each a
/// space (0) or a tab (1). All of it is whitespace, which servers from before
/// the version check trim off the name, so they still greet the player by it
/// (and the joiner sees no version in the reply, and leaves).
fn write_join_version(w: &mut Writer, version: u16) {
    w.bytes(JOIN_VERSION_MARK);
    for bit in (0..16).rev() {
        w.u8(if version >> bit & 1 == 1 { b'\t' } else { b' ' });
    }
}

/// Splits a `Join` payload into the name bytes and the version, if it has one.
fn read_join_version(payload: &[u8]) -> (&[u8], Option<u16>) {
    let Some(at) = payload.len().checked_sub(JOIN_VERSION_LEN) else { return (payload, None) };
    let (name, tail) = payload.split_at(at);
    let (mark, bits) = tail.split_at(JOIN_VERSION_MARK.len());
    if mark != JOIN_VERSION_MARK || !bits.iter().all(|b| *b == b' ' || *b == b'\t') {
        return (payload, None);
    }
    let version = bits.iter().fold(0u16, |v, b| v << 1 | (*b == b'\t') as u16);
    (name, Some(version))
}

impl ClientMsg {
    /// The `Join` this build sends: its name and its `PROTOCOL_VERSION`.
    pub fn join(username: impl Into<String>) -> Self {
        ClientMsg::Join { username: username.into(), version: Some(PROTOCOL_VERSION) }
    }

    pub fn encode(&self) -> Vec<u8> {
        match self {
            ClientMsg::Join { username, version } => {
                let mut w = Writer::new(MSG_JOIN);
                w.bytes(username.as_bytes());
                if let Some(v) = version {
                    write_join_version(&mut w, *v);
                }
                w.finish()
            }
            ClientMsg::Position { position, yaw, state_flags, event_flags, pitch } => {
                let mut w = Writer::new(MSG_POSITION);
                w.vec3(*position).f32(*yaw);
                if let Some(s) = state_flags {
                    w.u8(*s);
                    if let Some(e) = event_flags {
                        w.u8(*e);
                        if let Some(p) = pitch {
                            w.u8(quantize_pitch(*p));
                        }
                    }
                }
                w.finish()
            }
            ClientMsg::Disconnect => vec![MSG_DISCONNECT],
            ClientMsg::WorldChange { target } => vec![MSG_WORLD_CHANGE, *target],
            ClientMsg::Discover => vec![MSG_DISCOVER],
            ClientMsg::WallBroken { x, y } => vec![MSG_WALL_BROKEN, *x, *y],
            ClientMsg::Hit { victim_id, damage, weapon, backstab } => {
                let mut w = Writer::new(MSG_HIT);
                w.u32(*victim_id).f32(*damage);
                if let Some(name) = weapon {
                    w.str8(name);
                    if let Some(b) = backstab {
                        w.bool(*b);
                    }
                }
                w.finish()
            }
            ClientMsg::RespawnRequest => vec![MSG_RESPAWN_REQUEST],
            ClientMsg::StartGame { mode, duration, autofill, blue_bots } => {
                let mut w = Writer::new(MSG_START_GAME);
                if let Some(m) = mode {
                    w.u8(*m);
                    if let Some(d) = duration {
                        w.u16(*d);
                        if let Some((bots, mask)) = autofill {
                            w.u8(*bots).u8(*mask);
                            if let Some(blue) = blue_bots {
                                w.u8(*blue);
                            }
                        }
                    }
                }
                w.finish()
            }
            ClientMsg::SetTeam { team } => vec![MSG_SET_TEAM, *team],
            ClientMsg::LobbySettings(s) => {
                let mut w = Writer::new(MSG_LOBBY_SETTINGS);
                s.write(&mut w);
                w.finish()
            }
            ClientMsg::SndBombPickup => vec![MSG_SND_BOMB_PICKUP],
            ClientMsg::SndPlantStart => vec![MSG_SND_PLANT_START],
            ClientMsg::SndPlantStop => vec![MSG_SND_PLANT_STOP],
            ClientMsg::SndDefuseStart => vec![MSG_SND_DEFUSE_START],
            ClientMsg::SndDefuseStop => vec![MSG_SND_DEFUSE_STOP],
            ClientMsg::WeaponChange { name } => Writer::new(MSG_WEAPON_CHANGE).str8(name).finish(),
            ClientMsg::BotSpawn { behavior, level } => vec![MSG_BOT_SPAWN, *behavior, *level],
            ClientMsg::BotDespawnAll => vec![MSG_BOT_DESPAWN_ALL],
            ClientMsg::Chat { text } => Writer::new(MSG_CHAT).str8(text).finish(),
            ClientMsg::EndMatch => vec![MSG_END_MATCH],
        }
    }

    pub fn decode(packet: &[u8]) -> Option<Self> {
        let (&msg_type, payload) = packet.split_first()?;
        let mut r = Reader::new(payload);
        let msg = match msg_type {
            MSG_JOIN => {
                let (name, version) = read_join_version(r.rest());
                ClientMsg::Join { username: String::from_utf8_lossy(name).trim().to_string(), version }
            }
            MSG_POSITION => ClientMsg::Position {
                position: r.vec3()?,
                yaw: r.f32()?,
                state_flags: r.u8(),
                event_flags: r.u8(),
                pitch: r.u8().map(dequantize_pitch),
            },
            MSG_DISCONNECT => ClientMsg::Disconnect,
            MSG_WORLD_CHANGE => ClientMsg::WorldChange { target: r.u8()? },
            MSG_DISCOVER => ClientMsg::Discover,
            MSG_WALL_BROKEN => ClientMsg::WallBroken { x: r.u8()?, y: r.u8()? },
            MSG_HIT => ClientMsg::Hit {
                victim_id: r.u32()?,
                damage: r.f32()?,
                weapon: r.str8(),
                backstab: r.bool(),
            },
            MSG_RESPAWN_REQUEST => ClientMsg::RespawnRequest,
            MSG_START_GAME => {
                let mode = r.u8();
                let duration = r.u16();
                let autofill = match (r.u8(), r.u8()) {
                    (Some(b), Some(m)) => Some((b, m)),
                    _ => None,
                };
                let blue_bots = if autofill.is_some() { r.u8() } else { None };
                ClientMsg::StartGame { mode, duration, autofill, blue_bots }
            }
            MSG_SET_TEAM => ClientMsg::SetTeam { team: r.u8()? },
            MSG_LOBBY_SETTINGS => ClientMsg::LobbySettings(LobbySettings::read(&mut r)?),
            MSG_SND_BOMB_PICKUP => ClientMsg::SndBombPickup,
            MSG_SND_PLANT_START => ClientMsg::SndPlantStart,
            MSG_SND_PLANT_STOP => ClientMsg::SndPlantStop,
            MSG_SND_DEFUSE_START => ClientMsg::SndDefuseStart,
            MSG_SND_DEFUSE_STOP => ClientMsg::SndDefuseStop,
            MSG_WEAPON_CHANGE => ClientMsg::WeaponChange { name: r.str8()? },
            MSG_BOT_SPAWN => ClientMsg::BotSpawn { behavior: r.u8()?, level: r.u8()? },
            MSG_BOT_DESPAWN_ALL => ClientMsg::BotDespawnAll,
            MSG_CHAT => ClientMsg::Chat { text: r.str8()? },
            MSG_END_MATCH => ClientMsg::EndMatch,
            _ => return None,
        };
        Some(msg)
    }
}
