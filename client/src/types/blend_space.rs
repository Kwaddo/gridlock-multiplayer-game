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
    // kept sorted by direction (a test checks), so ring_mix doesnt copy and sort them every call
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
                (-90.0, PlayerSkin::WalkLeft),
                (0.0, PlayerSkin::RifleWalk),
                (90.0, PlayerSkin::WalkRight),
                (180.0, PlayerSkin::WalkBack),
            ],
        },
        Ring {
            speed: 8.0,
            clips: &[
                (-90.0, PlayerSkin::SprintLeft),
                (0.0, PlayerSkin::SprintForward),
                (90.0, PlayerSkin::SprintRight),
                (180.0, PlayerSkin::WalkBack),
            ],
        },
    ],
};

// the most clips one mix can hold: two rings, two neighbours on each
pub const MAX_MIX: usize = 4;

// a clip under this share of the mix cant be seen but still costs a whole sample, see Mix::drop_faint
const FAINT: f32 = 0.02;

// clips and how much of each, at most MAX_MIX, kept on the stack instead of a Vec every tick
#[derive(Clone, Copy, Debug)]
pub struct Mix {
    items: [(PlayerSkin, f32); MAX_MIX],
    len: usize,
}

impl Mix {
    fn new(fill: PlayerSkin) -> Self {
        Self { items: [(fill, 0.0); MAX_MIX], len: 0 }
    }

    // a clip twice (walk back is on both rings) adds up into one entry
    fn add(&mut self, skin: PlayerSkin, w: f32) {
        if w <= 0.0 {
            return;
        }
        match self.items[..self.len].iter_mut().find(|(s, _)| *s == skin) {
            Some(entry) => entry.1 += w,
            None => {
                self.items[self.len] = (skin, w);
                self.len += 1;
            }
        }
    }

    // fade out clips under FAINT (FAINT..2*FAINT ramps up, so a clip doesnt pop in or out) and share their weight out
    // before this a steady walker on the ring edge sampled 3-4 clips where 2 showed
    fn drop_faint(&mut self) {
        let (mut kept, mut total) = (0, 0.0);
        for i in 0..self.len {
            let (skin, w) = self.items[i];
            let w = w * ((w - FAINT) / FAINT).clamp(0.0, 1.0);
            if w > 0.0 {
                self.items[kept] = (skin, w);
                kept += 1;
                total += w;
            }
        }
        self.len = kept;
        self.items[..kept].iter_mut().for_each(|e| e.1 /= total);
    }
}

impl std::ops::Deref for Mix {
    type Target = [(PlayerSkin, f32)];
    fn deref(&self) -> &Self::Target {
        &self.items[..self.len]
    }
}

// movement relative to facing (x = right, y = forward) -> clips and how much of each, always adds up to 1
pub fn weights(space: &BlendSpace, local_vel: Vec2) -> Mix {
    let mut out = Mix::new(space.idle);
    let speed = local_vel.length();
    if speed < 1e-3 || space.rings.is_empty() {
        out.add(space.idle, 1.0);
        return out;
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

    match inner {
        None => out.add(space.idle, 1.0 - t),
        Some(ring) => ring_mix(ring, angle).into_iter().for_each(|(s, w)| out.add(s, w * (1.0 - t))),
    }
    ring_mix(outer, angle).into_iter().for_each(|(s, w)| out.add(s, w * t));
    out.drop_faint();
    out
}

// on one ring, the two clips either side of angle, split by how close it is to each
// the clips are already sorted by direction, a lone clip comes back with an empty partner
fn ring_mix(ring: &Ring, angle: f32) -> [(PlayerSkin, f32); 2] {
    let clips = ring.clips;
    let n = clips.len();
    if n == 1 {
        return [(clips[0].1, 1.0), (clips[0].1, 0.0)];
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
            return [(a_skin, 1.0 - t), (b_skin, t)];
        }
    }
    [(clips[0].1, 1.0), (clips[0].1, 0.0)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use PlayerSkin::*;

    fn w(space: &BlendSpace, x: f32, y: f32) -> Vec<(PlayerSkin, f32)> {
        weights(space, vec2(x, y)).to_vec()
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
                // dropped clips are gone, not left in at 0
                assert!(mix.len() <= MAX_MIX && mix.iter().all(|(_, w)| *w > 0.0), "{deg} {speed}: {mix:?}");
            }
        }
    }

    // a steady walker on the ring edge, 5.2 m/s and 3 degrees off forward
    // before drop_faint this sampled 4 clips, two of them under 7%
    #[test]
    fn faint_clips_are_dropped_and_the_rest_still_add_up() {
        let r = (-3f32).to_radians();
        let mix = w(&STANDING, r.sin() * 5.2, r.cos() * 5.2);
        assert_eq!(mix.len(), 3, "{mix:?}");
        assert_eq!(of(&mix, SprintLeft), 0.0, "{mix:?}");
        let sum: f32 = mix.iter().map(|(_, w)| w).sum();
        assert!((sum - 1.0).abs() < 1e-4, "{mix:?}");
        // barely moving is all idle, the moving clips are too faint to show
        assert_eq!(w(&STANDING, 0.0, 0.05), vec![(RifleIdle, 1.0)]);
    }

    // ring_mix relies on this instead of sorting every call
    #[test]
    fn ring_clips_are_sorted_by_direction() {
        for ring in STANDING.rings {
            assert!(ring.clips.windows(2).all(|p| p[0].0 < p[1].0));
        }
    }
}
