use std::{ collections::HashMap, net::{SocketAddr, UdpSocket}, time::{ Duration, Instant } };
use crate::{ bot::ai::{ self, BotState, BotBehavior }, physics::PhysicsConfig };
use crate::player::{ Player, Team, WorldId };
use crate::bot::shooting;
use crate::bot::behavior;
use crate::bot::personality::{ self, BotPersonalityId, BotArchetype };
use crate::protocols::{MSG_BOT_SHOT, MODE_TEAM_DEATHMATCH};
use crate::server::handlers::other;
pub struct BotManager {
    bot_addrs: Vec<SocketAddr>,
    bot_states: HashMap<SocketAddr, BotState>,
    last_tick: Instant,
    tick_interval: Duration,
    next_bot_idx: u16,
    range_challenge_deadline: Option<Instant>,
}

impl BotManager {
    pub fn new() -> Self {
        Self {
            bot_addrs: Vec::new(),
            bot_states: HashMap::new(),
            last_tick: Instant::now(),
            tick_interval: Duration::from_millis(33), // 33 ticks per second
            next_bot_idx: 0,
            range_challenge_deadline: None,
        }
    }
    pub fn virtual_addr(index: u16) -> SocketAddr {
        // generate unique virtual SocketAddr for each bot based on index
        SocketAddr::from(([127, 0, 0, 1], 50000 + index))
    }

    pub fn is_bot(&self, addr: &SocketAddr) -> bool {
        self.bot_states.contains_key(addr) || self.bot_addrs.contains(addr)
    }
    pub fn bot_states_mut(&mut self) -> &mut HashMap<SocketAddr, BotState> {
        &mut self.bot_states
    }
    pub fn spawn_bot(
        &mut self,
        players: &mut HashMap<SocketAddr, Player>,
        mut player: Player,
        bot_state: BotState
    ) -> SocketAddr {
        let addr = Self::virtual_addr(self.next_bot_idx);
        self.next_bot_idx += 1;

        player.is_bot = true;
        player.is_silent = bot_state.personality.archetype == crate::bot::personality::BotArchetype::KnifeSneaker;
        player.socket_addr = addr;
        players.insert(addr, player);

        self.bot_states.insert(addr, bot_state);
        self.bot_addrs.push(addr);

        addr
    }

    pub fn despawn_bot(&mut self, players: &mut HashMap<SocketAddr, Player>, addr: SocketAddr) {
        players.remove(&addr);
        self.bot_states.remove(&addr);
        self.bot_addrs.retain(|&a| a != addr);
    }
    pub fn despawn_all(&mut self, players: &mut HashMap<SocketAddr, Player>) {
        for addr in &self.bot_addrs {
            players.remove(addr);
        }
        self.bot_states.clear();
        self.bot_addrs.clear();
        self.range_challenge_deadline = None;
    }
    /// Spawn a bot with a full personality into a match.
    /// Returns the virtual SocketAddr assigned to the bot.
    pub fn spawn_personality_bot(
        &mut self,
        players: &mut HashMap<SocketAddr, Player>,
        personality_id: BotPersonalityId,
        team: Team,
        weapon_override: &str,
        spawn_pos: (f32, f32, f32),
    ) -> SocketAddr {
        let (pers, diff) = personality::get_personality(personality_id);
        let runtime = personality::compute_runtime(&pers, diff);
        let bot_id = 9000u32 + self.next_bot_idx as u32;

        let bot_player = Player {
            id: bot_id,
            username: pers.name.to_string(),
            position: spawn_pos,
            yaw: 0.0,
            pitch: 0.0,
            health: 125,
            max_health: 125,
            is_alive: true,
            kills: 0,
            deaths: 0,
            gun_level: 1,
            socket_addr: Self::virtual_addr(0), // spawn_bot will overwrite
            last_activity: Instant::now(),
            death_time: None,
            world: WorldId::Maze,
            post_teleport_freeze_until: Some(
                Instant::now()
                    + Duration::from_secs(crate::server::game::MATCH_POST_TELEPORT_FREEZE_SECS),
            ),
            is_sprinting: false,
            is_ads: false,
            is_crouched: false,
            is_airborne: false,
            is_prone: false,
            is_sliding: false,
            evt_jump_start: false,
            evt_landed: false,
            team,
            current_weapon: weapon_override.to_string(),
            is_bot: true,
            is_silent: false, // spawn_bot will set based on archetype
            auto_respawn: true,
            regen_resume_at: None,
        };

        let bot_state = BotState {
            movement: crate::physics::BotMovementState {
                velocity: glam::Vec3::ZERO,
                is_grounded: true,
                height: 1.6,
            },
            behavior: BotBehavior::Static, // decide_behavior will pick on first tick
            can_shoot: true,
            shooting: shooting::BotShootingState {
                fire_rate: 0.0,
                has_target: false,
                reaction_remaining: runtime.reaction_time_mult,
            },
            shooting_config: shooting::BotShootingConfig {
                fire_rate: pers.base_reaction_time * 1.5, // rough fire interval from personality
                angular_offset: runtime.aim_offset_mult,
                reaction_time: runtime.reaction_time_mult,
                detection_range: Some(pers.range_max),
            },
            target_id: None,
            reaction_timer: 0.0,
            personality: pers,
            difficulty: diff,
            archetype: pers.archetype,
            personality_id: Some(personality_id),
            last_known_enemy_pos: None,
            behavior_decision_timer: 0.0,
            hits_received_since_teleport: 0,
        };

        let addr = self.spawn_bot(players, bot_player, bot_state);
        println!("Spawned bot '{}' ({:?}) on {:?} with {}", pers.name, diff, team, weapon_override);
        addr
    }

    pub fn set_range_challenge_deadline(&mut self, deadline: Option<Instant>) {
        self.range_challenge_deadline = deadline;
    }
    pub fn handle_range_challenge_timeout(&mut self, players: &mut HashMap<SocketAddr, Player>) -> bool {
        let Some(deadline) = self.range_challenge_deadline else {
            return false;
        };
        if Instant::now() < deadline {
            return false;
        }
        self.despawn_all(players);
        true
    }
    pub fn tick(
        &mut self,
        socket: &UdpSocket,
        players: &mut HashMap<SocketAddr, Player>,
        match_end_pending: &mut Option<(Instant, u32, String)>,
        match_mode: u8,
        maze: Option<&crate::maze::MazeData>,
    ) {
        if self.last_tick.elapsed() < self.tick_interval {
            return;
        } else {
            self.last_tick = Instant::now();
        }
        let dt = self.tick_interval.as_secs_f32();

        // ── Brain: run decide_behavior for maze-world bots ──
        // Collect behavior changes first, then apply (avoids borrow conflicts).
        if let Some(m) = maze {
            let mut behavior_changes: Vec<(SocketAddr, ai::BotBehavior)> = Vec::new();
            for addr in &self.bot_addrs {
                let bot = match players.get(addr) {
                    Some(p) if p.is_alive && p.world == WorldId::Maze => p,
                    _ => continue,
                };
                if match_mode == MODE_TEAM_DEATHMATCH
                    && bot
                        .post_teleport_freeze_until
                        .map_or(false, |t| t > Instant::now())
                {
                    continue;
                }
                let state = match self.bot_states.get(addr) {
                    Some(s) => s,
                    None => continue,
                };
                // only run for bots with a personality_id (maze bots, not shooting range bots)
                if state.personality_id.is_none() {
                    continue;
                }
                if let Some(new_behavior) = behavior::decide_behavior(bot, state, players, m, &m.analysis) {
                    behavior_changes.push((*addr, new_behavior));
                }
            }
            // apply behavior changes
            for (addr, new_behavior) in behavior_changes {
                if let Some(state) = self.bot_states.get_mut(&addr) {
                    state.behavior = new_behavior;
                }
            }
        }

        for addr in &self.bot_addrs {
            if !players.get(addr).map_or(false, |p| p.is_alive) {
                continue;
            }
            let is_frozen = players.get(addr).map_or(false, |p| {
                match_mode == MODE_TEAM_DEATHMATCH
                    && p.world == WorldId::Maze
                    && p.post_teleport_freeze_until.map_or(false, |t| t > Instant::now())
            });
            let target_pos = self
                .bot_states
                .get(addr)
                .and_then(|s| s.target_id)
                .and_then(|target_id| players.values().find(|p| p.id == target_id).map(|p| p.position));
            if let Some(bot) = players.get_mut(addr) {
                if let Some(state) = self.bot_states.get_mut(addr) {
                    if is_frozen {
                        state.movement.velocity.x = 0.0;
                        state.movement.velocity.z = 0.0;
                        continue;
                    }
                    let maze_cells = maze.map(|m| (m.cells.as_slice(), m.width, m.height));
                    ai::tick_bot(bot, state, dt, &PhysicsConfig::default(), maze_cells);
                    if let Some(target_pos) = target_pos {
                        // remember last known enemy position for investigation behavior
                        state.last_known_enemy_pos = Some(target_pos);
                        let dx = target_pos.0 - bot.position.0;
                        let dz = target_pos.2 - bot.position.2;
                        if dx * dx + dz * dz > 0.0001 {
                            bot.yaw = dz.atan2(dx);
                        }
                    }
                }
            }
        }

        // Run shooting logic after movement updates; collect first to avoid borrow conflicts.
        let mut pending_shots: Vec<(SocketAddr, u32, Option<u32>, String)> = Vec::new();
        for addr in &self.bot_addrs {
            if !players.get(addr).map_or(false, |p| p.is_alive) {
                continue;
            }
            let is_frozen = players.get(addr).map_or(false, |p| {
                match_mode == MODE_TEAM_DEATHMATCH
                    && p.world == WorldId::Maze
                    && p.post_teleport_freeze_until.map_or(false, |t| t > Instant::now())
            });
            if is_frozen {
                continue;
            }
            if let Some(state) = self.bot_states.get_mut(addr) {
                if !state.can_shoot {
                    continue;
                }
                if let Some(decision) = shooting::try_shoot(*addr, players, state, maze, dt) {
                    let weapon = players
                        .get(addr)
                        .map(|p| p.current_weapon.clone())
                        .unwrap_or_else(|| "Unknown".to_string());
                    pending_shots.push((*addr, decision.target_id, decision.damage, weapon));
                }
            }
        }

        for (attacker_addr, victim_id, damage_opt, weapon) in pending_shots {
            let shooter_id = players.get(&attacker_addr).map(|p| p.id).unwrap_or(0);
            let shooter_pos = players
                .get(&attacker_addr)
                .map(|p| p.position)
                .unwrap_or((0.0, 0.0, 0.0));
            let victim_pos = players
                .values()
                .find(|p| p.id == victim_id)
                .map(|p| p.position)
                .unwrap_or((0.0, 0.0, 0.0));
            let mut shot_msg = vec![MSG_BOT_SHOT];
            shot_msg.extend_from_slice(&shooter_id.to_be_bytes());
            shot_msg.extend_from_slice(&victim_id.to_be_bytes());
            shot_msg.extend_from_slice(&(shooter_pos.0).to_be_bytes());
            shot_msg.extend_from_slice(&(shooter_pos.1 + 1.2).to_be_bytes());
            shot_msg.extend_from_slice(&(shooter_pos.2).to_be_bytes());
            shot_msg.extend_from_slice(&(victim_pos.0).to_be_bytes());
            shot_msg.extend_from_slice(&(victim_pos.1 + 1.0).to_be_bytes());
            shot_msg.extend_from_slice(&(victim_pos.2).to_be_bytes());
            for p in players.values() {
                if p.is_bot {
                    continue;
                }
                let _ = socket.send_to(&shot_msg, p.socket_addr);
            }

            if let Some(damage) = damage_opt {
                other::apply_damage(
                    socket,
                    players,
                    attacker_addr,
                    victim_id,
                    damage,
                    &weapon,
                    false,
                    match_end_pending,
                    match_mode,
                    &mut self.bot_states,
                    maze,
                );
            }
        }
    }
    
}
