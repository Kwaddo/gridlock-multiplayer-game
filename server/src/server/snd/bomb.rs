//! The bomb before it is planted: who gets it at round start, dropping it
//! when the carrier dies or leaves, and picking it up off the floor.
//!
//! Only human attackers carry the bomb. Bots have no bomb logic yet, so an
//! attacking team without humans plays the round without a bomb and can only
//! win by eliminating the defenders.

use std::collections::HashMap;

use crate::net::PeerId;
use crate::player::{Player, Team, WorldId};

use super::{SndPhase, SndState, SND_BOMB_PICKUP_RANGE};

impl SndState {
    /// Whether `p` may carry the bomb: a living human on the attacking team, in the maze.
    pub(super) fn can_carry_bomb(&self, p: &Player) -> bool {
        p.is_alive && !p.is_bot && p.world == WorldId::Maze && p.team == self.attacking_team
    }

    /// The bomb carrier for a new round: the human attacker with the lowest id
    /// (stable, unlike the map's order), or None when no human attacks.
    pub(super) fn pick_bomb_carrier<'p>(
        &self,
        players: &'p HashMap<PeerId, Player>,
    ) -> Option<&'p Player> {
        players.values().filter(|p| self.can_carry_bomb(p)).min_by_key(|p| p.id)
    }

    /// Hand the bomb to `carrier` at round start (None: no bomb this round).
    pub(super) fn give_bomb(&mut self, carrier: Option<(u32, (f32, f32, f32))>) {
        self.bomb_dropped_pos = None;
        self.bomb_carrier = carrier.map(|(id, _)| id);
        self.carrier_last_pos = carrier.map(|(_, pos)| pos);
    }

    /// Put the bomb on the floor at `pos` and clear the carrier.
    pub fn drop_bomb(&mut self, pos: (f32, f32, f32)) {
        self.bomb_carrier = None;
        self.carrier_last_pos = None;
        // the floor is level with the bomb site
        self.bomb_dropped_pos = Some((pos.0, self.bomb_site.1, pos.2));
        // a drop interrupts a plant in progress
        self.planter = None;
        self.plant_progress = 0.0;
    }

    /// Once per tick: drop the bomb if its carrier died, left the maze or
    /// disconnected; otherwise remember where they are, so a carrier who
    /// vanishes from `players` still drops it where they last stood.
    pub(super) fn track_bomb_carrier(&mut self, players: &HashMap<PeerId, Player>) {
        let Some(carrier_id) = self.bomb_carrier else {
            return;
        };
        match players.values().find(|p| p.id == carrier_id) {
            Some(p) if self.can_carry_bomb(p) => self.carrier_last_pos = Some(p.position),
            // dead (or moved out of the maze): drop it where they are now
            Some(p) => self.drop_bomb(p.position),
            // gone: drop it where they were last seen, or at their spawn
            None => {
                let spawn = match self.attacking_team {
                    Team::Blue => self.blue_spawn,
                    _ => self.red_spawn,
                };
                self.drop_bomb(self.carrier_last_pos.unwrap_or(spawn));
            }
        }
    }

    /// Once per tick: the nearest human attacker within pickup range of the
    /// dropped bomb picks it up. Defenders walk over it.
    pub(super) fn auto_pickup_bomb(&mut self, players: &HashMap<PeerId, Player>) {
        if !matches!(self.phase, SndPhase::FreezeTime | SndPhase::Active) {
            return;
        }
        let Some(bomb_pos) = self.bomb_dropped_pos else {
            return;
        };
        let nearest = players
            .values()
            .filter(|p| self.can_carry_bomb(p))
            .map(|p| (Self::distance(p.position, bomb_pos), p))
            .filter(|(d, _)| *d <= SND_BOMB_PICKUP_RANGE)
            .min_by(|(da, a), (db, b)| da.total_cmp(db).then(a.id.cmp(&b.id)));
        if let Some((_, p)) = nearest {
            self.take_bomb(p);
        }
    }

    fn take_bomb(&mut self, p: &Player) {
        self.bomb_carrier = Some(p.id);
        self.carrier_last_pos = Some(p.position);
        self.bomb_dropped_pos = None;
    }

    /// Handle bomb pickup request
    pub fn try_pickup_bomb(
        &mut self,
        player_addr: PeerId,
        players: &HashMap<PeerId, Player>,
    ) -> bool {
        let player = match players.get(&player_addr) {
            Some(p) => p,
            None => return false,
        };

        // Must be a living human attacker, and the bomb must be on the ground
        if !self.can_carry_bomb(player)
            || !matches!(self.phase, SndPhase::FreezeTime | SndPhase::Active)
        {
            return false;
        }

        if let Some(bomb_pos) = self.bomb_dropped_pos {
            if Self::distance(player.position, bomb_pos) <= SND_BOMB_PICKUP_RANGE {
                self.take_bomb(player);
                return true;
            }
        }

        false
    }
}
