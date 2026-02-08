use std::net::SocketAddr;
use std::time::Instant;

pub struct Player {
    pub id: u32,
    pub username: String,
    pub position: (f32, f32, f32),
    pub yaw: f32,
    pub pitch: f32,
    pub health: u32,
    pub is_alive: bool,
    pub kills: u32,
    pub deaths: u32,
    pub gun_level: u32,
    pub socket_addr: SocketAddr,
    pub last_activity: Instant,
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
            is_alive: true,
            kills: 0,
            deaths: 0,
            gun_level: 1,
            socket_addr: addr,
            last_activity: Instant::now(),
        }
    }
}
