//! Differential tests: the hoisted-reciprocal ray tests against the original
//! versions in `reference`, compared bit for bit.

use macroquad::prelude::*;

use super::reference;
use super::*;

/// Small deterministic xorshift so the test needs no RNG setup.
pub(crate) struct Rng(pub(crate) u64);

impl Rng {
    pub(crate) fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub(crate) fn f(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (self.next() >> 40) as f32 / (1u64 << 24) as f32 * (hi - lo)
    }
    pub(crate) fn int(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next() % (hi - lo + 1) as u64) as i32
    }
}

/// A direction component that is often exactly zero, signed zero, or right
/// around the 1e-8 parallel cut-off.
pub(crate) fn edgy_component(rng: &mut Rng) -> f32 {
    match rng.int(0, 9) {
        0 => 0.0,
        1 => -0.0,
        2 => 1e-8,
        3 => -1e-8,
        4 => rng.f(-2e-8, 2e-8),
        5 => 1.0,
        _ => rng.f(-1.0, 1.0),
    }
}

/// A random direction; normalized most of the time, raw (possibly all zero)
/// otherwise, as nothing in the ray test requires unit length.
pub(crate) fn random_dir(rng: &mut Rng) -> Vec3 {
    let d = vec3(edgy_component(rng), edgy_component(rng), edgy_component(rng));
    if rng.int(0, 3) != 0 && d.length() > 1e-6 {
        d.normalize()
    } else {
        d
    }
}

pub(crate) fn random_max_dist(rng: &mut Rng) -> f32 {
    match rng.int(0, 7) {
        0 => 0.0,
        1 => -rng.f(0.0, 1.0),
        2 => f32::INFINITY,
        3 => rng.f(0.0, 2.0),
        _ => rng.f(0.0, 60.0),
    }
}

pub(crate) fn bits(v: Vec3) -> [u32; 3] {
    v.to_array().map(f32::to_bits)
}

pub(crate) fn random_box(rng: &mut Rng) -> (Vec3, Vec3) {
    let center = vec3(rng.f(-5.0, 5.0), rng.f(-1.0, 3.0), rng.f(-5.0, 5.0));
    let half = match rng.int(0, 9) {
        // degenerate: a face or an edge
        0 => vec3(0.0, rng.f(0.0, 1.0), rng.f(0.0, 1.0)),
        // inside-out, the slab swap still makes these hittable on some axes
        1 => vec3(-rng.f(0.0, 0.5), rng.f(0.0, 1.0), rng.f(0.0, 1.0)),
        _ => vec3(rng.f(0.01, 1.0), rng.f(0.01, 1.0), rng.f(0.01, 1.0)),
    };
    (center, half)
}

/// Origins near, inside, or exactly on a face/corner of the box.
pub(crate) fn random_origin(rng: &mut Rng, center: Vec3, half: Vec3) -> Vec3 {
    let (min, max) = (center - half, center + half);
    let mut o = vec3(rng.f(-8.0, 8.0), rng.f(-2.0, 4.0), rng.f(-8.0, 8.0));
    match rng.int(0, 4) {
        0 => o = center + half * vec3(rng.f(-1.0, 1.0), rng.f(-1.0, 1.0), rng.f(-1.0, 1.0)),
        1 => o.x = if rng.int(0, 1) == 0 { min.x } else { max.x },
        2 => {
            o.y = if rng.int(0, 1) == 0 { min.y } else { max.y };
            o.z = if rng.int(0, 1) == 0 { min.z } else { max.z };
        }
        _ => {}
    }
    o
}

#[test]
fn hoisted_aabb_tests_match_original_bit_for_bit() {
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    let mut hits = 0usize;
    for _ in 0..200_000 {
        let (center, half) = random_box(&mut rng);
        let origin = random_origin(&mut rng, center, half);
        let dir = random_dir(&mut rng);
        let mut max_dist = random_max_dist(&mut rng);
        // Max range exactly at the surface.
        if rng.int(0, 3) == 0 {
            if let Some(d) = reference::ray_hit_aabb(origin, dir, center, half, f32::INFINITY) {
                max_dist = d;
            }
        }
        let ctx = format!("o={origin:?} d={dir:?} c={center:?} h={half:?} max={max_dist}");

        let old = reference::ray_hit_aabb(origin, dir, center, half, max_dist);
        let new = ray_hit_aabb(origin, dir, center, half, max_dist);
        let inv = ray_inv_dir(dir);
        let new_inv = ray_hit_aabb_inv(origin, dir, inv, center, half, max_dist);
        assert_eq!(old.map(f32::to_bits), new.map(f32::to_bits), "{ctx}");
        assert_eq!(old.map(f32::to_bits), new_inv.map(f32::to_bits), "{ctx}");
        hits += old.is_some() as usize;

        let old_n = reference::ray_hit_aabb_with_normal(origin, dir, center, half, max_dist);
        let new_n = ray_hit_aabb_with_normal(origin, dir, center, half, max_dist);
        let new_n_inv = ray_hit_aabb_with_normal_inv(origin, dir, inv, center, half, max_dist);
        let key = |h: Option<(f32, Vec3)>| h.map(|(d, n)| (d.to_bits(), bits(n)));
        assert_eq!(key(old_n), key(new_n), "normal {ctx}");
        assert_eq!(key(old_n), key(new_n_inv), "normal {ctx}");
    }
    assert!(hits > 20_000, "only {hits} hits");
}

pub(crate) struct TestEntity {
    pub(crate) position: Vec3,
    pub(crate) boxes: Vec<Hitbox>,
}

impl CombatEntity for TestEntity {
    fn entity_position(&self) -> Vec3 {
        self.position
    }
    fn hitboxes(&self) -> &[Hitbox] {
        &self.boxes
    }
    fn take_damage(&mut self, _dmg: f32) {}
    fn is_alive(&self) -> bool {
        true
    }
    fn entity_kind(&self) -> EntityKind {
        EntityKind::Dummy
    }
    fn entity_id(&self) -> Option<u32> {
        None
    }
    fn update(&mut self, _delta: f32) {}
    fn hit_timer(&self) -> f32 {
        0.0
    }
    fn entity_color(&self) -> Color {
        WHITE
    }
}

/// An entity with a random set of hitboxes: the default player stack,
/// overlapping random boxes, duplicates (ties) or none at all.
pub(crate) fn random_entity(rng: &mut Rng) -> TestEntity {
    let position = vec3(rng.f(-6.0, 6.0), rng.f(-0.5, 1.0), rng.f(-6.0, 6.0));
    let zone = |rng: &mut Rng| match rng.int(0, 2) {
        0 => HitZone::Head,
        1 => HitZone::Torso,
        _ => HitZone::Limbs,
    };
    let boxes = match rng.int(0, 5) {
        0 => default_player_hitboxes(),
        1 => Vec::new(),
        _ => {
            let n = rng.int(1, 12);
            let mut v: Vec<Hitbox> = Vec::new();
            for _ in 0..n {
                if !v.is_empty() && rng.int(0, 5) == 0 {
                    // Same box again under another zone: the first must win.
                    let b = &v[rng.int(0, v.len() as i32 - 1) as usize];
                    let (c, h) = (b.center_offset, b.half_extents);
                    v.push(Hitbox { center_offset: c, half_extents: h, zone: zone(rng) });
                    continue;
                }
                let (c, h) = random_box(rng);
                v.push(Hitbox {
                    center_offset: c * 0.3 + vec3(0.0, 0.8, 0.0),
                    half_extents: h * 0.4,
                    zone: zone(rng),
                });
            }
            v
        }
    };
    TestEntity { position, boxes }
}

#[test]
fn entity_ray_test_matches_original_bit_for_bit() {
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    let mut hits = 0usize;
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
            let origin = random_origin(&mut rng, center, half);
            let mut dir = random_dir(&mut rng);
            if rng.int(0, 2) == 0 {
                // aim at the entity so most rays hit something
                dir = (center + half * rng.f(-1.0, 1.0) - origin).normalize_or_zero();
            }
            let mut max_dist = random_max_dist(&mut rng);
            if rng.int(0, 3) == 0 {
                if let Some((_, d)) = reference::ray_test(&e, origin, dir, f32::INFINITY) {
                    max_dist = d;
                }
            }
            let old = reference::ray_test(&e, origin, dir, max_dist);
            let new = e.ray_test(origin, dir, max_dist);
            assert_eq!(
                old.map(|(z, d)| (z, d.to_bits())),
                new.map(|(z, d)| (z, d.to_bits())),
                "o={origin:?} d={dir:?} max={max_dist}"
            );
            hits += old.is_some() as usize;
        }
    }
    assert!(hits > 30_000, "only {hits} hits");
}
