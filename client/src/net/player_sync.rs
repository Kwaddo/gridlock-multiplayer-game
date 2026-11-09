//! Decoding the players' half of a poll: `MSG_STATE` (what moves) and
//! `MSG_PLAYER_INFO` (what rarely changes), kept apart as the server sends
//! them. Nothing here allocates per player per packet: states stay the
//! protocol's plain `PlayerState`, and only the latest one of a poll is kept.

use std::collections::HashMap;

use crate::net::protocols::{PlayerInfo, PlayerState};
use crate::types::world::WorldId;

/// What the server last said about each player's name, team, weapon, kills,
/// gun level and max health, by id.
pub type PlayerInfoCache = HashMap<u32, PlayerInfo>;

/// The other players in the latest `MSG_STATE` of a poll.
pub struct StateSnapshot {
    pub world: WorldId,
    /// Everyone but us. One-shot event flags of any earlier state in the same
    /// poll are folded in, so a jump or landing is not lost to the next one.
    pub players: Vec<PlayerState>,
}

/// What our own entry in the poll's states said.
#[derive(Default)]
pub(crate) struct SelfState {
    pub seen: bool,
    pub health: Option<u16>,
    pub freeze_secs: Option<u8>,
    pub died: bool,
}

/// Take one `MSG_STATE` into `latest`, our own entry into `me`.
pub(crate) fn take_state(
    latest: &mut Option<StateSnapshot>,
    me: &mut SelfState,
    player_id: u32,
    world: u8,
    mut players: Vec<PlayerState>,
) {
    if let Some(at) = players.iter().position(|e| e.id == player_id) {
        let e = players.remove(at);
        me.seen = true;
        me.health = Some(e.health);
        // authoritative post-teleport freeze, so movement/weapon locks apply
        // even if the MSG_WORLD_CHANGE was lost while joining the match
        me.freeze_secs = Some(e.freeze_secs);
        // dead in the state even if MSG_DEATH was missed
        me.died |= !e.alive;
    }
    if let Some(prev) = latest.take() {
        for e in players.iter_mut() {
            if let Some(p) = prev.players.iter().find(|p| p.id == e.id) {
                e.event_flags |= p.event_flags;
            }
        }
    }
    let world = WorldId::from_u8(world).unwrap_or(WorldId::Lobby);
    *latest = Some(StateSnapshot { world, players });
}

/// Take one `MSG_PLAYER_INFO` into the cache. Returns (anything changed,
/// our own entry changed).
pub(crate) fn take_info(cache: &mut PlayerInfoCache, player_id: u32, entries: Vec<PlayerInfo>) -> (bool, bool) {
    let (mut any, mut mine) = (false, false);
    for e in entries {
        if cache.get(&e.id) == Some(&e) {
            continue;
        }
        any = true;
        mine |= e.id == player_id;
        cache.insert(e.id, e);
    }
    (any, mine)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::protocols::{EVT_JUMP_START, EVT_LANDED};

    fn st(id: u32, x: f32, event_flags: u8, alive: bool) -> PlayerState {
        PlayerState {
            id,
            position: (x, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            state_flags: 0,
            event_flags,
            health: 90,
            alive,
            freeze_secs: 3,
        }
    }

    #[test]
    fn the_latest_state_wins_but_earlier_events_survive() {
        let (mut latest, mut me) = (None, SelfState::default());
        // a tick broadcast with events, then a mid-tick one without
        take_state(&mut latest, &mut me, 1, 1, vec![st(1, 0.0, 0, false), st(2, 1.0, EVT_JUMP_START, true)]);
        take_state(&mut latest, &mut me, 1, 1, vec![st(2, 2.0, EVT_LANDED, true), st(3, 5.0, 0, true)]);
        let snap = latest.unwrap();
        assert_eq!(snap.world, WorldId::Maze);
        let got: Vec<(u32, f32)> = snap.players.iter().map(|p| (p.id, p.position.0)).collect();
        assert_eq!(got, [(2, 2.0), (3, 5.0)]);
        assert_eq!(snap.players[0].event_flags, EVT_JUMP_START | EVT_LANDED);
        // our own entry is kept apart; a death in any state of the poll counts
        assert!(me.seen && me.died);
        assert_eq!((me.health, me.freeze_secs), (Some(90), Some(3)));
    }

    #[test]
    fn info_reports_only_real_changes() {
        let mut cache = PlayerInfoCache::new();
        let alice = PlayerInfo { id: 1, name: "alice".into(), team: 1, ..Default::default() };
        let bob = PlayerInfo { id: 2, name: "bob".into(), team: 2, ..Default::default() };
        assert_eq!(take_info(&mut cache, 2, vec![alice.clone(), bob.clone()]), (true, true));
        // the periodic resend of the same thing changes nothing
        assert_eq!(take_info(&mut cache, 2, vec![alice.clone(), bob.clone()]), (false, false));
        let armed = PlayerInfo { weapon: "AK-47".into(), ..alice };
        assert_eq!(take_info(&mut cache, 2, vec![armed, bob]), (true, false));
        assert_eq!(cache[&1].weapon, "AK-47");
    }
}
