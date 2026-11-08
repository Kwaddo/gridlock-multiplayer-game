//! Checks the grid-walk ray tests against the original brute-force scans of
//! every cell, kept here as the oracle.

use macroquad::prelude::*;
use std::collections::{HashMap, HashSet};

use super::super::Maze;
use crate::weapons::combat::{ray_hit_aabb, ray_hit_aabb_with_normal};

/// The original `ray_test_walls`: AABB-test every wall cell.
fn brute_walls(m: &Maze, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<((u8, u8), f32)> {
    let mut closest_hit: Option<((u8, u8), f32)> = None;
    for y in 0..m.height {
        for x in 0..m.width {
            if m.cells[m.idx(x as usize, y as usize)] == 0 {
                let wall_center = m.cell_to_world(x as i32, y as i32);
                let box_center = wall_center + vec3(0.0, m.wall_height * 0.5, 0.0);
                let half_extents = vec3(m.cell_size * 0.5, m.wall_height * 0.5, m.cell_size * 0.5);
                if let Some(dist) = ray_hit_aabb(origin, dir, box_center, half_extents, max_dist) {
                    if closest_hit.map_or(true, |(_, d)| dist < d) {
                        closest_hit = Some(((x, y), dist));
                    }
                }
            }
        }
    }
    closest_hit
}

/// The original `ray_test_walls_with_normal`.
fn brute_walls_n(m: &Maze, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<((u8, u8), f32, Vec3)> {
    let mut closest_hit: Option<((u8, u8), f32, Vec3)> = None;
    for y in 0..m.height {
        for x in 0..m.width {
            if m.cells[m.idx(x as usize, y as usize)] == 0 {
                let wall_center = m.cell_to_world(x as i32, y as i32);
                let box_center = wall_center + vec3(0.0, m.wall_height * 0.5, 0.0);
                let half_extents = vec3(m.cell_size * 0.5, m.wall_height * 0.5, m.cell_size * 0.5);
                if let Some((dist, normal)) =
                    ray_hit_aabb_with_normal(origin, dir, box_center, half_extents, max_dist)
                {
                    if closest_hit.map_or(true, |(_, d, _)| dist < d) {
                        closest_hit = Some(((x, y), dist, normal));
                    }
                }
            }
        }
    }
    closest_hit
}

/// The original `ray_test_floor_ceiling_with_normal`.
fn brute_floor_ceiling(m: &Maze, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<(f32, Vec3)> {
    let map_w = m.width as f32 * m.cell_size;
    let map_h = m.height as f32 * m.cell_size;
    let mut best: Option<(f32, Vec3)> = None;
    let floor_center = vec3(map_w * 0.5, -0.025, map_h * 0.5);
    let floor_half = vec3(map_w * 0.5, 0.025, map_h * 0.5);
    if let Some((d, n)) = ray_hit_aabb_with_normal(origin, dir, floor_center, floor_half, max_dist) {
        best = Some((d, n));
    }
    let roof_bottom_y = m.wall_height + 0.05;
    let panel_y = roof_bottom_y - 0.018;
    let panel_half_xz = m.cell_size * (1.0 - 0.03) * 0.5;
    let panel_half_y = 0.03 * 0.5;
    for y in 0..m.height {
        for x in 0..m.width {
            if m.cells[m.idx(x as usize, y as usize)] == 0 {
                continue;
            }
            let center = vec3((x as f32 + 0.5) * m.cell_size, panel_y, (y as f32 + 0.5) * m.cell_size);
            if let Some((d, n)) = ray_hit_aabb_with_normal(
                origin, dir, center,
                vec3(panel_half_xz, panel_half_y, panel_half_xz),
                max_dist,
            ) {
                if best.map_or(true, |(b, _)| d < b) {
                    best = Some((d, n));
                }
            }
        }
    }
    let roof_thickness = 0.06;
    let roof_top_y = roof_bottom_y + roof_thickness;
    let roof_center = vec3(map_w * 0.5, (roof_bottom_y + roof_top_y) * 0.5, map_h * 0.5);
    let roof_half = vec3(map_w * 0.505, roof_thickness * 0.5, map_h * 0.505);
    if let Some((d, n)) = ray_hit_aabb_with_normal(origin, dir, roof_center, roof_half, max_dist) {
        if best.map_or(true, |(b, _)| d < b) {
            best = Some((d, n));
        }
    }
    best
}

/// Small deterministic xorshift so the test needs no RNG setup.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn f(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (self.next() >> 40) as f32 / (1u64 << 24) as f32 * (hi - lo)
    }
    fn int(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next() % (hi - lo + 1) as u64) as i32
    }
}

fn random_maze(rng: &mut Rng) -> Maze {
    let width = rng.int(3, 43) as u8;
    let height = rng.int(3, 43) as u8;
    let wall_chance = rng.f(0.05, 0.7);
    let cells = (0..width as usize * height as usize)
        .map(|_| if rng.f(0.0, 1.0) < wall_chance { 0 } else { 1 })
        .collect();
    Maze {
        width,
        height,
        cells,
        cell_size: 1.8,
        wall_height: 2.4,
        teleporter: (1, 1),
        rooms: Vec::new(),
        props: Vec::new(),
        ceiling_lights: Vec::new(),
        breakable_walls: HashMap::new(),
        broken_wall_scars: HashSet::new(),
    }
}

/// A grid line or point on it, so rays start on faces and aim at corners.
fn grid_coord(rng: &mut Rng, cells: u8, cs: f32) -> f32 {
    rng.int(-1, cells as i32 + 1) as f32 * cs
}

fn random_ray(rng: &mut Rng, m: &Maze) -> (Vec3, Vec3, f32) {
    let cs = m.cell_size;
    let (w, h) = (m.width as f32 * cs, m.height as f32 * cs);
    let mut origin = vec3(rng.f(-3.0, w + 3.0), rng.f(-0.5, 3.0), rng.f(-3.0, h + 3.0));
    // Snap the origin onto grid lines some of the time.
    match rng.int(0, 5) {
        0 => origin.x = grid_coord(rng, m.width, cs),
        1 => origin.z = grid_coord(rng, m.height, cs),
        2 => {
            origin.x = grid_coord(rng, m.width, cs);
            origin.z = grid_coord(rng, m.height, cs);
        }
        _ => {}
    }
    let mut dir = match rng.int(0, 6) {
        // Aim exactly at a grid corner (grazing two or four cells).
        0 | 1 => {
            let target = vec3(
                grid_coord(rng, m.width, cs),
                rng.f(-0.2, 2.6),
                grid_coord(rng, m.height, cs),
            );
            target - origin
        }
        // Axis-aligned in XZ, optionally level.
        2 => vec3(rng.f(-1.0, 1.0), 0.0, 0.0),
        3 => vec3(0.0, if rng.int(0, 1) == 0 { 0.0 } else { rng.f(-0.3, 0.3) }, rng.f(-1.0, 1.0)),
        // Straight up or down.
        4 => vec3(0.0, rng.f(-1.0, 1.0), 0.0),
        _ => vec3(rng.f(-1.0, 1.0), rng.f(-0.6, 0.6), rng.f(-1.0, 1.0)),
    };
    // Exact diagonals hit corners for every cell they cross.
    if rng.int(0, 9) == 0 {
        dir = vec3(1.0f32.copysign(dir.x), dir.y, 1.0f32.copysign(dir.z));
    }
    if dir.length() < 1e-6 {
        dir = vec3(1.0, 0.0, 0.0);
    }
    let dir = dir.normalize();
    let max_dist = match rng.int(0, 4) {
        0 => rng.f(0.0, 2.0),
        1 => 1000.0,
        _ => rng.f(0.0, 120.0),
    };
    (origin, dir, max_dist)
}

fn assert_vec_close(a: Vec3, b: Vec3, ctx: &str) {
    assert!((a - b).length() <= 1e-4, "normal {a:?} != {b:?} ({ctx})");
}

#[test]
fn grid_walk_matches_brute_force() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut hits = 0usize;
    for _ in 0..60 {
        let m = random_maze(&mut rng);
        for _ in 0..2_000 {
            let (o, d, max_dist) = random_ray(&mut rng, &m);
            let ctx = format!("o={o:?} d={d:?} max={max_dist} maze={}x{}", m.width, m.height);

            let fast = m.ray_test_walls(o, d, max_dist);
            let slow = brute_walls(&m, o, d, max_dist);
            match (fast, slow) {
                (None, None) => {}
                (Some((fc, fd)), Some((sc, sd))) => {
                    hits += 1;
                    assert_eq!(fc, sc, "wall cell ({ctx})");
                    assert!((fd - sd).abs() <= 1e-4, "wall dist {fd} != {sd} ({ctx})");
                }
                _ => panic!("wall hit {fast:?} != {slow:?} ({ctx})"),
            }

            let fast_n = m.ray_test_walls_with_normal(o, d, max_dist);
            let slow_n = brute_walls_n(&m, o, d, max_dist);
            match (fast_n, slow_n) {
                (None, None) => {}
                (Some((fc, fd, fnrm)), Some((sc, sd, snrm))) => {
                    assert_eq!(fc, sc, "wall cell w/ normal ({ctx})");
                    assert!((fd - sd).abs() <= 1e-4, "wall dist w/ normal {fd} != {sd} ({ctx})");
                    assert_vec_close(fnrm, snrm, &ctx);
                }
                _ => panic!("wall hit w/ normal {fast_n:?} != {slow_n:?} ({ctx})"),
            }

            // Occlusion early-exit, at several cut-offs around the hit.
            for before in [max_dist - 0.1, max_dist, rng.f(-1.0, max_dist.min(200.0))] {
                let expected = slow.map_or(false, |(_, wd)| wd < before);
                assert_eq!(
                    m.ray_hits_wall_before(o, d, max_dist, before),
                    expected,
                    "occlusion before={before} ({ctx})"
                );
            }

            let fast_fc = m.ray_test_floor_ceiling_with_normal(o, d, max_dist);
            let slow_fc = brute_floor_ceiling(&m, o, d, max_dist);
            match (fast_fc, slow_fc) {
                (None, None) => {}
                (Some((fd, fnrm)), Some((sd, snrm))) => {
                    assert!((fd - sd).abs() <= 1e-4, "ceiling dist {fd} != {sd} ({ctx})");
                    assert_vec_close(fnrm, snrm, &ctx);
                }
                _ => panic!("floor/ceiling {fast_fc:?} != {slow_fc:?} ({ctx})"),
            }
        }
    }
    // The random rays must actually exercise wall hits.
    assert!(hits > 10_000, "only {hits} wall hits");
}

#[test]
fn ray_starting_inside_wall_hits_at_zero() {
    let mut rng = Rng(7);
    let mut m = random_maze(&mut rng);
    m.cells.iter_mut().for_each(|c| *c = 1);
    let i = m.idx(1, 1);
    m.cells[i] = 0;
    let o = m.cell_to_world(1, 1) + vec3(0.0, 1.0, 0.0);
    let hit = m.ray_test_walls(o, vec3(1.0, 0.0, 0.0), 50.0);
    assert_eq!(hit, brute_walls(&m, o, vec3(1.0, 0.0, 0.0), 50.0));
    assert_eq!(hit.map(|(c, d)| (c, d)), Some(((1, 1), 0.0)));
}
