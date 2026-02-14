use crate::weapons::stats::WeaponStats;
use macroquad::prelude::*;

/*
    Actual combat entity, we make this a trait so we can apply it to anything we want
    makes it easy to scale and add new enemy types later, like a moving dummy or a real enemy with AI,
    just implement this trait and it works with the same shooting code
*/


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind{
    Player,
    Dummy,
    MovingTarget,
    Bot,
}

pub trait CombatEntity {
    fn entity_position(&self) -> Vec3;
    fn hitboxes(&self) -> &[Hitbox];
    fn take_damage(&mut self, dmg: f32);
    fn is_alive(&self) -> bool;

    // ray test
    fn ray_test(&self, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<(HitZone, f32)> {
        let mut best: Option<(HitZone, f32)> = None;
        for hit_box in self.hitboxes() {
            let center = self.entity_position() + hit_box.center_offset;
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

    fn entity_kind(&self) -> EntityKind;
    fn entity_id(&self) -> Option<u32>;
    fn update(&mut self, delta: f32);
    fn hit_timer(&self) -> f32; // time since last hit, for hit feedback like flashing redfo
    fn entity_color(&self) -> Color; // rendering colors since we dont have models yet

}

// Hit zones: split player into head, body, and limbs for multipliers
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HitZone {
    Head,
    Torso,
    Limbs,
}

impl HitZone {
    pub fn damage_mult(&self) -> f32 {
        match self {
            HitZone::Head => 1.0, // per-weapon headshot_mult from stats is used instead
            HitZone::Torso => 1.0,
            HitZone::Limbs => 0.75,
        }
    }
}

pub struct Hitbox {
    pub center_offset: Vec3, // offset from player's position to center of this hitbox
    pub half_extents: Vec3,  // size of the hitbox (x=width/2, y=height/2, z=depth/2)
    pub zone: HitZone,       // which hitzone this box represents (head, torso, limbs)
}

/*
    Ray vs AABB intersection
    AABB = axis-aligned bounding box, defined by center and half-extents, its just a rectangular box
    that is not rotated, so its faces are aligned with the coordinate axes. This makes intersection tests simpler.
    half_extents is half the size of this box in each direction, like a radius in a triangle.

    this function returns true if a ray from `origin` in `dir` hits an axis-aligned box
    centered at `center` with half-extents `half`, within `max_dist`.
    in other words, when a ray is shot, we check if it intersects the AABB (hitbox) of the target enemy,
    and if so, we can determine which hit zone was hit and apply damage accordingly.
*/
pub fn ray_hit_aabb(
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

pub fn damage_falloff(distance: f32, stats: &WeaponStats) -> f32 {
    if distance <= stats.falloff_start {
        1.0
    } else if distance >= stats.falloff_end || stats.falloff_start >= stats.falloff_end {
        stats.falloff_min_mult
    } else {
        let t = (distance - stats.falloff_start) / (stats.falloff_end - stats.falloff_start);
        1.0 + t * (stats.falloff_min_mult - 1.0)
    }
}

pub fn default_player_hitboxes() -> Vec<Hitbox> {
    // Head:  1.35 → 1.65
    // Torso: 0.50 → 1.35  (flush with head bottom and legs top — no gaps)
    // Arms:  0.50 → 1.35  (match torso height)
    // Legs:  0.00 → 0.50
    vec![
        Hitbox {
            center_offset: vec3(0.0, 1.5, 0.0),
            half_extents: vec3(0.15, 0.15, 0.15),
            zone: HitZone::Head,
        },
        Hitbox {
            center_offset: vec3(0.0, 0.925, 0.0),
            half_extents: vec3(0.25, 0.425, 0.15),
            zone: HitZone::Torso,
        },
        Hitbox {
            center_offset: vec3(-0.4, 0.925, 0.0),
            half_extents: vec3(0.1, 0.425, 0.1),
            zone: HitZone::Limbs,
        },
        Hitbox {
            center_offset: vec3(0.4, 0.925, 0.0),
            half_extents: vec3(0.1, 0.425, 0.1),
            zone: HitZone::Limbs,
        },
        Hitbox {
            center_offset: vec3(-0.12, 0.25, 0.0),
            half_extents: vec3(0.1, 0.25, 0.1),
            zone: HitZone::Limbs,
        },
        Hitbox {
            center_offset: vec3(0.12, 0.25, 0.0),
            half_extents: vec3(0.1, 0.25, 0.1),
            zone: HitZone::Limbs,
        },
    ]
}
