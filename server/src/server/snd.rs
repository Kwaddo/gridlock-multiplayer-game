use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::Instant;

use crate::player::{Player, Team, WorldId};
use crate::protocols::*;
use crate::server::broadcast;

/// S&D round phase
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SndPhase {
    /// Freeze time at start of round (players can't move)
    FreezeTime = 0,
    /// Active round — players fight
    Active = 1,
    /// Bomb has been planted, defuse team must defuse
    BombPlanted = 2,
    /// Round over — short pause before next round
    RoundOver = 3,
}

/// Search & Destroy match state (lives entirely on the server)
pub struct SndState {
    pub current_round: u8,       // 1-based (1..=max_rounds)
    pub max_rounds: u8,          // total rounds (7)
    pub red_round_wins: u8,
    pub blue_round_wins: u8,
    pub phase: SndPhase,
    pub phase_timer: f32,        // seconds remaining in current phase

    /// Which team has the bomb this round (alternates each round)
    /// Round 1: Red attacks (has bomb), Blue defends
    pub attacking_team: Team,

    /// Player ID of whoever is carrying the bomb (None = bomb is on the ground or not yet assigned)
    pub bomb_carrier: Option<u32>,

    /// If the bomb is dropped (carrier died), its world position
    pub bomb_dropped_pos: Option<(f32, f32, f32)>,

    /// Bomb site world position (center of the middle room)
    pub bomb_site: (f32, f32, f32),

    /// Whether bomb has been planted at the site
    pub bomb_planted: bool,
    /// Plant progress (0.0 to 1.0) — takes PLANT_DURATION seconds
    pub plant_progress: f32,
    /// Who is currently planting (must be bomb carrier at bomb site)
    pub planter: Option<SocketAddr>,
    /// Defuse progress (0.0 to 1.0)
    pub defuse_progress: f32,
    /// Who is currently defusing
    pub defuser: Option<SocketAddr>,

    /// Time when the bomb was planted (for detonation timer)
    pub bomb_plant_time: Option<Instant>,

    /// Players who are alive this round (reset each round)
    /// Dead players spectate the bomb site
    pub round_alive: HashMap<SocketAddr, bool>,

    /// Saved spawn positions so we can re-use them across rounds
    pub red_spawn: (f32, f32, f32),
    pub blue_spawn: (f32, f32, f32),
}

// S&D timing constants
pub const SND_MAX_ROUNDS: u8 = 7;
pub const SND_FREEZE_TIME: f32 = 5.0;
pub const SND_ROUND_TIME: f32 = 90.0;     // seconds per round
pub const SND_ROUND_OVER_TIME: f32 = 5.0; // pause between rounds
pub const SND_PLANT_DURATION: f32 = 4.0;  // seconds to plant bomb
pub const SND_DEFUSE_DURATION: f32 = 5.0; // seconds to defuse
pub const SND_BOMB_TIMER: f32 = 40.0;     // seconds after plant until detonation
pub const SND_BOMB_PICKUP_RANGE: f32 = 2.5; // world units to pick up dropped bomb
pub const SND_BOMB_SITE_RANGE: f32 = 3.5;   // world units from bomb site center to plant

impl SndState {
    pub fn new(
        bomb_site: (f32, f32, f32),
        red_spawn: (f32, f32, f32),
        blue_spawn: (f32, f32, f32),
    ) -> Self {
        Self {
            current_round: 0, // will be set to 1 on first start_round
            max_rounds: SND_MAX_ROUNDS,
            red_round_wins: 0,
            blue_round_wins: 0,
            phase: SndPhase::FreezeTime,
            phase_timer: SND_FREEZE_TIME,
            attacking_team: Team::Red, // Red starts as attacker
            bomb_carrier: None,
            bomb_dropped_pos: None,
            bomb_site,
            bomb_planted: false,
            plant_progress: 0.0,
            planter: None,
            defuse_progress: 0.0,
            defuser: None,
            bomb_plant_time: None,
            round_alive: HashMap::new(),
            red_spawn,
            blue_spawn,
        }
    }

    /// Start a new round. Returns true if the match should continue, false if a team won the match.
    pub fn start_round(
        &mut self,
        socket: &UdpSocket,
        players: &mut HashMap<SocketAddr, Player>,
    ) -> bool {
        self.current_round += 1;

        // Check if a team has won the match (first to 4 rounds in a best-of-7)
        let rounds_to_win = (self.max_rounds / 2) + 1; // 4
        if self.red_round_wins >= rounds_to_win || self.blue_round_wins >= rounds_to_win {
            return false; // match over
        }

        if self.current_round > self.max_rounds {
            return false; // match over
        }

        // Alternate attacking team each round
        self.attacking_team = if self.current_round % 2 == 1 {
            Team::Red
        } else {
            Team::Blue
        };

        // Reset round state
        self.phase = SndPhase::FreezeTime;
        self.phase_timer = SND_FREEZE_TIME;
        self.bomb_planted = false;
        self.plant_progress = 0.0;
        self.planter = None;
        self.defuse_progress = 0.0;
        self.defuser = None;
        self.bomb_plant_time = None;
        self.bomb_dropped_pos = None;
        self.bomb_carrier = None;
        self.round_alive.clear();

        // Respawn all maze players at their team spawns, reset health
        // Give bomb to first player on attacking team
        let mut first_attacker: Option<u32> = None;

        for player in players.values_mut() {
            if player.world != WorldId::Maze {
                continue;
            }
            player.is_alive = true;
            player.health = player.max_health;
            player.shield = 0;
            player.death_time = None;

            // Spawn at team position
            match player.team {
                Team::Red => player.position = self.red_spawn,
                Team::Blue => player.position = self.blue_spawn,
                _ => {}
            }

            self.round_alive.insert(player.socket_addr, true);

            // Track first attacker to give them the bomb
            if player.team == self.attacking_team && first_attacker.is_none() {
                first_attacker = Some(player.id);
            }

            // Apply freeze
            player.post_teleport_freeze_until =
                Some(Instant::now() + std::time::Duration::from_secs(SND_FREEZE_TIME as u64));

            // Send respawn to all players
            let mut respawn_msg = vec![MSG_RESPAWN];
            respawn_msg.extend_from_slice(&player.position.0.to_be_bytes());
            respawn_msg.extend_from_slice(&player.position.1.to_be_bytes());
            respawn_msg.extend_from_slice(&player.position.2.to_be_bytes());
            respawn_msg.push(0u8); // spawn kind
            let _ = socket.send_to(&respawn_msg, player.socket_addr);
        }

        // Assign bomb to first attacker
        self.bomb_carrier = first_attacker;

        // Broadcast state
        broadcast::broadcast_state(socket, players);
        broadcast::broadcast_player_list(socket, players);

        true
    }

    /// Called every server tick. Returns Some((winner_team_name, winner_id)) if the match ends.
    pub fn tick(
        &mut self,
        dt: f32,
        socket: &UdpSocket,
        players: &mut HashMap<SocketAddr, Player>,
    ) -> Option<(String, u32)> {
        self.phase_timer -= dt;

        match self.phase {
            SndPhase::FreezeTime => {
                if self.phase_timer <= 0.0 {
                    self.phase = SndPhase::Active;
                    self.phase_timer = SND_ROUND_TIME;
                }
            }
            SndPhase::Active => {
                // Update plant progress
                if let Some(planter_addr) = self.planter {
                    if let Some(p) = players.get(&planter_addr) {
                        // Verify planter is still alive, still bomb carrier, and near bomb site
                        let is_valid = p.is_alive
                            && self.bomb_carrier == Some(p.id)
                            && p.team == self.attacking_team
                            && Self::distance(p.position, self.bomb_site) <= SND_BOMB_SITE_RANGE;
                        if is_valid {
                            self.plant_progress += dt / SND_PLANT_DURATION;
                            if self.plant_progress >= 1.0 {
                                // Bomb planted!
                                self.bomb_planted = true;
                                self.phase = SndPhase::BombPlanted;
                                self.phase_timer = SND_BOMB_TIMER;
                                self.bomb_plant_time = Some(Instant::now());
                                self.bomb_carrier = None;
                                self.planter = None;
                                self.plant_progress = 1.0;
                            }
                        } else {
                            self.planter = None;
                            self.plant_progress = 0.0;
                        }
                    } else {
                        self.planter = None;
                        self.plant_progress = 0.0;
                    }
                }

                // Check win conditions: all players on one team dead
                if let Some(winner) = self.check_team_eliminated(players) {
                    self.end_round(winner, socket, players);
                }

                // Round timer expired without plant
                if self.phase_timer <= 0.0 {
                    // Defending team wins if bomb not planted
                    let defending = self.defending_team();
                    self.end_round(defending, socket, players);
                }
            }
            SndPhase::BombPlanted => {
                // Update defuse progress
                if let Some(defuser_addr) = self.defuser {
                    if let Some(p) = players.get(&defuser_addr) {
                        let is_valid = p.is_alive
                            && p.team == self.defending_team()
                            && Self::distance(p.position, self.bomb_site) <= SND_BOMB_SITE_RANGE;
                        if is_valid {
                            self.defuse_progress += dt / SND_DEFUSE_DURATION;
                            if self.defuse_progress >= 1.0 {
                                // Defused! Defending team wins
                                let defending = self.defending_team();
                                self.end_round(defending, socket, players);
                            }
                        } else {
                            self.defuser = None;
                            self.defuse_progress = 0.0;
                        }
                    } else {
                        self.defuser = None;
                        self.defuse_progress = 0.0;
                    }
                }

                // Check team elimination (attackers can still win if all defenders die)
                if let Some(winner) = self.check_team_eliminated(players) {
                    // If all defenders are dead, attackers win
                    // If all attackers are dead but bomb is planted, bomb still ticks
                    if winner == self.attacking_team {
                        // Defenders all dead, but if bomb is planted it still goes off by timer
                        // Actually attackers win immediately since no one can defuse
                        self.end_round(winner, socket, players);
                    } else if winner == self.defending_team() {
                        // All attackers dead, but bomb is planted — defenders must defuse
                        // Don't end round, let bomb timer run
                    }
                }

                // Bomb detonates
                if self.phase_timer <= 0.0 {
                    // Bomb explodes — attacking team wins
                    self.end_round(self.attacking_team, socket, players);
                }
            }
            SndPhase::RoundOver => {
                if self.phase_timer <= 0.0 {
                    // Start next round
                    if !self.start_round(socket, players) {
                        // Match is over
                        let (winner_name, winner_id) = if self.red_round_wins > self.blue_round_wins
                        {
                            ("Red Team".to_string(), 0u32)
                        } else if self.blue_round_wins > self.red_round_wins {
                            ("Blue Team".to_string(), 0u32)
                        } else {
                            ("Draw".to_string(), 0u32)
                        };
                        return Some((winner_name, winner_id));
                    }
                }
            }
        }

        // Broadcast round state
        self.broadcast_round_state(socket, players);

        None
    }

    fn end_round(&mut self, winner: Team, _socket: &UdpSocket, _players: &mut HashMap<SocketAddr, Player>) {
        match winner {
            Team::Red => self.red_round_wins += 1,
            Team::Blue => self.blue_round_wins += 1,
            _ => {}
        }
        self.phase = SndPhase::RoundOver;
        self.phase_timer = SND_ROUND_OVER_TIME;
    }

    fn defending_team(&self) -> Team {
        match self.attacking_team {
            Team::Red => Team::Blue,
            Team::Blue => Team::Red,
            _ => Team::Blue,
        }
    }

    /// Check if all players on one team are dead.
    /// Returns the winning team (the team that still has alive players).
    fn check_team_eliminated(&self, players: &HashMap<SocketAddr, Player>) -> Option<Team> {
        let mut red_alive = 0u32;
        let mut blue_alive = 0u32;

        for p in players.values() {
            if p.world != WorldId::Maze || !p.is_alive {
                continue;
            }
            match p.team {
                Team::Red => red_alive += 1,
                Team::Blue => blue_alive += 1,
                _ => {}
            }
        }

        if red_alive == 0 && blue_alive > 0 {
            Some(Team::Blue)
        } else if blue_alive == 0 && red_alive > 0 {
            Some(Team::Red)
        } else {
            None
        }
    }

    /// Handle a player death in S&D — drop bomb if they had it, mark as dead
    pub fn on_player_death(
        &mut self,
        dead_player_id: u32,
        dead_player_pos: (f32, f32, f32),
    ) {
        // If the dead player was carrying the bomb, drop it
        if self.bomb_carrier == Some(dead_player_id) {
            self.bomb_carrier = None;
            self.bomb_dropped_pos = Some(dead_player_pos);
            self.plant_progress = 0.0;
            self.planter = None;
        }

        // If the dead player was defusing, cancel
        // (defuser tracking is by SocketAddr, handled separately)
    }

    /// Handle bomb pickup request
    pub fn try_pickup_bomb(
        &mut self,
        player_addr: SocketAddr,
        players: &HashMap<SocketAddr, Player>,
    ) -> bool {
        let player = match players.get(&player_addr) {
            Some(p) => p,
            None => return false,
        };

        // Must be alive, on attacking team, and bomb must be on the ground
        if !player.is_alive || player.team != self.attacking_team {
            return false;
        }

        if let Some(bomb_pos) = self.bomb_dropped_pos {
            if Self::distance(player.position, bomb_pos) <= SND_BOMB_PICKUP_RANGE {
                self.bomb_carrier = Some(player.id);
                self.bomb_dropped_pos = None;
                return true;
            }
        }

        false
    }

    /// Handle plant start request
    pub fn try_start_plant(&mut self, player_addr: SocketAddr, players: &HashMap<SocketAddr, Player>) {
        let player = match players.get(&player_addr) {
            Some(p) => p,
            None => return,
        };

        if !player.is_alive
            || player.team != self.attacking_team
            || self.bomb_carrier != Some(player.id)
            || self.bomb_planted
            || self.phase != SndPhase::Active
        {
            return;
        }

        if Self::distance(player.position, self.bomb_site) <= SND_BOMB_SITE_RANGE {
            self.planter = Some(player_addr);
            // Don't reset progress — allows resuming if they stop briefly
        }
    }

    pub fn stop_plant(&mut self, player_addr: SocketAddr) {
        if self.planter == Some(player_addr) {
            self.planter = None;
            self.plant_progress = 0.0;
        }
    }

    /// Handle defuse start request
    pub fn try_start_defuse(&mut self, player_addr: SocketAddr, players: &HashMap<SocketAddr, Player>) {
        let player = match players.get(&player_addr) {
            Some(p) => p,
            None => return,
        };

        if !player.is_alive
            || player.team != self.defending_team()
            || !self.bomb_planted
            || self.phase != SndPhase::BombPlanted
        {
            return;
        }

        if Self::distance(player.position, self.bomb_site) <= SND_BOMB_SITE_RANGE {
            self.defuser = Some(player_addr);
        }
    }

    pub fn stop_defuse(&mut self, player_addr: SocketAddr) {
        if self.defuser == Some(player_addr) {
            self.defuser = None;
            self.defuse_progress = 0.0;
        }
    }

    /// Broadcast the S&D round state to all players
    pub fn broadcast_round_state(
        &self,
        socket: &UdpSocket,
        players: &HashMap<SocketAddr, Player>,
    ) {
        let mut msg = Vec::with_capacity(64);
        msg.push(MSG_SND_ROUND_STATE);
        msg.push(self.current_round);
        msg.push(self.max_rounds);
        msg.push(self.red_round_wins);
        msg.push(self.blue_round_wins);
        msg.push(self.phase as u8);
        msg.push(self.attacking_team.as_u8());

        // Bomb carrier ID (0 = no carrier)
        let carrier_id = self.bomb_carrier.unwrap_or(0);
        msg.extend_from_slice(&carrier_id.to_be_bytes());

        // Dropped bomb position (or zeros)
        let (bx, by, bz) = self.bomb_dropped_pos.unwrap_or((0.0, 0.0, 0.0));
        msg.extend_from_slice(&bx.to_be_bytes());
        msg.extend_from_slice(&by.to_be_bytes());
        msg.extend_from_slice(&bz.to_be_bytes());

        // Bomb site position
        msg.extend_from_slice(&self.bomb_site.0.to_be_bytes());
        msg.extend_from_slice(&self.bomb_site.1.to_be_bytes());
        msg.extend_from_slice(&self.bomb_site.2.to_be_bytes());

        // Bomb planted flag
        msg.push(if self.bomb_planted { 1u8 } else { 0u8 });

        // Plant/defuse progress
        msg.extend_from_slice(&self.plant_progress.to_be_bytes());
        msg.extend_from_slice(&self.defuse_progress.to_be_bytes());

        // Phase timer
        msg.extend_from_slice(&self.phase_timer.to_be_bytes());

        for p in players.values() {
            let _ = socket.send_to(&msg, p.socket_addr);
        }
    }

    fn distance(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
        let dx = a.0 - b.0;
        let dz = a.2 - b.2;
        (dx * dx + dz * dz).sqrt()
    }
}
