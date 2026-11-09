use crate::codec::{Reader, Vec3, Writer};
use crate::*;

/// Server -> client.
#[derive(Clone, Debug, PartialEq)]
pub enum ServerMsg {
    /// `version` (`PROTOCOL_VERSION`) trails the packet: builds from before
    /// the version check never send it, and ignore it when they get it. Id 0
    /// turns the joiner away (another version); see `ClientMsg::Join`.
    Welcome { id: u32, username: String, version: Option<u16> },
    /// Everyone in the receiver's `world`.
    State { world: u8, players: Vec<PlayerState> },
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
    /// LAN discovery reply; `version` trails it as in `Welcome`.
    Discover { players: u8, max_players: u8, name: String, version: Option<u16> },
    WallBroken { x: u8, y: u8 },
    Death { killer_id: u32, weapon: String },
    Respawn { position: Vec3, kind: Option<u8> }, // kind: 0=random, 1=teleporter fallback
    Damage { amount: f32, attacker_id: u32, stun: bool },
    Kill { victim_id: u32, weapon: String },
    /// `headshot` trails the packet, so older builds still read the rest, before this kills never said if they were headshots
    KillFeed { killer_id: u32, victim_id: u32, weapon: String, headshot: bool },
    StartDenied { reason: Option<u8> }, // START_DENIED_*; None = running and full
    MatchJoined { id: u32, name: String },
    MatchEnd { winner_id: u32, winner_name: String, countdown: Option<u8> },
    PlayerList(Vec<PlayerListEntry>),
    MatchCountdown { secs: u8, mode: Option<u8> },
    MatchTimer { secs: u16 },
    SndRoundState(SndRound),
    LobbyLeader { id: u32 }, // 0 = no leader
    /// Every shot bots fired in one bot tick.
    BotShots(Vec<BotShot>),
    /// What rarely changes about players, see `PlayerInfo`.
    PlayerInfo(Vec<PlayerInfo>),
    Chat { from_id: u32, name: String, text: String }, // from_id 0 = server notice
    PlayerJoined { id: u32, name: String },
    PlayerLeft { id: u32, name: String, reason: u8 },
    MatchStatus { status: u8 },
    TeamScore { red: u16, blue: u16, to_win: u16 },
    LobbySettings(LobbySettings),
}

impl ServerMsg {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            ServerMsg::Welcome { id, username, version } => {
                let mut w = Writer::new(MSG_WELCOME);
                w.u32(*id).str8(username);
                if let Some(v) = version {
                    w.u16(*v);
                }
                w.finish()
            }
            ServerMsg::State { world, players } => {
                let count = players.len().min(u16::MAX as usize);
                let mut w = Writer::new(MSG_STATE);
                w.u8(*world).u16(count as u16);
                for p in players.iter().take(count) {
                    p.write(&mut w);
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
            ServerMsg::Discover { players, max_players, name, version } => {
                let mut w = Writer::new(MSG_DISCOVER);
                w.u8(*players).u8(*max_players).str8(name);
                if let Some(v) = version {
                    w.u16(*v);
                }
                w.finish()
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
            ServerMsg::KillFeed { killer_id, victim_id, weapon, headshot } => Writer::new(MSG_KILL_FEED)
                .u32(*killer_id)
                .u32(*victim_id)
                .str8(weapon)
                .bool(*headshot)
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
            ServerMsg::BotShots(shots) => {
                let mut w = Writer::new(MSG_BOT_SHOT);
                w.u8(shots.len().min(255) as u8);
                for shot in shots.iter().take(255) {
                    shot.write(&mut w);
                }
                w.finish()
            }
            ServerMsg::PlayerInfo(entries) => {
                let mut w = Writer::new(MSG_PLAYER_INFO);
                w.u8(entries.len().min(255) as u8);
                for e in entries.iter().take(255) {
                    e.write(&mut w);
                }
                w.finish()
            }
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
            ServerMsg::LobbySettings(s) => {
                let mut w = Writer::new(MSG_LOBBY_SETTINGS);
                s.write(&mut w);
                w.finish()
            }
            ServerMsg::TeamScore { red, blue, to_win } => {
                Writer::new(MSG_TEAM_SCORE).u16(*red).u16(*blue).u16(*to_win).finish()
            }
        }
    }

    pub fn decode(packet: &[u8]) -> Option<Self> {
        let (&msg_type, payload) = packet.split_first()?;
        let mut r = Reader::new(payload);
        let msg = match msg_type {
            MSG_WELCOME => {
                ServerMsg::Welcome { id: r.u32()?, username: r.str8()?, version: r.u16() }
            }
            MSG_STATE => {
                let world = r.u8()?;
                let count = r.u16()? as usize;
                // cap the preallocation by what the packet can hold
                let mut players = Vec::with_capacity(count.min(payload.len() / PlayerState::WIRE_BYTES));
                for _ in 0..count {
                    players.push(PlayerState::read(&mut r)?);
                }
                ServerMsg::State { world, players }
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
            MSG_DISCOVER => ServerMsg::Discover {
                players: r.u8()?,
                max_players: r.u8()?,
                name: r.str8()?,
                version: r.u16(),
            },
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
                headshot: r.bool().unwrap_or(false), // older packets have no headshot byte
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
            MSG_BOT_SHOT => {
                let mut shots = Vec::new();
                for _ in 0..r.u8()? {
                    shots.push(BotShot::read(&mut r)?);
                }
                ServerMsg::BotShots(shots)
            }
            MSG_PLAYER_INFO => {
                let mut entries = Vec::new();
                for _ in 0..r.u8()? {
                    entries.push(PlayerInfo::read(&mut r)?);
                }
                ServerMsg::PlayerInfo(entries)
            }
            MSG_CHAT => ServerMsg::Chat { from_id: r.u32()?, name: r.str8()?, text: r.str8()? },
            MSG_PLAYER_JOINED => ServerMsg::PlayerJoined { id: r.u32()?, name: r.str8()? },
            MSG_PLAYER_LEFT => {
                ServerMsg::PlayerLeft { id: r.u32()?, name: r.str8()?, reason: r.u8()? }
            }
            MSG_MATCH_STATUS => ServerMsg::MatchStatus { status: r.u8()? },
            MSG_TEAM_SCORE => ServerMsg::TeamScore { red: r.u16()?, blue: r.u16()?, to_win: r.u16()? },
            MSG_LOBBY_SETTINGS => ServerMsg::LobbySettings(LobbySettings::read(&mut r)?),
            _ => return None,
        };
        Some(msg)
    }
}
