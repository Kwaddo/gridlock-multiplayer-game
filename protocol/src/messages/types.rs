//! Structs carried inside `ServerMsg` and `ClientMsg`.

use crate::codec::{dequantize_pitch, dequantize_yaw, quantize_pitch, quantize_yaw, Reader, Vec3, Writer};

/// One entry of `MSG_STATE`: what changes from tick to tick. 19 bytes on the
/// wire: `[id:4][x,y,z: fixed-point i16 each][yaw:u16][pitch:i8 degrees][state_flags:1]
/// [event_flags:1][health:u16][alive:1][freeze_secs:1]`. Position, yaw and pitch
/// come back quantized (see `codec::quantize_coord`, `codec::quantize_yaw`, `codec::quantize_pitch`).
/// What rarely changes travels in `PlayerInfo` instead.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerState {
    pub id: u32,
    pub position: Vec3,
    pub yaw: f32,
    // look up/down in radians, + = up, so other players see where you aim
    pub pitch: f32,
    pub state_flags: u8,
    pub event_flags: u8,
    pub health: u16,
    pub alive: bool,
    // remaining post-teleport freeze seconds (0 = none)
    pub freeze_secs: u8,
}

impl PlayerState {
    pub const WIRE_BYTES: usize = 19;

    pub(super) fn write(&self, w: &mut Writer) {
        w.u32(self.id).pos16(self.position).u16(quantize_yaw(self.yaw));
        w.u8(quantize_pitch(self.pitch));
        w.u8(self.state_flags).u8(self.event_flags).u16(self.health);
        w.bool(self.alive).u8(self.freeze_secs);
    }

    pub(super) fn read(r: &mut Reader) -> Option<Self> {
        Some(PlayerState {
            id: r.u32()?,
            position: r.pos16()?,
            yaw: dequantize_yaw(r.u16()?),
            pitch: dequantize_pitch(r.u8()?),
            state_flags: r.u8()?,
            event_flags: r.u8()?,
            health: r.u16()?,
            alive: r.bool()?,
            freeze_secs: r.u8()?,
        })
    }
}

/*
    one cardboard box in msg_boxes (id, x,y,z,yaw,carrier)
    14 bytes, server ownns every boxm client draws what it last sent
    Carried box sits on its carrier so position is where the server last saw it
    clients should draw at the carrier instead
*/

#[derive(Clone,Copy,Debug,PartialEq)]
pub struct CardboardBox {
    pub id: u16,
    pub position: Vec3,
    pub yaw: f32,
    // player inside, 0 = empty (player ids start at 1, same as LobbyLeader)
    pub carrier: u32,
}

impl CardboardBox {
    pub(super) fn write(&self, w: &mut Writer) {
        // 14 bytes: [id:2][x,y,z:3x i16][yaw:u16][carrier:u32]
        w.u16(self.id).pos16(self.position).u16(quantize_yaw(self.yaw)).u32(self.carrier);    }

    pub(super) fn read(r: &mut Reader) -> Option<Self> {
        Some(CardboardBox {
            id: r.u16()?,
            position: r.pos16()?,
            yaw: dequantize_yaw(r.u16()?),
            carrier: r.u32()?,
        })
    }
}

/// One entry of `MSG_PLAYER_INFO`: what `MSG_STATE` leaves out because it
/// rarely changes. `[id:4][name:str8][team:1][weapon:str8][kills:4]
/// [gun_level:1][max_health:u16]`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerInfo {
    pub id: u32,
    pub name: String,
    pub team: u8,
    // the weapon in hand, drawn on the world model; empty = none
    pub weapon: String,
    pub kills: u32,
    pub gun_level: u8,
    pub max_health: u16,
}

impl PlayerInfo {
    pub(super) fn write(&self, w: &mut Writer) {
        w.u32(self.id).str8(&self.name).u8(self.team).str8(&self.weapon);
        w.u32(self.kills).u8(self.gun_level).u16(self.max_health);
    }

    pub(super) fn read(r: &mut Reader) -> Option<Self> {
        Some(PlayerInfo {
            id: r.u32()?,
            name: r.str8()?,
            team: r.u8()?,
            weapon: r.str8()?,
            kills: r.u32()?,
            gun_level: r.u8()?,
            max_health: r.u16()?,
        })
    }
}

/// One shot in `MSG_SHOTS` (`[shooter:4][victim:4][start: 3 x i16][end: 3 x i16]`,
/// fixed-point like `PlayerState::position`): where a bot's or another player's tracer
/// runs. `victim` is 0 for a player's shot (the server does not know what it hit).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    pub shooter_id: u32,
    pub victim_id: u32,
    pub start: Vec3,
    pub end: Vec3,
}

impl Shot {
    pub(super) fn write(&self, w: &mut Writer) {
        w.u32(self.shooter_id).u32(self.victim_id).pos16(self.start).pos16(self.end);
    }

    pub(super) fn read(r: &mut Reader) -> Option<Self> {
        Some(Shot { shooter_id: r.u32()?, victim_id: r.u32()?, start: r.pos16()?, end: r.pos16()? })
    }
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

/// `MSG_SND_ROUND_STATE` (48-byte payload; packets from older servers stop
/// one byte short, before `bomb_dropped`).
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
    // dropped bomb position, only meaningful while `bomb_dropped`
    pub bomb_pos: Vec3,
    pub bomb_site: Vec3,
    pub bomb_planted: bool,
    pub plant_progress: f32,
    pub defuse_progress: f32,
    pub phase_timer: f32,
    /// The bomb lies on the floor at `bomb_pos`. Older servers never drop it
    /// and leave the byte out, which reads as false.
    pub bomb_dropped: bool,
}

/*
    smoke break round as one player sees it, 18 bytes
    sent to each player on its own cause side is theirs, on change and every second
*/
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmokeRound {
    pub round: u16,
    pub phase: u8, // SMOKE_PHASE_*
    pub phase_timer: f32, // seconds left in this phase
    pub coins_left: u16,
    pub side: u8, // SMOKE_SIDE_*
    // how many rounds this match plays
    pub max_rounds: u8,
    // a hider who hasn't gone golden yet this round and still can
    pub golden_ready: bool,
    // the side that won the round once it's over (SMOKE_SIDE_*), SMOKE_NO_WINNER till then
    pub winner: u8,
    // you were found this round: you hunt but can't gain points
    pub found: bool,
    // seconds your gun stays locked, 0 = ready
    pub gun_lock: f32,
}

impl SmokeRound {
    pub(super) fn write(&self, w: &mut Writer) {
        w.u16(self.round).u8(self.phase).f32(self.phase_timer).u16(self.coins_left);
        w.u8(self.side).u8(self.max_rounds).bool(self.golden_ready).u8(self.winner);
        w.bool(self.found).f32(self.gun_lock);
    }

    pub(super) fn read(r: &mut Reader) -> Option<Self> {
        Some(SmokeRound {
            round: r.u16()?,
            phase: r.u8()?,
            phase_timer: r.f32()?,
            coins_left: r.u16()?,
            side: r.u8()?,
            max_rounds: r.u8()?,
            golden_ready: r.bool()?,
            winner: r.u8()?,
            found: r.bool()?,
            gun_lock: r.f32()?,
        })
    }
}

// one row of the smoke break scoreboard, 12 bytes
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmokeScore {
    pub id: u32,
    pub points: i32,
    pub side: u8,
    // what this round has added so far, for the vote board
    pub round_points: i16,
    // the hunter multiplier in tenths, 10 is x1.0, 3 is x0.3
    pub mult_tenths: u8,
}

impl SmokeScore {
    pub(super) fn write(&self, w: &mut Writer) {
        w.u32(self.id).u32(self.points as u32).u8(self.side).u16(self.round_points as u16).u8(self.mult_tenths);
    }

    pub(super) fn read(r: &mut Reader) -> Option<Self> {
        Some(SmokeScore {
            id: r.u32()?,
            points: r.u32()? as i32,
            side: r.u8()?,
            round_points: r.u16()? as i16,
            mult_tenths: r.u8()?,
        })
    }
}

/// What the host has picked in the lobby console for the next match. The
/// host sends it as it changes; the server passes it to everyone so the
/// other players can see what is coming.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LobbySettings {
    pub mode: u8,
    /// seconds; 0 = mode default, 65535 = unlimited (as in `StartGame`)
    pub duration: u16,
    /// bots on Red, or every bot in Gun Game
    pub red_bots: u8,
    pub blue_bots: u8,
    /// bit0 = Recruit .. bit4 = Nightmare
    pub diff_mask: u8,
}

impl LobbySettings {
    pub(super) fn write(&self, w: &mut Writer) {
        w.u8(self.mode).u16(self.duration).u8(self.red_bots).u8(self.blue_bots).u8(self.diff_mask);
    }

    pub(super) fn read(r: &mut Reader) -> Option<Self> {
        Some(LobbySettings {
            mode: r.u8()?,
            duration: r.u16()?,
            red_bots: r.u8()?,
            blue_bots: r.u8()?,
            diff_mask: r.u8()?,
        })
    }
}
