use std::{collections::HashMap, hash::Hash, net::SocketAddr, time::{Duration, Instant}};
use crate::{bot::ai::{self, BotState}, physics::PhysicsConfig};
use crate::player::Player;
pub struct BotManager{
    bot_addrs: Vec<SocketAddr>,
    bot_states: HashMap<SocketAddr, BotState>,
    last_tick: Instant,
    tick_interval: Duration,
    next_bot_idx: u16,
}

impl BotManager {
    pub fn new() -> Self {
        Self {
            bot_addrs: Vec::new(),
            bot_states: HashMap::new(),
            last_tick: Instant::now(),
            tick_interval: Duration::from_millis(33), // 33 ticks per second
            next_bot_idx: 0,
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
        bot_state: BotState,
    ) -> SocketAddr {
        let addr = Self::virtual_addr(self.next_bot_idx);
        self.next_bot_idx += 1;

        player.is_bot = true;
        players.insert(addr, player);
        
        self.bot_states.insert(addr, bot_state);
        self.bot_addrs.push(addr);

        addr
    }

    pub fn despawn_bot(
        &mut self,
        players: &mut HashMap<SocketAddr, Player>,
        addr: SocketAddr,
    ) {
        players.remove(&addr);
        self.bot_states.remove(&addr);
        self.bot_addrs.retain(|&a| a != addr);
    }
    pub fn despawn_all(
        &mut self,
        players: &mut HashMap<SocketAddr, Player>,
    ) {
        for addr in &self.bot_addrs {
            players.remove(addr);
        }
        self.bot_states.clear();
        self.bot_addrs.clear();
    }
    pub fn tick(
        &mut self,
        players: &mut HashMap<SocketAddr, Player>,
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
    }
}