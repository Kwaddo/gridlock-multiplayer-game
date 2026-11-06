//! Typed messages. `encode` produces a full packet (`[msg_type][payload]`);
//! `decode` takes a full packet and returns `None` for unknown or truncated ones.
//! `Option` fields are optional trailing bytes: they must stay at the end and are
//! only written when `Some`.

use crate::codec::{Reader, Vec3, Writer};
use crate::*;

/// One entry of `MSG_STATE`.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerState {
    pub id: u32,
    pub position: Vec3,
    pub yaw: f32,
    pub world: u8,
    pub state_flags: u8,
    pub event_flags: u8,
    pub kills: u32,
    pub gun_level: u8,
    pub health: f32,
    pub max_health: f32,
    pub name: String,
    pub alive: bool,
    // remaining post-teleport freeze seconds (0 = none)
    pub freeze_secs: u8,
    pub team: u8,
    pub weapon: String,
}

/// One entry of `MSG_PLAYER_LIST`.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerListEntry {
    pub id: u32,
    pub name: String,
    pub kills: u32,
    pub deaths: u32,
    pub world: u8,
    pub alive: bool,
    pub gun_level: u8,
    pub team: u8,
    pub weapon: String,
    pub is_bot: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MazeRoom {
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MazeProp {
    pub kind: u8,
    pub x: u8,
    pub y: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MazeLight {
    pub x: u8,
    pub y: u8,
    pub variant: u8,
}

/// `MSG_MAZE` layout: header, rooms, props, ceiling lights, then `width*height` cells.
#[derive(Clone, Debug, PartialEq)]
pub struct MazeLayout {
    pub width: u8,
    pub height: u8,
    pub teleporter: (u8, u8),
    pub rooms: Vec<MazeRoom>,
    pub props: Vec<MazeProp>,
    pub lights: Vec<MazeLight>,
    pub cells: Vec<u8>,
}

/// `MSG_SND_ROUND_STATE` (fixed 47-byte payload).
#[derive(Clone, Debug, PartialEq)]
pub struct SndRound {
    pub round: u8,
    pub max_rounds: u8,
    pub red_wins: u8,
    pub blue_wins: u8,
    pub phase: u8,
    pub attacking_team: u8,
    // 0 = no carrier
    pub bomb_carrier_id: u32,
    // dropped bomb position, zeros when not dropped
    pub bomb_pos: Vec3,
    pub bomb_site: Vec3,
    pub bomb_planted: bool,
    pub plant_progress: f32,
    pub defuse_progress: f32,
    pub phase_timer: f32,
}

/// Server -> client.
#[derive(Clone, Debug, PartialEq)]
pub enum ServerMsg {
    Welcome { id: u32, username: String },
    State(Vec<PlayerState>),
    Maze(MazeLayout),
    WorldChange {
        world: u8,
        spawn: Vec3,
        // only sent when entering the maze at match start
        mode: Option<u8>,
        freeze_secs: Option<u8>,
        match_timer: Option<u16>,
    },
    Assets { world: u8, entries: Vec<(String, String)> },
    /// LAN discovery reply.
    Discover { players: u8, max_players: u8, name: String },
    WallBroken { x: u8, y: u8 },
    Death { killer_id: u32, weapon: String },
    Respawn { position: Vec3, kind: Option<u8> }, // kind: 0=random, 1=teleporter fallback
    Damage { amount: f32, attacker_id: u32, stun: bool },
    Kill { victim_id: u32, weapon: String },
    KillFeed { killer_id: u32, victim_id: u32, weapon: String },
    StartDenied { reason: Option<u8> }, // START_DENIED_*; None = running and full
    MatchJoined { id: u32, name: String },
    MatchEnd { winner_id: u32, winner_name: String, countdown: Option<u8> },
    PlayerList(Vec<PlayerListEntry>),
    MatchCountdown { secs: u8, mode: Option<u8> },
    MatchTimer { secs: u16 },
    SndRoundState(SndRound),
    LobbyLeader { id: u32 }, // 0 = no leader
    BotShot { shooter_id: u32, victim_id: u32, start: Vec3, end: Vec3 },
    Chat { from_id: u32, name: String, text: String }, // from_id 0 = server notice
    PlayerJoined { id: u32, name: String },
    PlayerLeft { id: u32, name: String, reason: u8 },
    MatchStatus { status: u8 },
}

/// Client -> server.
#[derive(Clone, Debug, PartialEq)]
pub enum ClientMsg {
    /// Username is the raw remainder of the packet (no length prefix).
    Join { username: String },
    Position { position: Vec3, yaw: f32, state_flags: Option<u8>, event_flags: Option<u8> },
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
        // (bots_per_team, difficulty mask)
        autofill: Option<(u8, u8)>,
    },
    SetTeam { team: u8 },
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

impl ServerMsg {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            ServerMsg::Welcome { id, username } => {
                Writer::new(MSG_WELCOME).u32(*id).str8(username).finish()
            }
            ServerMsg::State(players) => {
                let mut w = Writer::new(MSG_STATE);
                w.u32(players.len() as u32);
                for p in players {
                    w.u32(p.id).vec3(p.position).f32(p.yaw);
                    w.u8(p.world).u8(p.state_flags).u8(p.event_flags);
                    w.u32(p.kills).u8(p.gun_level).f32(p.health).f32(p.max_health);
                    w.str8(&p.name).bool(p.alive).u8(p.freeze_secs);
                    w.u8(p.team).str8(&p.weapon);
                }
                w.finish()
            }
            ServerMsg::Maze(m) => {
                let mut w = Writer::new(MSG_MAZE);
                w.u8(m.width).u8(m.height).u8(m.teleporter.0).u8(m.teleporter.1);
                w.u8(m.rooms.len().min(255) as u8);
                for r in m.rooms.iter().take(255) {
                    w.u8(r.x).u8(r.y).u8(r.w).u8(r.h);
                }
                w.u8(m.props.len().min(255) as u8);
                for p in m.props.iter().take(255) {
                    w.u8(p.kind).u8(p.x).u8(p.y);
                }
                w.u8(m.lights.len().min(255) as u8);
                for l in m.lights.iter().take(255) {
                    w.u8(l.x).u8(l.y).u8(l.variant);
                }
                w.bytes(&m.cells).finish()
            }
            ServerMsg::WorldChange { world, spawn, mode, freeze_secs, match_timer } => {
                let mut w = Writer::new(MSG_WORLD_CHANGE);
                w.u8(*world).vec3(*spawn);
                if let Some(mode) = mode {
                    w.u8(*mode);
                    if let Some(f) = freeze_secs {
                        w.u8(*f);
                        if let Some(t) = match_timer {
                            w.u16(*t);
                        }
                    }
                }
                w.finish()
            }
            ServerMsg::Assets { world, entries } => {
                let mut w = Writer::new(MSG_ASSETS);
                w.u8(*world).u8(entries.len().min(255) as u8);
                for (key, path) in entries.iter().take(255) {
                    w.str8(key).str8(path);
                }
                w.finish()
            }
            ServerMsg::Discover { players, max_players, name } => {
                Writer::new(MSG_DISCOVER).u8(*players).u8(*max_players).str8(name).finish()
            }
            ServerMsg::WallBroken { x, y } => Writer::new(MSG_WALL_BROKEN).u8(*x).u8(*y).finish(),
            ServerMsg::Death { killer_id, weapon } => {
                Writer::new(MSG_DEATH).u32(*killer_id).str8(weapon).finish()
            }
            ServerMsg::Respawn { position, kind } => {
                let mut w = Writer::new(MSG_RESPAWN);
                w.vec3(*position);
                if let Some(k) = kind {
                    w.u8(*k);
                }
                w.finish()
            }
            ServerMsg::Damage { amount, attacker_id, stun } => {
                Writer::new(MSG_DAMAGE).f32(*amount).u32(*attacker_id).bool(*stun).finish()
            }
            ServerMsg::Kill { victim_id, weapon } => {
                Writer::new(MSG_KILL).u32(*victim_id).str8(weapon).finish()
            }
            ServerMsg::KillFeed { killer_id, victim_id, weapon } => Writer::new(MSG_KILL_FEED)
                .u32(*killer_id)
                .u32(*victim_id)
                .str8(weapon)
                .finish(),
            ServerMsg::StartDenied { reason } => {
                let mut w = Writer::new(MSG_START_DENIED);
                if let Some(r) = reason {
                    w.u8(*r);
                }
                w.finish()
            }
            ServerMsg::MatchJoined { id, name } => {
                Writer::new(MSG_MATCH_JOINED).u32(*id).str8(name).finish()
            }
            ServerMsg::MatchEnd { winner_id, winner_name, countdown } => {
                let mut w = Writer::new(MSG_MATCH_END);
                w.u32(*winner_id).str8(winner_name);
                if let Some(c) = countdown {
                    w.u8(*c);
                }
                w.finish()
            }
            ServerMsg::PlayerList(entries) => {
                let mut w = Writer::new(MSG_PLAYER_LIST);
                w.u8(entries.len().min(255) as u8);
                for e in entries.iter().take(255) {
                    w.u32(e.id).str8(&e.name).u32(e.kills).u32(e.deaths);
                    w.u8(e.world).bool(e.alive).u8(e.gun_level).u8(e.team);
                    w.str8(&e.weapon).bool(e.is_bot);
                }
                w.finish()
            }
            ServerMsg::MatchCountdown { secs, mode } => {
                let mut w = Writer::new(MSG_MATCH_COUNTDOWN);
                w.u8(*secs);
                if let Some(m) = mode {
                    w.u8(*m);
                }
                w.finish()
            }
            ServerMsg::MatchTimer { secs } => Writer::new(MSG_MATCH_TIMER).u16(*secs).finish(),
            ServerMsg::SndRoundState(s) => {
                let mut w = Writer::new(MSG_SND_ROUND_STATE);
                w.u8(s.round).u8(s.max_rounds).u8(s.red_wins).u8(s.blue_wins);
                w.u8(s.phase).u8(s.attacking_team).u32(s.bomb_carrier_id);
                w.vec3(s.bomb_pos).vec3(s.bomb_site).bool(s.bomb_planted);
                w.f32(s.plant_progress).f32(s.defuse_progress).f32(s.phase_timer);
                w.finish()
            }
            ServerMsg::LobbyLeader { id } => Writer::new(MSG_LOBBY_LEADER).u32(*id).finish(),
            ServerMsg::BotShot { shooter_id, victim_id, start, end } => Writer::new(MSG_BOT_SHOT)
                .u32(*shooter_id)
                .u32(*victim_id)
                .vec3(*start)
                .vec3(*end)
                .finish(),
            ServerMsg::Chat { from_id, name, text } => {
                Writer::new(MSG_CHAT).u32(*from_id).str8(name).str8(text).finish()
            }
            ServerMsg::PlayerJoined { id, name } => {
                Writer::new(MSG_PLAYER_JOINED).u32(*id).str8(name).finish()
            }
            ServerMsg::PlayerLeft { id, name, reason } => {
                Writer::new(MSG_PLAYER_LEFT).u32(*id).str8(name).u8(*reason).finish()
            }
            ServerMsg::MatchStatus { status } => vec![MSG_MATCH_STATUS, *status],
        }
    }

    pub fn decode(packet: &[u8]) -> Option<Self> {
        let (&msg_type, payload) = packet.split_first()?;
        let mut r = Reader::new(payload);
        let msg = match msg_type {
            MSG_WELCOME => ServerMsg::Welcome { id: r.u32()?, username: r.str8()? },
            MSG_STATE => {
                let count = r.u32()? as usize;
                // cap the preallocation; a bogus count must not allocate gigabytes
                let mut players = Vec::with_capacity(count.min(256));
                for _ in 0..count {
                    players.push(PlayerState {
                        id: r.u32()?,
                        position: r.vec3()?,
                        yaw: r.f32()?,
                        world: r.u8()?,
                        state_flags: r.u8()?,
                        event_flags: r.u8()?,
                        kills: r.u32()?,
                        gun_level: r.u8()?,
                        health: r.f32()?,
                        max_health: r.f32()?,
                        name: r.str8()?,
                        alive: r.bool()?,
                        freeze_secs: r.u8()?,
                        team: r.u8()?,
                        weapon: r.str8()?,
                    });
                }
                ServerMsg::State(players)
            }
            MSG_MAZE => {
                let width = r.u8()?;
                let height = r.u8()?;
                let teleporter = (r.u8()?, r.u8()?);
                let mut rooms = Vec::new();
                for _ in 0..r.u8()? {
                    rooms.push(MazeRoom { x: r.u8()?, y: r.u8()?, w: r.u8()?, h: r.u8()? });
                }
                let mut props = Vec::new();
                for _ in 0..r.u8()? {
                    props.push(MazeProp { kind: r.u8()?, x: r.u8()?, y: r.u8()? });
                }
                let mut lights = Vec::new();
                for _ in 0..r.u8()? {
                    lights.push(MazeLight { x: r.u8()?, y: r.u8()?, variant: r.u8()? });
                }
                let cells = r.bytes(width as usize * height as usize)?.to_vec();
                ServerMsg::Maze(MazeLayout { width, height, teleporter, rooms, props, lights, cells })
            }
            MSG_WORLD_CHANGE => ServerMsg::WorldChange {
                world: r.u8()?,
                spawn: r.vec3()?,
                mode: r.u8(),
                freeze_secs: r.u8(),
                match_timer: r.u16(),
            },
            MSG_ASSETS => {
                let world = r.u8()?;
                let mut entries = Vec::new();
                for _ in 0..r.u8()? {
                    entries.push((r.str8()?, r.str8()?));
                }
                ServerMsg::Assets { world, entries }
            }
            MSG_DISCOVER => {
                ServerMsg::Discover { players: r.u8()?, max_players: r.u8()?, name: r.str8()? }
            }
            MSG_WALL_BROKEN => ServerMsg::WallBroken { x: r.u8()?, y: r.u8()? },
            MSG_DEATH => ServerMsg::Death { killer_id: r.u32()?, weapon: r.str8()? },
            MSG_RESPAWN => ServerMsg::Respawn { position: r.vec3()?, kind: r.u8() },
            MSG_DAMAGE => {
                ServerMsg::Damage { amount: r.f32()?, attacker_id: r.u32()?, stun: r.bool()? }
            }
            MSG_KILL => ServerMsg::Kill { victim_id: r.u32()?, weapon: r.str8()? },
            MSG_KILL_FEED => ServerMsg::KillFeed {
                killer_id: r.u32()?,
                victim_id: r.u32()?,
                weapon: r.str8()?,
            },
            MSG_START_DENIED => ServerMsg::StartDenied { reason: r.u8() },
            MSG_MATCH_JOINED => ServerMsg::MatchJoined { id: r.u32()?, name: r.str8()? },
            MSG_MATCH_END => ServerMsg::MatchEnd {
                winner_id: r.u32()?,
                winner_name: r.str8()?,
                countdown: r.u8(),
            },
            MSG_PLAYER_LIST => {
                let mut entries = Vec::new();
                for _ in 0..r.u8()? {
                    entries.push(PlayerListEntry {
                        id: r.u32()?,
                        name: r.str8()?,
                        kills: r.u32()?,
                        deaths: r.u32()?,
                        world: r.u8()?,
                        alive: r.bool()?,
                        gun_level: r.u8()?,
                        team: r.u8()?,
                        weapon: r.str8()?,
                        is_bot: r.bool()?,
                    });
                }
                ServerMsg::PlayerList(entries)
            }
            MSG_MATCH_COUNTDOWN => ServerMsg::MatchCountdown { secs: r.u8()?, mode: r.u8() },
            MSG_MATCH_TIMER => ServerMsg::MatchTimer { secs: r.u16()? },
            MSG_SND_ROUND_STATE => ServerMsg::SndRoundState(SndRound {
                round: r.u8()?,
                max_rounds: r.u8()?,
                red_wins: r.u8()?,
                blue_wins: r.u8()?,
                phase: r.u8()?,
                attacking_team: r.u8()?,
                bomb_carrier_id: r.u32()?,
                bomb_pos: r.vec3()?,
                bomb_site: r.vec3()?,
                bomb_planted: r.bool()?,
                plant_progress: r.f32()?,
                defuse_progress: r.f32()?,
                phase_timer: r.f32()?,
            }),
            MSG_LOBBY_LEADER => ServerMsg::LobbyLeader { id: r.u32()? },
            MSG_BOT_SHOT => ServerMsg::BotShot {
                shooter_id: r.u32()?,
                victim_id: r.u32()?,
                start: r.vec3()?,
                end: r.vec3()?,
            },
            MSG_CHAT => ServerMsg::Chat { from_id: r.u32()?, name: r.str8()?, text: r.str8()? },
            MSG_PLAYER_JOINED => ServerMsg::PlayerJoined { id: r.u32()?, name: r.str8()? },
            MSG_PLAYER_LEFT => {
                ServerMsg::PlayerLeft { id: r.u32()?, name: r.str8()?, reason: r.u8()? }
            }
            MSG_MATCH_STATUS => ServerMsg::MatchStatus { status: r.u8()? },
            _ => return None,
        };
        Some(msg)
    }
}

impl ClientMsg {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            ClientMsg::Join { username } => Writer::new(MSG_JOIN).bytes(username.as_bytes()).finish(),
            ClientMsg::Position { position, yaw, state_flags, event_flags } => {
                let mut w = Writer::new(MSG_POSITION);
                w.vec3(*position).f32(*yaw);
                if let Some(s) = state_flags {
                    w.u8(*s);
                    if let Some(e) = event_flags {
                        w.u8(*e);
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
            ClientMsg::StartGame { mode, duration, autofill } => {
                let mut w = Writer::new(MSG_START_GAME);
                if let Some(m) = mode {
                    w.u8(*m);
                    if let Some(d) = duration {
                        w.u16(*d);
                        if let Some((bots, mask)) = autofill {
                            w.u8(*bots).u8(*mask);
                        }
                    }
                }
                w.finish()
            }
            ClientMsg::SetTeam { team } => vec![MSG_SET_TEAM, *team],
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
            MSG_JOIN => ClientMsg::Join {
                username: String::from_utf8_lossy(r.rest()).trim().to_string(),
            },
            MSG_POSITION => ClientMsg::Position {
                position: r.vec3()?,
                yaw: r.f32()?,
                state_flags: r.u8(),
                event_flags: r.u8(),
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
                ClientMsg::StartGame { mode, duration, autofill }
            }
            MSG_SET_TEAM => ClientMsg::SetTeam { team: r.u8()? },
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

#[cfg(test)]
mod tests {
    use super::*;

    fn rt_server(m: ServerMsg) {
        assert_eq!(ServerMsg::decode(&m.encode()), Some(m));
    }

    fn rt_client(m: ClientMsg) {
        assert_eq!(ClientMsg::decode(&m.encode()), Some(m));
    }

    #[test]
    fn server_messages_round_trip() {
        let ps = PlayerState {
            id: 7,
            position: (1.0, 2.0, 3.0),
            yaw: 0.5,
            world: 1,
            state_flags: FLAG_ADS | FLAG_IS_BOT,
            event_flags: EVT_LANDED,
            kills: 3,
            gun_level: 2,
            health: 80.0,
            max_health: 100.0,
            name: "alice".into(),
            alive: true,
            freeze_secs: 2,
            team: 1,
            weapon: "M4A1".into(),
        };
        rt_server(ServerMsg::State(vec![ps.clone(), PlayerState { id: 8, ..ps }]));
        rt_server(ServerMsg::Welcome { id: 1, username: "bob".into() });
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
        rt_server(ServerMsg::Discover { players: 2, max_players: 16, name: "Gridlock".into() });
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
        rt_server(ServerMsg::BotShot { shooter_id: 1, victim_id: 2, start: (0.0, 1.2, 0.0), end: (1.0, 1.0, 1.0) });
        rt_server(ServerMsg::Chat { from_id: 3, name: "alice".into(), text: "gg".into() });
        rt_server(ServerMsg::Chat { from_id: 0, name: String::new(), text: "Match ended".into() });
        rt_server(ServerMsg::PlayerJoined { id: 4, name: "bob".into() });
        rt_server(ServerMsg::MatchStatus { status: MATCH_STATUS_RUNNING });
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
        rt_client(ClientMsg::Join { username: "alice".into() });
        rt_client(ClientMsg::Position { position: (1.0, 2.0, 3.0), yaw: 0.1, state_flags: Some(1), event_flags: Some(2) });
        rt_client(ClientMsg::Position { position: (1.0, 2.0, 3.0), yaw: 0.1, state_flags: None, event_flags: None });
        rt_client(ClientMsg::Hit { victim_id: 2, damage: 30.0, weapon: Some("AK".into()), backstab: Some(false) });
        rt_client(ClientMsg::StartGame { mode: Some(3), duration: Some(0), autofill: Some((4, 0b11111)) });
        rt_client(ClientMsg::StartGame { mode: None, duration: None, autofill: None });
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
            ServerMsg::decode(&ServerMsg::Welcome { id: 1, username: long }.encode()).unwrap()
        else {
            panic!()
        };
        assert_eq!(username.len(), 255);
    }
}
