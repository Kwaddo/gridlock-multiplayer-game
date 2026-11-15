//! Random mazes for the differential tests, the prop height band check, and
//! `clamp_move` against its original version.

use macroquad::prelude::*;
use std::collections::{HashMap, HashSet};

use super::{CeilingLightInfo, Maze, Prop, PropKind};
use crate::weapons::combat::tests::Rng;

/// A random maze with walls, props of every kind (several per cell at
/// times) and ceiling lights, some on wall cells or stacked twice.
pub(crate) fn random_maze(rng: &mut Rng) -> Maze {
    let width = rng.int(3, 43) as u8;
    let height = rng.int(3, 43) as u8;
    let wall_chance = rng.f(0.05, 0.7);
    let cells = (0..width as usize * height as usize)
        .map(|_| if rng.f(0.0, 1.0) < wall_chance { 0 } else { 1 })
        .collect();
    let cell = |rng: &mut Rng| (rng.int(0, width as i32 - 1) as u8, rng.int(0, height as i32 - 1) as u8);
    let props = (0..rng.int(0, 130))
        .map(|_| {
            let (x, y) = cell(rng);
            Prop { kind: PropKind::from_u8(rng.int(1, 8) as u8).unwrap(), x, y }
        })
        .collect();
    let ceiling_lights = (0..rng.int(0, 80))
        .map(|_| {
            let (x, y) = cell(rng);
            CeilingLightInfo { x, y, variant: rng.int(0, 3) as u8 }
        })
        .collect();
    Maze {
        width,
        height,
        cells,
        cell_size: if rng.int(0, 3) == 0 { rng.f(0.5, 3.0) } else { 1.8 },
        wall_height: if rng.int(0, 3) == 0 { rng.f(1.0, 4.0) } else { 2.4 },
        teleporter: (1, 1),
        rooms: Vec::new(),
        props,
        ceiling_lights,
        breakable_walls: HashMap::new(),
        broken_wall_scars: HashSet::new(),
    }
}

#[test]
fn bullet_hitboxes_fit_in_band() {
    let (lo, hi) = PropKind::BULLET_HITBOX_Y;
    let mut rng = Rng(11);
    let m = random_maze(&mut rng);
    for v in 0..=255u8 {
        let Some(kind) = PropKind::from_u8(v) else { continue };
        let Some((offset, half)) = kind.bullet_hitbox() else { continue };
        // the very arithmetic the ray test does for a prop's box
        for (x, y) in [(0, 0), (m.width as i32 - 1, m.height as i32 - 1), (3, 7)] {
            let center = m.cell_to_world(x, y) + offset;
            let (min, max) = (center - half, center + half);
            assert!(lo <= min.y.min(max.y), "{kind:?} bottom {} below band", min.y);
            assert!(hi >= max.y.max(min.y), "{kind:?} top {} above band", max.y);
        }
    }
}

/// The original `clamp_move`.
fn clamp_move_reference(m: &Maze, current: Vec3, desired: Vec3, radius: f32) -> Vec3 {
    let mut result = desired;
    if m.is_blocked(result, radius) {
        result.x = current.x;
        if m.is_blocked(result, radius) {
            result.x = desired.x;
            result.z = current.z;
            if m.is_blocked(result, radius) {
                result = current;
            }
        }
    }
    result
}

#[test]
fn clamp_move_matches_original_bit_for_bit() {
    let mut rng = Rng(0x5851_F42D_4C95_7F2D);
    for _ in 0..200 {
        let m = random_maze(&mut rng);
        let (w, h) = (m.width as f32 * m.cell_size, m.height as f32 * m.cell_size);
        for _ in 0..500 {
            let mut current = vec3(rng.f(-1.0, w + 1.0), rng.f(-1.0, 2.0), rng.f(-1.0, h + 1.0));
            if rng.int(0, 5) == 0 {
                // on the map edge, where signed zeros can show up
                current.x = 0.0;
                current.z = -0.0;
            }
            let flip_zero = |v: f32| if v == 0.0 { -v } else { v };
            let desired = match rng.int(0, 4) {
                0 => current,
                // equal by == but with the other signed zero
                1 => vec3(flip_zero(current.x), current.y, flip_zero(current.z)),
                2 => vec3(current.x, rng.f(-1.0, 2.0), current.z),
                _ => current + vec3(rng.f(-0.3, 0.3), 0.0, rng.f(-0.3, 0.3)),
            };
            let radius = rng.f(0.0, 0.6);
            let old = clamp_move_reference(&m, current, desired, radius);
            let new = m.clamp_move(current, desired, radius);
            assert_eq!(
                old.to_array().map(f32::to_bits),
                new.to_array().map(f32::to_bits),
                "current={current:?} desired={desired:?} r={radius}"
            );
        }
    }
}
