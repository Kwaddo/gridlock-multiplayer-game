use std::{ collections::HashMap, net::{SocketAddr, UdpSocket}, time::{ Duration, Instant } };
use crate::{ bot::ai::{ self, BotState }, physics::PhysicsConfig };
use crate::player::Player;
use crate::bot::shooting;
use crate::protocols::MSG_BOT_SHOT;
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
        // check membership in bot_states or bot_addrs
        self.bot_states.contains_key(addr) || self.bot_addrs.contains(addr)
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
        match_mode: u8
    ) {
        if self.last_tick.elapsed() < self.tick_interval {
            return;
        } else {
            self.last_tick = Instant::now();
        }
        let dt = self.tick_interval.as_secs_f32();

        for addr in &self.bot_addrs {
            if let Some(bot) = players.get_mut(addr) {
                if let Some(state) = self.bot_states.get_mut(addr) {
                    ai::tick_bot(bot, state, dt, &PhysicsConfig::default());
                }
            }
        }

        // Run shooting logic after movement updates; collect first to avoid borrow conflicts.
        let mut pending_hits: Vec<(SocketAddr, u32, u32, String)> = Vec::new();
        for addr in &self.bot_addrs {
            if let Some(state) = self.bot_states.get_mut(addr) {
                if !state.can_shoot {
                    continue;
                }
                if let Some((victim_id, damage)) = shooting::try_shoot(*addr, players, state, dt) {
                    let weapon = players
                        .get(addr)
                        .map(|p| p.current_weapon.clone())
                        .unwrap_or_else(|| "Unknown".to_string());
                    pending_hits.push((*addr, victim_id, damage, weapon));
                }
            }
        }

        for (attacker_addr, victim_id, damage, weapon) in pending_hits {
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
            );
        }
    }
    
}
