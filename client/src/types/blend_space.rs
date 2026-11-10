use macroquad::prelude::*;

use crate::types::player::PlayerSkin;

/*
    a 2d blend space where clips are placed by move direction and speed
    the body's real movement picks a mix of the nearest clips instead of one clip
    rings go outward by speed, idle sits in the middle
*/

pub struct Ring {
    pub speed: f32,
    pub clips: &'static [(f32, PlayerSkin)], // (direction in degrees, 0 forward, 90 right, -90 left, 180 back)
}

pub struct BlendSpace {
    pub idle: PlayerSkin,
    pub rings: &'static [Ring],
}

// standing: walk ring at 5 m/s, sprint ring at 8, sprinting back has no clip so walk back stands in
pub static STANDING: BlendSpace = BlendSpace {
    idle: PlayerSkin::RifleIdle,
    rings: &[
        Ring {
            speed: 5.0,
            clips: &[
                (0.0, PlayerSkin::RifleWalk),
                (90.0, PlayerSkin::WalkRight),
                (180.0, PlayerSkin::WalkBack),
                (-90.0, PlayerSkin::WalkLeft),
            ],
        },
        Ring {
            speed: 8.0,
            clips: &[
                (0.0, PlayerSkin::SprintForward),
                (90.0, PlayerSkin::SprintRight),
                (180.0, PlayerSkin::WalkBack),
                (-90.0, PlayerSkin::SprintLeft),
            ],
        },
    ],
};

// movement relative to facing (x = right, y = forward) -> clips and how much of each, always adds up to 1
pub fn weights(space: &BlendSpace, local_vel: Vec2) -> Vec<(PlayerSkin, f32)> {
    let speed = local_vel.length();
    if speed < 1e-3 || space.rings.is_empty() {
        return vec![(space.idle, 1.0)];
    }
    let angle = local_vel.x.atan2(local_vel.y).to_degrees();

    // idle at speed 0, then each ring, find the two either side of our speed
    let below = space.rings.iter().rposition(|r| r.speed <= speed);
    let (inner, outer, t) = match below {
        None => (None, &space.rings[0], speed / space.rings[0].speed),
        Some(i) if i + 1 == space.rings.len() => (Some(&space.rings[i]), &space.rings[i], 1.0),
        Some(i) => {
            let (a, b) = (&space.rings[i], &space.rings[i + 1]);
            (Some(a), b, (speed - a.speed) / (b.speed - a.speed))
        }
    };

    let mut out: Vec<(PlayerSkin, f32)> = Vec::new();
    let mut add = |skin: PlayerSkin, w: f32| {
        if w <= 0.0 {
            return;
        }
        match out.iter_mut().find(|(s, _)| *s == skin) {
            Some(entry) => entry.1 += w,
            None => out.push((skin, w)),
        }
    };
    match inner {
        None => add(space.idle, 1.0 - t),
        Some(ring) => ring_mix(ring, angle).into_iter().for_each(|(s, w)| add(s, w * (1.0 - t))),
    }
    ring_mix(outer, angle).into_iter().for_each(|(s, w)| add(s, w * t));
    out
}

// on one ring, the two clips either side of angle, split by how close it is to each
fn ring_mix(ring: &Ring, angle: f32) -> Vec<(PlayerSkin, f32)> {
    let mut clips: Vec<(f32, PlayerSkin)> = ring.clips.to_vec();
    clips.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let n = clips.len();
    if n == 1 {
        return vec![(clips[0].1, 1.0)];
    }
    for i in 0..n {
        let (a, a_skin) = clips[i];
        let (mut b, b_skin) = clips[(i + 1) % n];
        if b <= a {
            b += 360.0; // the last clip wraps round to the first
        }
        let mut x = angle;
        if x < a {
            x += 360.0;
        }
        if x >= a && x <= b {
            let t = (x - a) / (b - a);
            return vec![(a_skin, 1.0 - t), (b_skin, t)];
        }
    }
    vec![(clips[0].1, 1.0)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use PlayerSkin::*;

    fn w(space: &BlendSpace, x: f32, y: f32) -> Vec<(PlayerSkin, f32)> {
        weights(space, vec2(x, y))
    }

    fn of(mix: &[(PlayerSkin, f32)], skin: PlayerSkin) -> f32 {
        mix.iter().filter(|(s, _)| *s == skin).map(|(_, w)| w).sum()
    }

    #[test]
    fn standing_still_is_idle() {
        assert_eq!(w(&STANDING, 0.0, 0.0), vec![(RifleIdle, 1.0)]);
    }

    #[test]
    fn straight_forward_at_walk_speed_is_all_walk() {
        let mix = w(&STANDING, 0.0, 5.0);
        assert!((of(&mix, RifleWalk) - 1.0).abs() < 1e-4, "{mix:?}");
    }

    #[test]
    fn diagonal_is_half_forward_half_right() {
        let d = 5.0 / 2f32.sqrt();
        let mix = w(&STANDING, d, d);
        assert!((of(&mix, RifleWalk) - 0.5).abs() < 1e-3, "{mix:?}");
        assert!((of(&mix, WalkRight) - 0.5).abs() < 1e-3, "{mix:?}");
    }

    #[test]
    fn half_walk_speed_is_half_idle() {
        let mix = w(&STANDING, 0.0, 2.5);
        assert!((of(&mix, RifleIdle) - 0.5).abs() < 1e-4, "{mix:?}");
        assert!((of(&mix, RifleWalk) - 0.5).abs() < 1e-4, "{mix:?}");
    }

    #[test]
    fn between_walk_and_sprint_is_a_mix() {
        let mix = w(&STANDING, 0.0, 6.5);
        assert!((of(&mix, RifleWalk) - 0.5).abs() < 1e-4, "{mix:?}");
        assert!((of(&mix, SprintForward) - 0.5).abs() < 1e-4, "{mix:?}");
    }

    #[test]
    fn sprinting_back_uses_walk_back() {
        let mix = w(&STANDING, 0.0, -8.0);
        assert!((of(&mix, WalkBack) - 1.0).abs() < 1e-4, "{mix:?}");
    }

    #[test]
    fn weights_always_add_up_to_one() {
        for deg in (-180..=180).step_by(15) {
            for speed in [0.5, 2.0, 5.0, 6.0, 8.0, 12.0] {
                let r = (deg as f32).to_radians();
                let mix = w(&STANDING, r.sin() * speed, r.cos() * speed);
                let sum: f32 = mix.iter().map(|(_, w)| w).sum();
                assert!((sum - 1.0).abs() < 1e-4, "{deg} {speed}: {mix:?}");
                assert!(mix.iter().all(|(_, w)| *w >= 0.0));
            }
        }
    }
}
