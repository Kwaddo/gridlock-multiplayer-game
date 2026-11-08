//! Structs carried inside `ServerMsg` and `ClientMsg`.

use crate::codec::{dequantize_yaw, quantize_yaw, Reader, Vec3, Writer};

/// One entry of `MSG_STATE`: what changes from tick to tick. 18 bytes on the
/// wire: `[id:4][x,y,z: fixed-point i16 each][yaw:u16][state_flags:1]
/// [event_flags:1][health:u16][alive:1][freeze_secs:1]`. Position and yaw
/// come back quantized (see `codec::quantize_coord`, `codec::quantize_yaw`).
/// What rarely changes travels in `PlayerInfo` instead.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerState {
    pub id: u32,
    pub position: Vec3,
    pub yaw: f32,
    pub state_flags: u8,
    pub event_flags: u8,
    pub health: u16,
    pub alive: bool,
    // remaining post-teleport freeze seconds (0 = none)
    pub freeze_secs: u8,
}

impl PlayerState {
    pub const WIRE_BYTES: usize = 18;

    pub(super) fn write(&self, w: &mut Writer) {
        w.u32(self.id).pos16(self.position).u16(quantize_yaw(self.yaw));
        w.u8(self.state_flags).u8(self.event_flags).u16(self.health);
        w.bool(self.alive).u8(self.freeze_secs);
    }

    pub(super) fn read(r: &mut Reader) -> Option<Self> {
        Some(PlayerState {
            id: r.u32()?,
            position: r.pos16()?,
            yaw: dequantize_yaw(r.u16()?),
            state_flags: r.u8()?,
            event_flags: r.u8()?,
            health: r.u16()?,
            alive: r.bool()?,
            freeze_secs: r.u8()?,
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

/// One shot in `MSG_BOT_SHOT` (`[shooter:4][victim:4][start: 3 x i16][end: 3 x i16]`,
/// fixed-point like `PlayerState::position`): where a bot's tracer runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BotShot {
    pub shooter_id: u32,
    pub victim_id: u32,
    pub start: Vec3,
    pub end: Vec3,
}

impl BotShot {
    pub(super) fn write(&self, w: &mut Writer) {
        w.u32(self.shooter_id).u32(self.victim_id).pos16(self.start).pos16(self.end);
    }

    pub(super) fn read(r: &mut Reader) -> Option<Self> {
        Some(BotShot { shooter_id: r.u32()?, victim_id: r.u32()?, start: r.pos16()?, end: r.pos16()? })
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
