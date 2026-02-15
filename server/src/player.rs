use std::net::SocketAddr;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldId {
    Lobby = 0,
    Maze = 1,
}

impl WorldId {
    pub fn from_u8(value: u8) -> Option<WorldId> {
        match value {
            0 => Some(WorldId::Lobby),
            1 => Some(WorldId::Maze),
            _ => None,
        }
    }
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

pub struct Player {
    pub id: u32,
    pub username: String,
    pub position: (f32, f32, f32),
    pub yaw: f32,
    pub pitch: f32,
    pub health: u32,
    pub shield: u32,
    pub max_health: u32,
    pub max_shield: u32,
    pub is_alive: bool,
    pub kills: u32,
    pub deaths: u32,
    pub gun_level: u32,
    pub socket_addr: SocketAddr,
    pub last_activity: Instant,
    pub world: WorldId, // where is this player right now? lobby, maze, etc
}

impl Player {
    pub fn new(id: u32, username: String, addr: SocketAddr) -> Player {
        Player {
            id,
            username,
            position: (0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            health: 100,
            shield: 0,
            max_health: 100,
            max_shield: 100,
            is_alive: true,
            kills: 0,
            deaths: 0,
            gun_level: 1,
            socket_addr: addr,
            last_activity: Instant::now(),
            world: WorldId::Lobby, // start in the lobby by default
        }
    }
}
