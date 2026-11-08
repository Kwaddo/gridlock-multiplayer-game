//! Structs carried inside `ServerMsg` and `ClientMsg`.

use crate::codec::{Reader, Vec3, Writer};

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
