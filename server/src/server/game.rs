use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use crate::maze::MazeData;
use crate::player::Player;
use crate::player::WorldId;

// configuration constants replicated from original main.rs
pub const PLAYER_TIMEOUT: Duration = Duration::from_secs(30);
pub const MAZE_CELL_SIZE: f32 = 1.8;
pub const MAZE_WIDTH: u8 = 35;
pub const MAZE_HEIGHT: u8 = 35;
pub const MAZE_AUTO_RESPAWN_DELAY: Duration = Duration::from_secs(3);

// Match / game loop settings (basic foundation)
pub const MATCH_DURATION: Duration = Duration::from_secs(120); // match length (seconds)
pub const MIN_PLAYERS_TO_START: usize = 1; // minimal players required to start a match
pub const MATCH_MAX_PLAYERS: usize = 16; // maximum concurrent players in a running match
#[allow(dead_code)]
pub const MATCH_SCORE_TO_WIN: u32 = 20; // kills required to win the match (unused for Gun Game)

pub const MATCH_START_COUNTDOWN_SECS: u8 = 5; // seconds shown to players before a match actually starts // seconds shown to players before a match actually starts
pub const MATCH_END_COUNTDOWN_SECS: u8 = 6; // seconds to wait after a win before returning players to lobby
pub const MATCH_POST_TELEPORT_FREEZE_SECS: u64 = 10; // seconds players cannot move right after teleport into Maze

// --- Gun Game configuration (single-mode for now)
// Customize this progression to change the sequence of weapons for Gun Game.
// Names must match client-side `WeaponStats.name` values. Keep this in
// sync with the client `WeaponStats.name` strings (see
// client/src/weapons/stats/*).
#[allow(dead_code)]
pub const GUN_GAME_PROGRESSION: [&str; 19] = [
    // Pistols
    "M1911",
    "Desert Eagle",
    "Revolver",
    "Suppressed Pistol",
    // SMGs
    "MP5",
    "P90",
    "Tommy Gun",
    // Shotguns
    "Benelli",
    "SPAS-12",
    // Assault Rifles
    "M4A1",
    "AK-47",
    "FAMAS",
    "M16",
    "SCAR-H",
    "AS VAL",
    // Snipers
    "Bolt Action",
    "Hunting Rifle",
    "Semi-Auto Sniper",
    // Other
    "Minigun",
];

/// iterate over all alive/dead players and automatically respawn as needed
pub fn process_auto_respawns(
    socket: &UdpSocket,
    maze: &MazeData,
    players: &mut HashMap<SocketAddr, Player>,
) {
    let now = Instant::now();
    // include a bool indicating whether the teleporter fallback was used
    let mut to_respawn: Vec<(SocketAddr, f32, f32, f32, bool, String)> =
        Vec::with_capacity(players.len());

    for player in players.values_mut() {
        if player.is_alive || player.world != WorldId::Maze {
            continue;
        }

        let Some(death_time) = player.death_time else {
            continue;
        };

        if now.duration_since(death_time) >= MAZE_AUTO_RESPAWN_DELAY {
            player.is_alive = true;
            player.health = player.max_health;
            player.shield = 0;
            player.death_time = None;

            // choose a random spawn inside the maze (may fallback to teleporter)
            let (sx, sy, sz, used_fallback) = maze.random_spawn_world(MAZE_CELL_SIZE);
            player.position = (sx, sy, sz);
            to_respawn.push((
                player.socket_addr,
                sx,
                sy,
                sz,
                used_fallback,
                player.username.clone(),
            ));
        }
    }

    for (socket_addr, sx, sy, sz, used_fallback, username) in to_respawn {
        println!(
            "Player '{}' auto-respawned in {:?}",
            username,
            WorldId::Maze
        );
        let mut respawn_msg = vec![crate::protocols::MSG_RESPAWN];
        respawn_msg.extend_from_slice(&sx.to_be_bytes());
        respawn_msg.extend_from_slice(&sy.to_be_bytes());
        respawn_msg.extend_from_slice(&sz.to_be_bytes());
        // extra byte: 0 = random spawn, 1 = teleporter fallback
        respawn_msg.push(if used_fallback { 1u8 } else { 0u8 });
        let _ = socket.send_to(&respawn_msg, socket_addr);
    }
}

/// Remove players whose last activity timestamp is older than the timeout constant.
/// Returns true if any players were removed.
pub fn cleanup_inactive_players(players: &mut HashMap<SocketAddr, Player>) -> bool {
    let now = Instant::now();
    let mut to_remove = Vec::new();

    for (addr, player) in players.iter() {
        if now.duration_since(player.last_activity) > PLAYER_TIMEOUT {
            to_remove.push(*addr);
        }
    }

    let removed_any = !to_remove.is_empty();
    for addr in to_remove {
        if let Some(p) = players.remove(&addr) {
            println!(
                "Player '{}' timed out. Total players: {}",
                p.username,
                players.len()
            );
        }
    }
    removed_any
}
