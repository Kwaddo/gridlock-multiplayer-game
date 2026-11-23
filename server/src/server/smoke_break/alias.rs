/*
    who's in which box stays a secret
    every boxed hider and decoy gets an alias id at round start, and everyone but the one inside
    sees that box (its MSG_STATE entry, its MSG_BOXES carrier, its gold glow) only under the alias
    an aliased entry is built from position and facing alone, so a decoy and a hider standing in
    the same spot facing the same way come out byte for byte the same
    the alias lives on the player (Player::smoke_alias), so every broadcast can use it without the round
*/

use std::f32::consts::TAU;

use super::SmokeState;
use crate::net::PeerId;
use crate::player::{Player, Team};
use crate::protocols::*;

/// alias ids start here, far past anything a real player can get (humans count up from 1, fighting bots
/// from 9000, decoys from 30000, a joiner takes the highest id + 1)
pub const SMOKE_ALIAS_BASE: u32 = 0x4000_0000;
/// aliases count up through the match and wrap inside this, so a box never reuses one from last round
const ALIAS_SPAN: u32 = 1 << 24;

/// what every aliased entry says about health, the same for everyone (a fresh player's)
const MASK_HEALTH: u16 = 125;

/// one fresh alias for this match
impl SmokeState {
    pub(super) fn next_alias(&mut self) -> u32 {
        let alias = SMOKE_ALIAS_BASE + self.aliases_used % ALIAS_SPAN;
        self.aliases_used = self.aliases_used.wrapping_add(1);
        alias
    }
}

/// the alias `p` (under `peer`) shows to `receiver`, None when they see the real thing
/// the one inside always sees their own box under their real id, their game counts on it
pub fn alias_for(peer: PeerId, p: &Player, receiver: Option<PeerId>) -> Option<u32> {
    p.smoke_alias.filter(|_| receiver != Some(peer))
}

/// where an aliased box stands and faces, as the wire shows it: on the floor (decoys never leave it)
/// and facing in 0..TAU (a client's yaw keeps counting past a full turn, a decoy's starts in 0..TAU)
pub fn masked_pose(position: (f32, f32, f32), yaw: f32) -> ((f32, f32, f32), f32) {
    ((position.0, 0.0, position.2), yaw.rem_euclid(TAU))
}

/// a box's `MSG_STATE` entry under its alias: position and facing only, everything else the same for all
pub fn masked_state(p: &Player, alias: u32) -> PlayerState {
    let (position, yaw) = masked_pose(p.position, p.yaw);
    PlayerState {
        id: alias,
        position,
        yaw,
        pitch: 0.0,
        state_flags: 0,
        event_flags: 0,
        health: MASK_HEALTH,
        alive: true,
        freeze_secs: 0,
    }
}

/// an alias's `MSG_PLAYER_INFO`: what a decoy has (no name, no gun, a fresh player's rest), under the alias
/// the same for every alias, whoever is inside (a decoy's real id never goes out at all)
pub fn masked_info(alias: u32) -> PlayerInfo {
    PlayerInfo {
        id: alias,
        name: String::new(),
        team: Team::Spectator.as_u8(),
        weapon: String::new(),
        kills: 0,
        gun_level: 1,
        max_health: MASK_HEALTH,
    }
}

#[cfg(test)]
#[path = "alias_tests.rs"]
mod tests;
