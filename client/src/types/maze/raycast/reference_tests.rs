//! Differential tests: every maze ray test against its original version in
//! `reference`, compared bit for bit (cell, distance and normal) over random
//! mazes with props and lights.

use macroquad::prelude::*;
use std::collections::HashSet;

use super::super::tests::random_maze;
use super::super::Maze;
use super::reference;
use crate::weapons::combat::tests::{bits, random_dir, random_max_dist, Rng};

/// A box in the maze worth aiming at or starting in: a prop's bullet box, a
/// light housing, a wall column, or a ceiling panel.
fn random_target_box(rng: &mut Rng, m: &Maze) -> Option<(Vec3, Vec3)> {
    match rng.int(0, 3) {
        0 if !m.props.is_empty() => {
            let p = m.props[rng.int(0, m.props.len() as i32 - 1) as usize];
            let (off, half) = p.kind.bullet_hitbox()?;
            Some((m.cell_to_world(p.x as i32, p.y as i32) + off, half))
        }
        1 if !m.ceiling_lights.is_empty() => {
            let l = m.ceiling_lights[rng.int(0, m.ceiling_lights.len() as i32 - 1) as usize];
            Some(m.light_housing(l.x, l.y))
        }
        2 => {
            let x = rng.int(0, m.width as i32 - 1) as u8;
            let y = rng.int(0, m.height as i32 - 1) as u8;
            Some(m.wall_box(x, y))
        }
        _ => None,
    }
}

/// A point on, in, or near the box: a face, a corner, or inside.
fn point_at_box(rng: &mut Rng, center: Vec3, half: Vec3) -> Vec3 {
    let (min, max) = (center - half, center + half);
    let pick = |rng: &mut Rng, lo: f32, hi: f32| match rng.int(0, 3) {
        0 => lo,
        1 => hi,
        _ => rng.f(lo, hi),
    };
    vec3(pick(rng, min.x, max.x), pick(rng, min.y, max.y), pick(rng, min.z, max.z))
}

fn random_ray(rng: &mut Rng, m: &Maze) -> (Vec3, Vec3, f32) {
    let cs = m.cell_size;
    let (w, h) = (m.width as f32 * cs, m.height as f32 * cs);
    let mut origin = vec3(rng.f(-3.0, w + 3.0), rng.f(-0.5, 3.5), rng.f(-3.0, h + 3.0));
    match rng.int(0, 7) {
        // eye heights
        0 | 1 => origin.y = rng.f(1.0, 1.8),
        // exactly on the prop band edges, and exactly on the light band
        2 => {
            let (lo, hi) = super::super::PropKind::BULLET_HITBOX_Y;
            origin.y = if rng.int(0, 1) == 0 { lo } else { hi };
        }
        3 => {
            let (c, hf) = m.light_housing(0, 0);
            origin.y = [c.y - hf.y, c.y + hf.y, c.y][rng.int(0, 2) as usize];
        }
        // starting on or inside a box
        4 => {
            if let Some((c, hf)) = random_target_box(rng, m) {
                origin = point_at_box(rng, c, hf);
            }
        }
        // on grid lines
        5 => origin.x = rng.int(-1, m.width as i32 + 1) as f32 * cs,
        _ => {}
    }
    let mut dir = random_dir(rng);
    match rng.int(0, 5) {
        // aim at a box, often grazing its face or corner
        0 | 1 => {
            if let Some((c, hf)) = random_target_box(rng, m) {
                dir = (point_at_box(rng, c, hf) - origin).normalize_or_zero();
            }
        }
        // level shot
        2 => {
            dir.y = 0.0;
            dir = dir.normalize_or_zero();
        }
        _ => {}
    }
    (origin, dir, random_max_dist(rng).max(-0.5) * 2.0)
}

type Key3 = ((u8, u8), u32, [u32; 3]);

fn key3(h: Option<((u8, u8), f32, Vec3)>) -> Option<Key3> {
    h.map(|(c, d, n)| (c, d.to_bits(), bits(n)))
}

fn key2(h: Option<(f32, Vec3)>) -> Option<(u32, [u32; 3])> {
    h.map(|(d, n)| (d.to_bits(), bits(n)))
}

#[test]
fn maze_ray_tests_match_original_bit_for_bit() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ 0xF00D);
    let (mut prop_hits, mut light_hits, mut surface_lights, mut surface_props) = (0, 0, 0, 0);
    for _ in 0..80 {
        let m = random_maze(&mut rng);
        let destroyed: HashSet<(u8, u8)> = m
            .ceiling_lights
            .iter()
            .filter(|_| rng.int(0, 2) == 0)
            .map(|l| (l.x, l.y))
            .collect();
        for _ in 0..2_500 {
            let (o, d, mut max_dist) = random_ray(&mut rng, &m);
            // Max range exactly at the surface the ray hits.
            if rng.int(0, 4) == 0 {
                if let Some(hit) = reference::ray_test_surfaces(&m, o, d, 200.0, &destroyed) {
                    max_dist = hit.0;
                }
            }
            let ctx = format!("o={o:?} d={d:?} max={max_dist} maze={}x{}", m.width, m.height);

            let old = reference::ray_test_props(&m, o, d, max_dist);
            assert_eq!(old.map(f32::to_bits), m.ray_test_props(o, d, max_dist).map(f32::to_bits), "props {ctx}");

            let old = reference::ray_test_walls(&m, o, d, max_dist);
            let new = m.ray_test_walls(o, d, max_dist);
            assert_eq!(old.map(|(c, d)| (c, d.to_bits())), new.map(|(c, d)| (c, d.to_bits())), "walls {ctx}");

            for before in [max_dist, max_dist - 0.1, rng.f(-1.0, max_dist.clamp(0.0, 200.0))] {
                assert_eq!(
                    reference::ray_hits_wall_before(&m, o, d, max_dist, before),
                    m.ray_hits_wall_before(o, d, max_dist, before),
                    "before={before} {ctx}"
                );
            }

            let old = reference::ray_test_props_with_normal(&m, o, d, max_dist);
            assert_eq!(key2(old), key2(m.ray_test_props_with_normal(o, d, max_dist)), "props n {ctx}");
            prop_hits += old.is_some() as usize;

            let old = reference::ray_test_walls_with_normal(&m, o, d, max_dist);
            assert_eq!(key3(old), key3(m.ray_test_walls_with_normal(o, d, max_dist)), "walls n {ctx}");

            let old = reference::ray_test_floor_ceiling_with_normal(&m, o, d, max_dist);
            let new = m.ray_test_floor_ceiling_with_normal(o, d, max_dist);
            assert_eq!(key2(old), key2(new), "floor/ceiling {ctx}");

            let old = reference::ray_test_ceiling_lights_with_normal(&m, o, d, max_dist, &destroyed);
            let new = m.ray_test_ceiling_lights_with_normal(o, d, max_dist, &destroyed);
            assert_eq!(key3(old), key3(new), "lights {ctx}");
            light_hits += old.is_some() as usize;

            let old = reference::ray_test_surfaces(&m, o, d, max_dist, &destroyed);
            let new = m.ray_test_surfaces(o, d, max_dist, &destroyed);
            let key = |h: Option<super::SurfaceHit>| h.map(|(d, n, w, l)| (d.to_bits(), bits(n), w, l));
            assert_eq!(key(old), key(new), "surfaces {ctx}");
            if let Some((_, _, w, l)) = old {
                surface_lights += l.is_some() as usize;
                surface_props += (w.is_none() && l.is_none()) as usize;
            }
        }
    }
    // The random rays must actually exercise props and lights.
    assert!(prop_hits > 5_000, "only {prop_hits} prop hits");
    assert!(light_hits > 2_000, "only {light_hits} light hits");
    assert!(surface_lights > 1_000, "only {surface_lights} surface light picks");
    assert!(surface_props > 10_000, "only {surface_props} prop/floor/ceiling picks");
}
