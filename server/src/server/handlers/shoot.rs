//! A player's shot (`ClientMsg::Shoot`), relayed so the others in their world
//! see the muzzle flash and tracer, and hear it. It waits in the server's
//! `broadcast::ShotQueue` and goes out with the bots' shots at the next bot
//! tick, in the same packet.

use std::collections::HashMap;
use std::time::Duration;
use platform::Instant;

use crate::bot::awareness::model::is_suppressed;
use crate::net::PeerId;
use crate::player::Player;
use crate::protocols::Shot;
use crate::server::broadcast::{ShotQueue, SHOT_START_HEIGHT};
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

/// Queue `src`'s shot for the humans in its world (the shooter's own client
/// drops it, it drew the shot when it fired). The tracer starts
/// at the shooter's position as the server knows it (at gun height), never
/// where the client says, and ends at most `MAX_SHOT_DISTANCE` away. Shots
/// from the dead, from bots, from an aliased (Smoke Break) box, from someone
/// in their spawn freeze and faster than the weapon fires are dropped. So
/// are a suppressed weapon's (judged by what the server knows they hold):
/// it has no tracer or flash, its shooter's own client draws none either.
/// Bots still hear those, barely (`BotManager::hear_shot`).
pub fn handle_shoot(
    shots: &mut ShotQueue,
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
    // frozen at a spawn the shot can't hurt anyone, so nobody sees it either;
    // a suppressed shot shows nothing to see
    if shooter.spawn_frozen(now) || is_suppressed(&shooter.current_weapon) {
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
    shots.push(shooter.world, shot);
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
