use std::collections::HashMap;
use crate::net::{Outbox, PeerId};
use std::time::Duration;
use platform::Instant;

use crate::maze::MazeData;
use crate::player::Player;
use crate::player::WorldId;

// configuration constants replicated from original main.rs
// Clients send at least a keepalive every 4 s (client/src/app/runner.rs), so a
// player silent this long is gone (e.g. a force-closed browser).
pub const PLAYER_TIMEOUT: Duration = Duration::from_secs(10);
// World state goes out at most this often (30 a second), however fast the
// server is ticked: the browser host ticks every frame (up to 144 a second)
// and the native host after every packet. Rarer events still send at once.
pub const STATE_BROADCAST_INTERVAL: Duration = Duration::from_millis(33);
pub const MAZE_CELL_SIZE: f32 = 1.8;
pub const MAZE_WIDTH: u8 = 35;
pub const MAZE_HEIGHT: u8 = 35;
pub const MAZE_AUTO_RESPAWN_DELAY: Duration = Duration::from_secs(3);

// Match / game loop settings (basic foundation)
// match length; `None` = infinite, `Some(d)` = end matches after `d` elapses
pub const MATCH_DURATION: Option<Duration> = Some(Duration::from_secs(900)); // TDM default: 15 minutes
pub const MIN_PLAYERS_TO_START: usize = 1; // minimal players required to start a match
pub const MATCH_MAX_PLAYERS: usize = 16; // maximum concurrent players in a running match
pub const MATCH_SCORE_TO_WIN: u32 = 100; // kills required to win the match (used for TDM)

pub const MATCH_START_COUNTDOWN_SECS: u8 = 5; // seconds shown to players before a match actually starts // seconds shown to players before a match actually starts
pub const MATCH_END_COUNTDOWN_SECS: u8 = 6; // seconds to wait after a win before returning players to lobby
pub const MATCH_POST_TELEPORT_FREEZE_SECS: u64 = 3; // seconds players cannot move right after teleport into Maze

// --- Gun Game configuration (single-mode for now)
// Customize this progression to change the sequence of weapons for Gun Game.
// Names must match client-side `WeaponStats.name` values. Keep this in
// sync with the client `WeaponStats.name` strings (see
// client/src/weapons/stats/*).
pub const GUN_GAME_PROGRESSION: [&str; 16] = [
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
];

// Team loadouts: list of primary weapons available per team. Knife is always
// implicitly available as a melee weapon.
pub const TEAM_LOADOUT_RED: [&str; 5] = ["M4A1", "MP5", "Benelli", "Desert Eagle", "Bolt Action"];

pub const TEAM_LOADOUT_BLUE: [&str; 5] =
    ["AK-47", "P90", "SPAS-12", "Revolver", "Bolt Action"];

use crate::player::Team;

/// Kills each team has scored in the current match. Counted when the kill
/// happens, so a player leaving doesn't take their kills off the team.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TeamScore {
    pub red: u32,
    pub blue: u32,
}

impl TeamScore {
    /// A kill by `killer` of `victim`; team kills and spectators don't score.
    pub fn record_kill(&mut self, killer: Team, victim: Team) {
        if killer == victim {
            return;
        }
        match killer {
            Team::Red => self.red += 1,
            Team::Blue => self.blue += 1,
            Team::Spectator => {}
        }
    }

    /// The team at or past `MATCH_SCORE_TO_WIN`, if any.
    pub fn winner(&self) -> Option<Team> {
        if self.red >= MATCH_SCORE_TO_WIN && self.red >= self.blue {
            Some(Team::Red)
        } else if self.blue >= MATCH_SCORE_TO_WIN {
            Some(Team::Blue)
        } else {
            None
        }
    }
}

/// The host: the human player with the lowest ID (the first to join), in the lobby
/// or in a match, so it stays the same while a match runs. Only the host starts
/// and ends matches; anyone may join one that is running. None when no human is
/// connected.
pub fn host_addr(players: &HashMap<PeerId, Player>) -> Option<PeerId> {
    players
        .iter()
        .filter(|(_, p)| !p.is_bot)
        .min_by_key(|(_, p)| p.id)
        .map(|(addr, _)| *addr)
}

/// Auto-assign balanced teams (Red/Blue) to all players currently in the lobby.
/// Uses a deterministic ordering (player ID ascending) and alternates assignments
/// so all clients can predict the same result.
pub fn auto_assign_lobby_teams(players: &mut HashMap<PeerId, Player>) {
    let mut lobby_pairs: Vec<(PeerId, u32)> = players
        .iter()
        .filter_map(|(addr, p)| {
            if p.world == WorldId::Lobby && !p.is_bot {
                Some((*addr, p.id))
            } else {
                None
            }
        })
        .collect();
    lobby_pairs.sort_by_key(|(_, id)| *id);

    for (i, (addr, _)) in lobby_pairs.iter().enumerate() {
        if let Some(p) = players.get_mut(addr) {
            p.team = if i % 2 == 0 { Team::Red } else { Team::Blue };
        }
    }
}

/// Choose a weapon for `player` from their team's loadout attempting to avoid
/// duplicates currently assigned to teammates when possible.
pub fn choose_team_weapon_for_player(
    players: &std::collections::HashMap<PeerId, crate::player::Player>,
    player: &crate::player::Player,
) -> String {
    let pool: &[&str] = match player.team {
        Team::Red => &TEAM_LOADOUT_RED,
        Team::Blue => &TEAM_LOADOUT_BLUE,
        Team::Spectator => return "Knife".to_string(),
    };

    // collect weapons already used by teammates
    let mut used = std::collections::HashSet::new();
    for p in players.values() {
        if p.team == player.team {
            if !p.current_weapon.is_empty() {
                used.insert(p.current_weapon.clone());
            }
        }
    }

    // pick the first unused weapon from pool
    for &w in pool.iter() {
        if !used.contains(w) {
            return w.to_string();
        }
    }

    // all weapons used by teammates (or pool empty) — fall back to first
    pool.get(0)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Knife".to_string())
}

/// iterate over all alive/dead players and automatically respawn as needed
/// Choose a spawn point in the maze that is at least `min_dist` away
/// from every other player.  Falls back to a normal random spawn if no
/// suitable spot is found after a few tries.
pub fn random_spawn_far(
    maze: &MazeData,
    players: &HashMap<PeerId, Player>,
    min_dist: f32,
) -> (f32, f32, f32, bool) {
    // repeatedly sample candidate locations and test distance
    let mut best = maze.random_spawn_world(MAZE_CELL_SIZE);
    let min2 = min_dist * min_dist;
    for _ in 0..8 {
        let cand = maze.random_spawn_world(MAZE_CELL_SIZE);
        let (cx, _cy, cz, _used) = cand;
        // check squared distance to every live player
        let ok = players.values().all(|p| {
            let dx = p.position.0 - cx;
            let dz = p.position.2 - cz;
            dx * dx + dz * dz >= min2
        });
        if ok {
            best = cand;
            break;
        }
    }
    best
}

pub fn process_auto_respawns(
    socket: &Outbox,
    maze: Option<&MazeData>,
    players: &mut HashMap<PeerId, Player>,
    match_mode: u8,
) {
    let now = Instant::now();
    // include a bool indicating whether the teleporter fallback was used
    let mut to_respawn: Vec<(PeerId, f32, f32, f32, bool, String)> =
        Vec::with_capacity(players.len());

    // first pass: gather respawn candidates without mutating `players`
    let mut spawn_info: Vec<(PeerId, (f32, f32, f32, bool))> = Vec::new();
    for (addr, player) in players.iter() {
        if !player.auto_respawn || player.is_alive {
            continue;
        }
        if let Some(death_time) = player.death_time {
            if now.duration_since(death_time) < MAZE_AUTO_RESPAWN_DELAY {
                continue;
            }
            let spawn = match (player.world, maze) {
                (WorldId::Maze, Some(m)) => match player.team {
                    crate::player::Team::Red => {
                        let (red, _blue) = m.team_spawn_positions(MAZE_CELL_SIZE);
                        (red.0, red.1, red.2, false)
                    }
                    crate::player::Team::Blue => {
                        let (_red, blue) = m.team_spawn_positions(MAZE_CELL_SIZE);
                        (blue.0, blue.1, blue.2, false)
                    }
                    _ => random_spawn_far(m, players, MAZE_CELL_SIZE * 3.0),
                },
                _ => (0.0_f32, 0.5_f32, 0.0_f32, false),
            };
            spawn_info.push((*addr, spawn));
        }
    }

    // second pass: perform mutations and collect respawn messages
    for &(addr, (sx, sy, sz, used_fallback)) in &spawn_info {
        if let Some(player) = players.get_mut(&addr) {
            player.is_alive = true;
            player.health = player.max_health;
            player.death_time = None;
            player.position = (sx, sy, sz);
            to_respawn.push((
                addr,
                sx,
                sy,
                sz,
                used_fallback,
                player.username.clone(),
            ));
        }
    }

    // third pass: send respawn packets and update weapons
    for (socket_addr, sx, sy, sz, used_fallback, username) in to_respawn.iter() {
        log!(
            "Player '{}' auto-respawned in {:?}",
            username,
            WorldId::Maze
        );
        // extra byte: 0 = random spawn, 1 = teleporter fallback
        let respawn_msg = crate::protocols::ServerMsg::Respawn {
            position: (*sx, *sy, *sz),
            kind: Some(*used_fallback as u8),
        }
        .encode();
        // choose weapon for the respawned player based on mode
        let chosen_weapon = if let Some(pref) = players.get(socket_addr) {
            if match_mode == crate::protocols::MODE_GUN_GAME {
                let gl = pref.gun_level as usize;
                if gl >= 1 && gl <= GUN_GAME_PROGRESSION.len() {
                    GUN_GAME_PROGRESSION[gl - 1].to_string()
                } else if gl > GUN_GAME_PROGRESSION.len() {
                    "Punch".to_string()
                } else {
                    GUN_GAME_PROGRESSION[0].to_string()
                }
            } else if pref.team != Team::Spectator {
                // TDM/S&D: keep current weapon, or pick from team loadout
                if !pref.current_weapon.is_empty() {
                    pref.current_weapon.clone()
                } else {
                    choose_team_weapon_for_player(players, pref)
                }
            } else {
                "Knife".to_string()
            }
        } else {
            "Knife".to_string()
        };
        if let Some(pmut) = players.get_mut(socket_addr) {
            pmut.current_weapon = chosen_weapon;
        }

        if let Some(p) = players.get(socket_addr) {
            if !p.is_bot {
                let _ = socket.send_to(&respawn_msg, socket_addr);
            }
        }
    }
}

/// Regenerate player health after 7 seconds out of combat.
/// Applies to all worlds; combat resets the timer via regen_resume_at.
pub fn process_lobby_regen(players: &mut HashMap<PeerId, Player>) {
    let now = Instant::now();
    for player in players.values_mut() {
        if player.is_bot || !player.is_alive {
            // In lobby, keep human players alive (no real death)
            if player.world == WorldId::Lobby && !player.is_bot && !player.is_alive {
                player.is_alive = true;
            }
            continue;
        }
        if player.health == 0 {
            player.health = 1;
        }
        if player.health >= player.max_health {
            continue;
        }
        if let Some(resume_at) = player.regen_resume_at {
            if now < resume_at {
                continue;
            }
        }
        player.health = (player.health + 1).min(player.max_health);
    }
}

/// Remove players whose last activity timestamp is older than the timeout constant.
/// Returns the players removed.
pub fn cleanup_inactive_players(players: &mut HashMap<PeerId, Player>) -> Vec<Player> {
    let now = Instant::now();
    let mut to_remove = Vec::new();

    for (addr, player) in players.iter() {
        if player.is_bot {
            continue;
        }
        if now.duration_since(player.last_activity) > PLAYER_TIMEOUT {
            to_remove.push(*addr);
        }
    }

    let mut removed = Vec::with_capacity(to_remove.len());
    for addr in to_remove {
        if let Some(p) = players.remove(&addr) {
            log!(
                "Player '{}' timed out. Total players: {}",
                p.username,
                players.len()
            );
            removed.push(p);
        }
    }
    removed
}
