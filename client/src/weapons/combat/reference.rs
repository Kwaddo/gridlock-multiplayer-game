//! The ray tests as they were before the reciprocal direction was hoisted out
//! of them, kept verbatim as the oracle the faster versions must match bit
//! for bit.

use macroquad::prelude::*;

use super::{CombatEntity, HitZone};

/// The original `CombatEntity::ray_test` default method.
pub(crate) fn ray_test<E: CombatEntity + ?Sized>(
    e: &E,
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
) -> Option<(HitZone, f32)> {
    let mut best: Option<(HitZone, f32)> = None;
    for hit_box in e.hitboxes() {
        let center = e.entity_position() + hit_box.center_offset;
        if let Some(dist) = ray_hit_aabb(origin, dir, center, hit_box.half_extents, max_dist) {
            match best {
                None => best = Some((hit_box.zone, dist)),
                Some((_, d)) if dist < d => best = Some((hit_box.zone, dist)),
                _ => {}
            }
        }
    }
    best
}

/// The original `ray_hit_aabb`.
pub(crate) fn ray_hit_aabb(
    origin: Vec3,
    dir: Vec3,
    center: Vec3,
    half: Vec3,
    max_dist: f32,
) -> Option<f32> {
    let min = center - half;
    let max = center + half;

    let mut tmin = 0.0_f32;
    let mut tmax = max_dist;

    for i in 0..3 {
        let o = match i {
            0 => origin.x,
            1 => origin.y,
            _ => origin.z,
        };
        let d = match i {
            0 => dir.x,
            1 => dir.y,
            _ => dir.z,
        };
        let bmin = match i {
            0 => min.x,
            1 => min.y,
            _ => min.z,
        };
        let bmax = match i {
            0 => max.x,
            1 => max.y,
            _ => max.z,
        };

        if d.abs() < 1e-8 {
            // ray parallel to slab — miss if origin outside
            if o < bmin || o > bmax {
                return None;
            }
        } else {
            let inv_d = 1.0 / d;
            let mut t1 = (bmin - o) * inv_d;
            let mut t2 = (bmax - o) * inv_d;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            tmin = tmin.max(t1);
            tmax = tmax.min(t2);
            if tmin > tmax {
                return None;
            }
        }
    }
    Some(tmin)
}

/// The original `ray_hit_aabb_with_normal`.
pub(crate) fn ray_hit_aabb_with_normal(
    origin: Vec3,
    dir: Vec3,
    center: Vec3,
    half: Vec3,
    max_dist: f32,
) -> Option<(f32, Vec3)> {
    let min = center - half;
    let max = center + half;

    let mut tmin = 0.0_f32;
    let mut tmax = max_dist;
    let mut hit_axis: usize = 0;
    let mut hit_sign: f32 = -1.0;

    for i in 0..3 {
        let o = match i {
            0 => origin.x,
            1 => origin.y,
            _ => origin.z,
        };
        let d = match i {
            0 => dir.x,
            1 => dir.y,
            _ => dir.z,
        };
        let bmin = match i {
            0 => min.x,
            1 => min.y,
            _ => min.z,
        };
        let bmax = match i {
            0 => max.x,
            1 => max.y,
            _ => max.z,
        };

        if d.abs() < 1e-8 {
            if o < bmin || o > bmax {
                return None;
            }
        } else {
            let inv_d = 1.0 / d;
            let mut t1 = (bmin - o) * inv_d;
            let mut t2 = (bmax - o) * inv_d;
            let enters_from_min_side = d > 0.0;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            if t1 > tmin {
                tmin = t1;
                hit_axis = i;
                hit_sign = if enters_from_min_side { -1.0 } else { 1.0 };
            }
            tmax = tmax.min(t2);
            if tmin > tmax {
                return None;
            }
        }
    }

    let normal = match hit_axis {
        0 => vec3(hit_sign, 0.0, 0.0),
        1 => vec3(0.0, hit_sign, 0.0),
        _ => vec3(0.0, 0.0, hit_sign),
    };
    Some((tmin, normal))
}
