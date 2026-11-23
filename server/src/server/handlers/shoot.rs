//! A player's shot (`ClientMsg::Shoot`), relayed so the others in their world
//! see the muzzle flash and tracer, and hear it. Bots' shots go out the same
//! way, see `broadcast::shots_to_world`.

use std::collections::HashMap;
use std::time::Duration;
use platform::Instant;

use crate::net::{Outbox, PeerId};
use crate::player::Player;
use crate::protocols::Shot;
use crate::server::broadcast::{self, SHOT_START_HEIGHT};
use crate::weapons_stats::get_weapon_stats;

/// Farthest a relayed tracer runs from the shooter (m), a little past the
/// longest gun range (120 m). A farther end is pulled in along the shot.
pub const MAX_SHOT_DISTANCE: f32 = 150.0;

/// Share of the held weapon's shot interval that must pass between two
/// relayed shots. Below 1 because packets bunch up on the way and the server
/// learns of a weapon switch up to one position send (33 ms) late.
const FIRE_INTERVAL_SLACK: f32 = 0.75;

/// Shortest time between two relayed shots of `weapon`: its fire rate, with
/// some slack. This only keeps the relay's bandwidth sane (a minigun); hits
/// are reported and checked apart from it.
pub fn min_shot_interval(weapon: &str) -> Duration {
    let shots_per_sec = get_weapon_stats(weapon).shots_per_sec.max(0.1);
    Duration::from_secs_f32(FIRE_INTERVAL_SLACK / shots_per_sec)
}

/// Relay `src`'s shot to every other human in its world. The tracer starts
/// at the shooter's position as the server knows it (at gun height), never
/// where the client says, and ends at most `MAX_SHOT_DISTANCE` away. Shots
/// from the dead, from bots, from an aliased (Smoke Break) box and faster than
/// the weapon fires are dropped.
pub fn handle_shoot(
    socket: &Outbox,
    players: &mut HashMap<PeerId, Player>,
    src: PeerId,
    end: (f32, f32, f32),
    now: Instant,
) {
    if !(end.0.is_finite() && end.1.is_finite() && end.2.is_finite()) {
        return;
    }
    let Some(shooter) = players.get_mut(&src) else { return };
    // a box under a Smoke Break alias must never show its real id where it stands
    if shooter.is_bot || !shooter.is_alive || shooter.smoke_alias.is_some() {
        return;
    }
    let interval = min_shot_interval(&shooter.current_weapon);
    if shooter.last_shot_at.is_some_and(|t| now.saturating_duration_since(t) < interval) {
        return;
    }
    shooter.last_shot_at = Some(now);

    let (x, y, z) = shooter.position;
    let start = (x, y + SHOT_START_HEIGHT, z);
    let shot = Shot { shooter_id: shooter.id, victim_id: 0, start, end: cap_end(start, end) };
    let world = shooter.world;
    broadcast::shots_to_world(socket, players, world, vec![shot], Some(src));
}

/// `end`, pulled in along the shot to at most `MAX_SHOT_DISTANCE` from `start`.
fn cap_end(start: (f32, f32, f32), end: (f32, f32, f32)) -> (f32, f32, f32) {
    let d = (end.0 - start.0, end.1 - start.1, end.2 - start.2);
    let len = (d.0 * d.0 + d.1 * d.1 + d.2 * d.2).sqrt();
    if len <= MAX_SHOT_DISTANCE {
        return end;
    }
    let k = MAX_SHOT_DISTANCE / len;
    (start.0 + d.0 * k, start.1 + d.1 * k, start.2 + d.2 * k)
}

#[cfg(test)]
#[path = "shoot_tests.rs"]
mod tests;
