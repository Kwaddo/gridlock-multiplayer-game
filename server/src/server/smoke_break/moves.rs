/*
    a hider's game says where its box is, the server doesn't just take its word for it
    a box goes 1.8 m/s (client cardboard.rs BOX_SPEED), so every move gets a distance budget that fills
    at that speed with room to spare, a step past it gets cut short, and a step off the floor stays put
    if the game ends up somewhere the server won't follow, it gets told where its box really is
*/

use std::time::Duration;

use platform::Instant;

use super::spots::area_of;
use super::SmokeState;
use crate::net::PeerId;
use crate::protocols::*;
use crate::server::Server;

/// the client's box speed
const BOX_SPEED: f32 = 1.8;
/// how much faster than that the budget fills (lag, frame hitches)
const SPEED_SLACK: f32 = 1.5;
/// most the budget holds: a game that stalled for about a second still catches up in one step,
/// a longer pause buys no more than that
const MAX_BUDGET: f32 = 2.5;
/// a step this long is no walk at all (a teleport, or a packet from before the round moved them): the box stays put
const JUMP_PAST: f32 = 5.0;
/// a step is checked for floor this often along its line, so it can't hop a wall between two rooms
const PATH_STEP: f32 = 0.25;
/// the game is this far from where the server keeps the box: it gets told
const CORRECT_PAST: f32 = 1.0;
/// at most one correction this often, the packets already on their way are from before it
const CORRECT_EVERY: Duration = Duration::from_millis(500);

pub(super) struct MoveBudget {
    metres: f32,
    at: Instant,
    last_fix: Option<Instant>,
}

impl MoveBudget {
    fn new(now: Instant) -> Self {
        MoveBudget { metres: MAX_BUDGET, at: now, last_fix: None }
    }
}

impl SmokeState {
    /// where a boxed hider moving `from` -> `to` really ends up, and whether their game needs telling
    pub(super) fn trusted_move(
        &mut self,
        peer: PeerId,
        from: (f32, f32, f32),
        to: (f32, f32, f32),
        now: Instant,
    ) -> ((f32, f32, f32), bool) {
        let budget = self.moves.entry(peer).or_insert_with(|| MoveBudget::new(now));
        let dt = now.saturating_duration_since(budget.at).as_secs_f32();
        budget.at = now;
        budget.metres = (budget.metres + dt * BOX_SPEED * SPEED_SLACK).min(MAX_BUDGET);

        let (dx, dz) = (to.0 - from.0, to.2 - from.2);
        let step = (dx * dx + dz * dz).sqrt();
        let k = if step > budget.metres { budget.metres / step } else { 1.0 };
        let mut at = (from.0 + dx * k, to.1, from.2 + dz * k);
        // off the map, into a wall or no walk at all: the box stays where it was
        if step > JUMP_PAST || !on_floor_along(from, at) {
            at = from;
        }
        let (ox, oz) = (to.0 - at.0, to.2 - at.2);
        budget.metres -= ((at.0 - from.0).powi(2) + (at.2 - from.2).powi(2)).sqrt();

        let off = (ox * ox + oz * oz).sqrt() > CORRECT_PAST;
        let due = budget.last_fix.map_or(true, |t| now.saturating_duration_since(t) >= CORRECT_EVERY);
        let fix = off && due;
        if fix {
            budget.last_fix = Some(now);
        }
        (at, fix)
    }
}

/// every point along from -> to (sampled every PATH_STEP) is on the arena floor
fn on_floor_along(from: (f32, f32, f32), to: (f32, f32, f32)) -> bool {
    let (dx, dz) = (to.0 - from.0, to.2 - from.2);
    let n = ((dx * dx + dz * dz).sqrt() / PATH_STEP).ceil().max(1.0) as usize;
    (1..=n).all(|i| {
        let t = i as f32 / n as f32;
        area_of(from.0 + dx * t, from.2 + dz * t).is_some()
    })
}

impl Server {
    /// a position update from a boxed hider during smoke break: cut down to what the box could have done,
    /// their game told where it really is if it's too far off. None for anyone else, their update stands as it is
    pub(in crate::server) fn smoke_trusted_position(
        &mut self,
        peer: PeerId,
        to: (f32, f32, f32),
    ) -> Option<(f32, f32, f32)> {
        if !self.smoke_on() || !self.boxes.is_carrying(peer) {
            return None;
        }
        let from = self.players.get(&peer).filter(|p| !p.is_bot).map(|p| p.position)?;
        let smoke = self.smoke_state.as_mut()?;
        let (at, fix) = smoke.trusted_move(peer, from, to, Instant::now());
        if fix {
            let msg = ServerMsg::Respawn { position: at, kind: None }.encode();
            let _ = self.outbox.send_to(&msg, peer);
        }
        Some(at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // the middle of the middle room
    const MID: (f32, f32, f32) = (30.6, 0.0, 9.0);

    #[test]
    fn walking_at_box_speed_goes_through_untouched() {
        let mut s = SmokeState::new();
        let (peer, mut now) = (PeerId(1), Instant::now());
        let mut at = MID;
        for _ in 0..30 {
            now = now + Duration::from_millis(33);
            let to = (at.0 + BOX_SPEED * 0.033, 0.0, at.2);
            let (got, fix) = s.trusted_move(peer, at, to, now);
            assert_eq!((got, fix), (to, false));
            at = got;
        }
    }

    #[test]
    fn a_teleport_gets_cut_short_and_the_game_told() {
        let mut s = SmokeState::new();
        let (peer, now) = (PeerId(1), Instant::now());
        let to = (MID.0 + 4.0, 0.0, MID.2);
        let (got, fix) = s.trusted_move(peer, MID, to, now + Duration::from_millis(33));
        assert!((got.0 - MID.0 - MAX_BUDGET).abs() < 1e-3, "only as far as the budget");
        assert!(fix);
        // the packets after it are still far off, but the game was only just told
        let (got, again) = s.trusted_move(peer, got, to, now + Duration::from_millis(66));
        assert!(!again);
        // further than any walk: not a step of it
        let far = (MID.0 + 10.0, 0.0, MID.2);
        let (stay, _) = s.trusted_move(peer, got, far, now + Duration::from_secs(5));
        assert_eq!(stay, got);
    }

    #[test]
    fn a_long_pause_buys_no_teleport() {
        let mut s = SmokeState::new();
        let (peer, now) = (PeerId(1), Instant::now());
        let to = (MID.0 + 4.0, 0.0, MID.2);
        let (got, _) = s.trusted_move(peer, MID, to, now + Duration::from_secs(10));
        assert!(got.0 - MID.0 <= MAX_BUDGET + 1e-3);
    }

    #[test]
    fn off_the_floor_the_box_stays_put() {
        let mut s = SmokeState::new();
        let (peer, now) = (PeerId(1), Instant::now());
        // just inside the leftmost area's left edge, the border wall is past it
        let (x, y, _, h) = *crate::maze::arena::ARENA_AREAS.iter().min_by_key(|a| a.0).unwrap();
        let c = crate::server::game::MAZE_CELL_SIZE;
        let inside = (x as f32 * c + 0.2, 0.0, (y as f32 + h as f32 / 2.0) * c);
        let (got, _) = s.trusted_move(peer, inside, (inside.0 - 0.4, 0.0, inside.2), now);
        assert_eq!(got, inside);
    }

    #[test]
    fn a_short_stall_catches_up_without_a_correction() {
        let mut s = SmokeState::new();
        let (peer, now) = (PeerId(1), Instant::now());
        // the game froze for 1.2 s while the box kept going at box speed
        let to = (MID.0 + BOX_SPEED * 1.2, 0.0, MID.2);
        let (got, fix) = s.trusted_move(peer, MID, to, now + Duration::from_millis(1200));
        assert_eq!((got, fix), (to, false));
    }

    #[test]
    fn a_line_between_two_rooms_past_the_hallway_is_not_on_floor() {
        let c = crate::server::game::MAZE_CELL_SIZE;
        // the first two rooms at the same depth, outside the hallway's band: both ends are floor, the wall between isn't
        let (a, b) = ((8.5 * c, 0.0, 2.0 * c), (13.5 * c, 0.0, 2.0 * c));
        assert!(area_of(a.0, a.2).is_some() && area_of(b.0, b.2).is_some());
        assert!(!on_floor_along(a, b));
        // through the hallway is fine
        let (a, b) = ((8.5 * c, 0.0, 5.0 * c), (13.5 * c, 0.0, 5.0 * c));
        assert!(on_floor_along(a, b));
    }
}
