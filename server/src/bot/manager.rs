use std::{ collections::HashMap, time::Duration };
use platform::Instant;
use crate::net::{ BOT_PEER_BASE, Outbox, PeerId };
use std::f32::consts::PI;
use crate::{ bot::ai::{ self, BotState, BotBehavior }, physics::PhysicsConfig };
use crate::player::{ Player, Team, WorldId };
use crate::bot::shooting;
use crate::bot::behavior;
use crate::bot::personality::{ self, BotPersonalityId };
use crate::protocols::Shot;
use crate::server::broadcast;
use crate::server::handlers::other;
/// Seconds between a match bot's decisions (plus up to 0.1 s jitter).
const DECISION_INTERVAL: f32 = 0.2;
/// Bot shots per `MSG_SHOTS` packet: 20 bytes each, so one stays under 1100 bytes.
const MAX_SHOTS_PER_PACKET: usize = 50;
/// Longest step (seconds) one bot tick simulates, however long it has been.
const MAX_BOT_DT: f32 = 0.1;

pub struct BotManager {
    bot_addrs: Vec<PeerId>,
    bot_states: HashMap<PeerId, BotState>,
    last_tick: Instant,
    /// Paces bot ticks to an even 30 a second.
    cadence: crate::cadence::Cadence,
    next_bot_idx: u16,
    range_challenge_deadline: Option<Instant>,
}

impl BotManager {
    pub fn new() -> Self {
        Self {
            bot_addrs: Vec::new(),
            bot_states: HashMap::new(),
            last_tick: Instant::now(),
            cadence: crate::cadence::Cadence::new(crate::cadence::HZ30),
            next_bot_idx: 0,
            range_challenge_deadline: None,
        }
    }
    pub fn virtual_addr(index: u16) -> PeerId {
        // generate unique virtual PeerId for each bot based on index
        PeerId(BOT_PEER_BASE + index as u32)
    }

    pub fn bot_states_mut(&mut self) -> &mut HashMap<PeerId, BotState> {
        &mut self.bot_states
    }
    pub fn spawn_bot(
        &mut self,
        players: &mut HashMap<PeerId, Player>,
        mut player: Player,
        bot_state: BotState
    ) -> PeerId {
        let addr = Self::virtual_addr(self.next_bot_idx);
        self.next_bot_idx += 1;

        player.is_bot = true;
        player.is_silent = false;
        players.insert(addr, player);

        self.bot_states.insert(addr, bot_state);
        self.bot_addrs.push(addr);

        addr
    }

    pub fn despawn_all(&mut self, players: &mut HashMap<PeerId, Player>) {
        for addr in &self.bot_addrs {
            players.remove(addr);
        }
        self.bot_states.clear();
        self.bot_addrs.clear();
        self.range_challenge_deadline = None;
    }
    /// Despawn one bot (say, to make room for a player joining late).
    /// Returns its player, or None if `addr` isn't one of ours.
    pub fn despawn_bot(&mut self, players: &mut HashMap<PeerId, Player>, addr: PeerId) -> Option<Player> {
        if !self.bot_addrs.contains(&addr) {
            return None;
        }
        self.bot_addrs.retain(|a| *a != addr);
        self.bot_states.remove(&addr);
        players.remove(&addr)
    }
    /// Despawn only shooting-range bots (those without a personality_id).
    /// Game bots with personalities are left intact.
    pub fn despawn_range_bots(&mut self, players: &mut HashMap<PeerId, Player>) {
        let to_remove: Vec<PeerId> = self.bot_addrs.iter()
            .filter(|addr| self.bot_states.get(addr).map_or(false, |s| s.personality_id.is_none()))
            .copied()
            .collect();
        for addr in &to_remove {
            players.remove(addr);
            self.bot_states.remove(addr);
        }
        self.bot_addrs.retain(|a| !to_remove.contains(a));
        self.range_challenge_deadline = None;
    }
    /// Spawn a bot with a full personality into a match.
    /// Returns the virtual PeerId assigned to the bot.
    pub fn spawn_personality_bot(
        &mut self,
        players: &mut HashMap<PeerId, Player>,
        personality_id: BotPersonalityId,
        team: Team,
        weapon_override: &str,
        spawn_pos: (f32, f32, f32)
    ) -> PeerId {
        let (pers, diff) = personality::get_personality(personality_id);
        let runtime = personality::compute_runtime(&pers, diff);
        let bot_id = 9000u32 + (self.next_bot_idx as u32);

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
            last_activity: Instant::now(),
            death_time: None,
            world: WorldId::Maze,
            post_teleport_freeze_until: Some(
                Instant::now() +
                    Duration::from_secs(crate::server::game::MATCH_POST_TELEPORT_FREEZE_SECS)
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
            in_box: false,
            smoke_alias: None,
            last_shot_at: None,
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
                aim_threshold: (runtime.aim_offset_mult * 0.5).clamp(0.03, 0.20), // around 1.7 for nightmare to 11 degrees for recruit
            },
            target_id: None,
            reaction_timer: 0.0,
            personality: pers,
            difficulty: diff,
            archetype: pers.archetype,
            personality_id: Some(personality_id),
            last_known_enemy_pos: None,
            behavior_decision_timer: 0.0,
            stuck_timer: 0.0,
            idle_look_timer: 0.0,
            current_yaw: 0.0,
            current_pitch: 0.0,
            // a stance from the first decision on, not always a push
            tactics: ai::BotTactics { stance_roll: rand::random(), ..Default::default() },
        };

        let addr = self.spawn_bot(players, bot_player, bot_state);
        log!("Spawned bot '{}' ({:?}) on {:?} with {}", pers.name, diff, team, weapon_override);
        addr
    }

    pub fn set_range_challenge_deadline(&mut self, deadline: Option<Instant>) {
        self.range_challenge_deadline = deadline;
    }
    pub fn handle_range_challenge_timeout(
        &mut self,
        players: &mut HashMap<PeerId, Player>
    ) -> bool {
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
        socket: &Outbox,
        players: &mut HashMap<PeerId, Player>,
        match_end_pending: &mut Option<(Instant, u32, String)>,
        match_mode: u8,
        maze: Option<&crate::maze::MazeData>,
        team_score: &mut crate::server::game::TeamScore,
        // S&D freeze time: every bot in the maze stands still
        round_frozen: bool,
    ) {
        // one clock read for the whole bot tick (on the web host each is a JS call)
        let now = Instant::now();
        if !self.cadence.due(now) {
            return;
        }
        let elapsed = now.saturating_duration_since(self.last_tick);
        self.last_tick = now;
        // Step by the real time since the last bot tick: the host may tick
        // less often than the gate allows (an idle native host waits up to
        // 100 ms for a packet). Capped so a stall doesn't make bots jump.
        let dt = elapsed.as_secs_f32().min(MAX_BOT_DT);

        // ── Brain: run decide_behavior for maze-world bots ──
        // A few times a second rather than every tick: decisions with random
        // rolls made 30 times a second flip-flop (a camper "decides" to push
        // within moments), and nobody re-plans that fast.
        if let Some(m) = maze {
            for addr in &self.bot_addrs {
                let bot = match players.get(addr) {
                    Some(p) if p.is_alive && p.world == WorldId::Maze => p,
                    _ => {
                        continue;
                    }
                };
                // frozen like the humans at match start and in S&D freeze time
                if is_frozen(bot, now, round_frozen) {
                    continue;
                }
                let Some(state) = self.bot_states.get_mut(addr) else {
                    continue;
                };
                // only run for bots with a personality_id (maze bots, not shooting range bots)
                if state.personality_id.is_none() {
                    continue;
                }
                state.behavior_decision_timer -= dt;
                if state.behavior_decision_timer > 0.0 {
                    continue;
                }
                // a little jitter so a team of bots doesn't think in lockstep
                state.behavior_decision_timer = DECISION_INTERVAL + rand::random::<f32>() * 0.1;
                if let Some(new_behavior) = behavior::decide_behavior(bot, state, players, m, &m.analysis) {
                    state.behavior = new_behavior;
                }
            }
        }

        for addr in &self.bot_addrs {
            let Some(p) = players.get(addr).filter(|p| p.is_alive) else {
                continue;
            };
            let is_frozen = is_frozen(p, now, round_frozen);
            let Some(state) = self.bot_states.get_mut(addr) else {
                continue;
            };
            let find_alive = |id: u32| players.values().find(|p| p.id == id && p.is_alive && !p.in_box).map(|p| p.position);
            // the enemy in sight, if any
            let target_pos = state.target_id.and_then(find_alive);
            // else whoever shot us: where they are now, or where they shot from
            let alert_pos = state
                .tactics
                .alert
                .map(|(id, pos, _)| find_alive(id).unwrap_or(pos));
            if let Some(bot) = players.get_mut(addr) {
                if is_frozen {
                    state.movement.velocity.x = 0.0;
                    state.movement.velocity.z = 0.0;
                    continue;
                }
                // fighting moves (strafing, keeping range) only for match bots;
                // shooting-range bots keep to their set routine
                let engage = target_pos.filter(|_| state.personality_id.is_some()).map(|pos| {
                    let dx = pos.0 - bot.position.0;
                    let dz = pos.2 - bot.position.2;
                    ai::Engagement { target_pos: pos, distance: (dx * dx + dz * dz).sqrt() }
                });
                let maze_cells = maze.map(|m| (m.cells.as_slice(), m.width, m.height, m.prop_cells()));
                ai::tick_bot(bot, state, dt, &PhysicsConfig::default(), maze_cells, engage);

                if let Some(target_pos) = target_pos {
                    // remember last known enemy position for investigation behavior
                    state.last_known_enemy_pos = Some(target_pos);
                }
                aim_at(bot, state, target_pos.or(alert_pos), dt);

                // the alert fades, and ends once we see someone
                if let Some((_, _, left)) = &mut state.tactics.alert {
                    *left -= dt;
                    if *left <= 0.0 || target_pos.is_some() {
                        state.tactics.alert = None;
                    }
                }
            }
        }

        // Run shooting logic after movement updates; collect first to avoid borrow conflicts.
        // The clock is read again here: movement and pathfinding for every bot
        // ran since `now`, and a freeze ending in between must not hold a shot.
        let now = Instant::now();
        let mut pending_shots: Vec<(PeerId, u32, Option<u32>, String)> = Vec::new();
        for addr in &self.bot_addrs {
            if players.get(addr).filter(|p| p.is_alive).map_or(true, |p| is_frozen(p, now, round_frozen)) {
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

        // every tracer of this tick goes out ahead of the damage it deals
        send_bot_shot_iter(socket, players, pending_shots.iter().map(|(a, v, _, _)| (*a, *v)));

        for (attacker_addr, victim_id, damage_opt, weapon) in pending_shots {
            if let Some(damage) = damage_opt {
                other::apply_damage(
                    socket,
                    players,
                    attacker_addr,
                    victim_id,
                    damage,
                    &weapon,
                    false,
                    false, // bots dont roll hit zones, so never a headshot
                    match_end_pending,
                    match_mode,
                    &mut self.bot_states,
                    maze,
                    team_score
                );
            }
        }
    }
}

/// A bot that may not move, turn or shoot: in the maze during its
/// post-teleport freeze, or during S&D freeze time (`round_frozen`).
fn is_frozen(p: &Player, now: Instant, round_frozen: bool) -> bool {
    p.world == WorldId::Maze && (round_frozen || p.post_teleport_freeze_until.is_some_and(|t| t > now))
}

/// Turn the bot's aim toward `look_pos` at its turn rate. Whenever it has
/// something to aim at, the yaw/pitch everyone sees is the aim it shoots
/// with, not the way it walks.
pub(crate) fn aim_at(bot: &mut Player, state: &mut BotState, look_pos: Option<(f32, f32, f32)>, dt: f32) {
    let Some(look_pos) = look_pos else {
        // nothing to aim at: the aim follows where the bot looks, so
        // spotting someone turns from here, not from a stale angle
        state.current_yaw = bot.yaw;
        state.current_pitch = bot.pitch;
        return;
    };
    let dx = look_pos.0 - bot.position.0;
    let dy = look_pos.1 - bot.position.1;
    let dz = look_pos.2 - bot.position.2;
    let horiz_dist = (dx * dx + dz * dz).sqrt(); // for FOV and yaw calculations
    if horiz_dist > 0.01 {
        let target_yaw = dz.atan2(dx); // atan2 is a method that returns the angle between the positive x-axis and the point (dx, dz)
        let target_pitch = -(dy / horiz_dist.max(0.001)).atan(); // negative because in most FPS coordinate systems, looking up is negative pitch

        // Smoothly rotate towards target based on personality turn_rate.
        // turn_rate_mult > 1 means slower (easier bots), so divide.
        let turn_rate = state.personality.base_turn_rate
            / personality::difficulty_mods(state.difficulty).turn_rate_mult.max(0.01);

        // lerp yaw
        let yaw_diff = {
            let d = target_yaw - state.current_yaw;
            ((d + PI).rem_euclid(2.0 * PI)) - PI // shortest angle difference
        };

        let max_turn = turn_rate * dt;
        state.current_yaw += yaw_diff.clamp(-max_turn, max_turn);

        let pitch_diff = target_pitch - state.current_pitch;
        state.current_pitch += pitch_diff.clamp(-max_turn * 0.6, max_turn * 0.6);
    }
    // standing on top of them too: tick_bot faced the way it walked
    bot.yaw = state.current_yaw;
    bot.pitch = state.current_pitch;
}

/// [`send_bot_shot_iter`] over a slice.
#[cfg(test)]
pub(crate) fn send_bot_shots(socket: &Outbox, players: &HashMap<PeerId, Player>, shots: &[(PeerId, u32)]) {
    send_bot_shot_iter(socket, players, shots.iter().copied());
}

/// Tell the humans in each shooter's world about `shots` (shooter, victim id)
/// at once: one `MSG_SHOTS` per player per bot tick, not one per shot.
/// Damage moves nobody, so the tracer ends are where a shot-by-shot send
/// found them too. Each packet's shots are built straight into the message,
/// with no copies. A shot whose shooter or victim is gone is not sent (it
/// would draw a tracer from or to the world origin), and humans in another
/// world never get it (they would hear it and see its tracer at that world's
/// coordinates).
fn send_bot_shot_iter(
    socket: &Outbox,
    players: &HashMap<PeerId, Player>,
    shots: impl Iterator<Item = (PeerId, u32)>,
) {
    let position_of_id = |id: u32| players.values().find(|p| p.id == id).map(|p| p.position);
    let send = |world: WorldId, batch: Vec<Shot>| {
        broadcast::shots_to_world(socket, players, world, batch, None);
    };
    // bots are almost always all in one world, so this holds one batch
    let mut batches: Vec<(WorldId, Vec<Shot>)> = Vec::new();
    for (attacker_addr, victim_id) in shots {
        let Some(shooter) = players.get(&attacker_addr) else { continue };
        let Some(to) = position_of_id(victim_id) else { continue };
        let from = shooter.position;
        let shot = Shot {
            shooter_id: shooter.id,
            victim_id,
            start: (from.0, from.1 + broadcast::SHOT_START_HEIGHT, from.2),
            end: (to.0, to.1 + 1.0, to.2),
        };
        let slot = match batches.iter().position(|(w, _)| *w == shooter.world) {
            Some(i) => i,
            None => {
                batches.push((shooter.world, Vec::new()));
                batches.len() - 1
            }
        };
        let (world, batch) = &mut batches[slot];
        batch.push(shot);
        if batch.len() == MAX_SHOTS_PER_PACKET {
            send(*world, std::mem::take(batch));
        }
    }
    for (world, batch) in batches {
        if !batch.is_empty() {
            send(world, batch);
        }
    }
}
