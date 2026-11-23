//! `unblocked_hit` against the entity check `fire_hitscan` did before it:
//! the original ray test at full range, then the blocked check.

use macroquad::prelude::*;

use super::unblocked_hit;
use crate::weapons::combat::reference;
use crate::weapons::combat::tests::{random_dir, random_entity, random_origin, Rng};
use crate::weapons::combat::{CombatEntity, HitZone};

/// The original per-entity check in `fire_hitscan`.
fn unblocked_hit_reference(
    entity: &impl CombatEntity,
    eye: Vec3,
    fwd: Vec3,
    range: f32,
    closest_blocking_dist: Option<f32>,
) -> Option<(HitZone, f32)> {
    let (zone, dist) = reference::ray_test(entity, eye, fwd, range)?;
    let blocked = closest_blocking_dist.map_or(false, |d| d < dist);
    (!blocked).then_some((zone, dist))
}

#[test]
fn unblocked_hit_matches_original_bit_for_bit() {
    let mut rng = Rng(0xA076_1D64_78BD_642F);
    let (mut hits, mut blocked) = (0usize, 0usize);
    for _ in 0..20_000 {
        let e = random_entity(&mut rng);
        for _ in 0..10 {
            let (center, half) = match e.boxes.len() {
                0 => (e.position, Vec3::ONE),
                n => {
                    let b = &e.boxes[rng.int(0, n as i32 - 1) as usize];
                    (e.position + b.center_offset, b.half_extents)
                }
            };
            let eye = random_origin(&mut rng, center, half);
            let fwd = if rng.int(0, 2) == 0 {
                random_dir(&mut rng)
            } else {
                (center + half * rng.f(-1.0, 1.0) - eye).normalize_or_zero()
            };
            let range = match rng.int(0, 3) {
                0 => rng.f(0.0, 3.0),
                _ => rng.f(0.0, 200.0),
            };
            let full = reference::ray_test(&e, eye, fwd, f32::INFINITY).map(|(_, d)| d);
            let blocking = match rng.int(0, 6) {
                0 => None,
                1 => Some(0.0),
                2 => Some(range + rng.f(0.0, 5.0)),
                // the wall sits exactly where a hitbox is hit
                3 | 4 => full.or(Some(range)),
                _ => Some(rng.f(0.0, range.max(0.1))),
            };
            let old = unblocked_hit_reference(&e, eye, fwd, range, blocking);
            let new = unblocked_hit(&e, eye, fwd, range, blocking);
            assert_eq!(
                old.map(|(z, d)| (z, d.to_bits())),
                new.map(|(z, d)| (z, d.to_bits())),
                "eye={eye:?} fwd={fwd:?} range={range} blocking={blocking:?}"
            );
            hits += old.is_some() as usize;
            let raw = reference::ray_test(&e, eye, fwd, range);
            blocked += (raw.is_some() && old.is_none()) as usize;
        }
    }
    assert!(hits > 20_000, "only {hits} hits");
    assert!(blocked > 5_000, "only {blocked} blocked hits");
}

#[test]
fn shots_pass_through_teammates_only_in_team_mode_matches() {
    use super::skips_teammate;
    use crate::net::protocols::{MODE_GUN_GAME, MODE_SEARCH_AND_DESTROY, MODE_TEAM_DEATHMATCH};
    for mode in [MODE_TEAM_DEATHMATCH, MODE_SEARCH_AND_DESTROY] {
        assert!(skips_teammate(true, Some(mode), 1, 1));
        assert!(skips_teammate(true, Some(mode), 2, 2));
        assert!(!skips_teammate(true, Some(mode), 1, 2), "enemies are hit");
        assert!(!skips_teammate(true, Some(mode), 0, 0), "team not known yet");
        assert!(!skips_teammate(false, Some(mode), 1, 1), "the lobby keeps its rules");
    }
    assert!(!skips_teammate(true, Some(MODE_GUN_GAME), 0, 0));
    assert!(!skips_teammate(true, None, 1, 1));
}
