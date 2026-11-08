//! `MSG_PLAYER_INFO`: what rarely changes about players (name, team, weapon,
//! kills, gun level, max health), kept out of the 30 Hz `MSG_STATE`.
//!
//! Rather than hook every place that changes one of those fields, each tick
//! compares the players against what was last sent and sends the difference.
//! A joiner gets everyone at once, and everyone gets everyone every few
//! seconds, since any packet may be lost.

use std::collections::{HashMap, HashSet};
use std::time::Duration;
use platform::Instant;

use super::Server;
use crate::net::PeerId;
use crate::player::Player;
use crate::protocols::*;

/// Resend everyone's info this often, for whoever lost a packet.
const FULL_EVERY: Duration = Duration::from_secs(2);
/// Entries per packet: keeps one well under the ~1100 bytes WebRTC sends unreliably.
const PER_PACKET: usize = 20;

#[derive(Default)]
pub(super) struct InfoSync {
    /// Each player's info as last sent, by player id.
    sent: HashMap<u32, PlayerInfo>,
    /// Humans that have had everyone's info.
    primed: HashSet<PeerId>,
    last_full: Option<Instant>,
}

pub(crate) fn info_of(p: &Player) -> PlayerInfo {
    PlayerInfo {
        id: p.id,
        name: p.username.clone(),
        team: p.team.as_u8(),
        weapon: p.current_weapon.clone(),
        kills: p.kills,
        gun_level: p.gun_level.min(u8::MAX as u32) as u8,
        max_health: p.max_health.min(u16::MAX as u32) as u16,
    }
}

/// `info == info_of(p)`, without building it.
fn same(info: &PlayerInfo, p: &Player) -> bool {
    info.name == p.username
        && info.team == p.team.as_u8()
        && info.weapon == p.current_weapon
        && info.kills == p.kills
        && info.gun_level == p.gun_level.min(u8::MAX as u32) as u8
        && info.max_health == p.max_health.min(u16::MAX as u32) as u16
}

fn packets<'a>(entries: impl Iterator<Item = &'a PlayerInfo>) -> Vec<Vec<u8>> {
    let entries: Vec<PlayerInfo> = entries.cloned().collect();
    entries.chunks(PER_PACKET).map(|c| ServerMsg::PlayerInfo(c.to_vec()).encode()).collect()
}

impl Server {
    /// Send whatever changed in anyone's info since it last went out, and
    /// everyone's to whoever has not had it yet (or to all, when due).
    pub(super) fn sync_player_info(&mut self) {
        let sync = &mut self.info_sync;
        let players = &self.players;

        if sync.sent.len() > players.len() {
            let ids: HashSet<u32> = players.values().map(|p| p.id).collect();
            sync.sent.retain(|id, _| ids.contains(id));
        }
        sync.primed.retain(|addr| players.get(addr).is_some_and(|p| !p.is_bot));

        let mut changed = Vec::new();
        for p in players.values() {
            if !sync.sent.get(&p.id).is_some_and(|s| same(s, p)) {
                let info = info_of(p);
                sync.sent.insert(p.id, info.clone());
                changed.push(info);
            }
        }

        let humans = players.iter().filter(|(_, p)| !p.is_bot).map(|(addr, _)| *addr);
        if sync.last_full.map_or(true, |t| t.elapsed() >= FULL_EVERY) {
            sync.last_full = Some(Instant::now());
            let all = packets(sync.sent.values());
            for addr in humans {
                for msg in &all {
                    let _ = self.outbox.send_to(msg, addr);
                }
                sync.primed.insert(addr);
            }
            return;
        }

        let newcomers: Vec<PeerId> = humans.filter(|addr| !sync.primed.contains(addr)).collect();
        if !changed.is_empty() {
            let msgs = packets(changed.iter());
            for addr in sync.primed.iter() {
                for msg in &msgs {
                    let _ = self.outbox.send_to(msg, *addr);
                }
            }
        }
        if !newcomers.is_empty() {
            let all = packets(sync.sent.values());
            for addr in newcomers {
                for msg in &all {
                    let _ = self.outbox.send_to(msg, addr);
                }
                sync.primed.insert(addr);
            }
        }
    }
}
