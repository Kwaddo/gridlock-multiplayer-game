//! The maze ray tests as they were before the per-ray reciprocal, the height
//! band rejects, the neighbourhood dedup and the shrinking surface reach,
//! kept verbatim (on top of the original `ray_hit_aabb`) as the oracle the
//! faster versions must match bit for bit.

use macroquad::prelude::*;
use std::collections::HashSet;

use super::super::Maze;
use super::grid_walk;
use crate::weapons::combat::reference::{ray_hit_aabb, ray_hit_aabb_with_normal};

/// The original `ray_test_props`.
pub(super) fn ray_test_props(m: &Maze, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<f32> {
    let mut closest_dist: Option<f32> = None;

    for prop in &m.props {
        if let Some((center_offset, half_extents)) = prop.kind.bullet_hitbox() {
            let prop_world_pos = m.cell_to_world(prop.x as i32, prop.y as i32);
            let box_center = prop_world_pos + center_offset;

            if let Some(dist) = ray_hit_aabb(origin, dir, box_center, half_extents, max_dist) {
                closest_dist = match closest_dist {
                    None => Some(dist),
                    Some(d) if dist < d => Some(dist),
                    Some(d) => Some(d),
                };
            }
        }
    }

    closest_dist
}

/// The original `ray_test_walls`.
pub(super) fn ray_test_walls(m: &Maze, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<((u8, u8), f32)> {
    closest_wall_hit(m, origin, dir, max_dist, |center, half| {
        ray_hit_aabb(origin, dir, center, half, max_dist).map(|d| (d, ()))
    })
    .map(|(cell, d, ())| (cell, d))
}

/// The original `ray_hits_wall_before`.
pub(super) fn ray_hits_wall_before(m: &Maze, origin: Vec3, dir: Vec3, max_dist: f32, before: f32) -> bool {
    let (y0, y1) = match m.wall_slab_interval(origin, dir, max_dist) {
        Some(range) => range,
        None => return false,
    };
    let mut blocked = false;
    m.walk_cells(origin, dir, y0, y1, |cx, cy, t_enter| {
        if t_enter > before + grid_walk::walk_slack(before) {
            return false;
        }
        for (x, y) in m.neighbourhood(cx, cy) {
            if m.cells[m.idx(x as usize, y as usize)] != 0 {
                continue;
            }
            let (center, half) = m.wall_box(x, y);
            if let Some(d) = ray_hit_aabb(origin, dir, center, half, max_dist) {
                if d < before {
                    blocked = true;
                    return false;
                }
            }
        }
        true
    });
    blocked
}

/// The original `closest_wall_hit`.
fn closest_wall_hit<T: Copy>(
    m: &Maze,
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
    test: impl Fn(Vec3, Vec3) -> Option<(f32, T)>,
) -> Option<((u8, u8), f32, T)> {
    let (y0, y1) = m.wall_slab_interval(origin, dir, max_dist)?;
    let mut closest_hit: Option<((u8, u8), f32, T)> = None;

    m.walk_cells(origin, dir, y0, y1, |cx, cy, t_enter| {
        if let Some((_, d, _)) = closest_hit {
            if t_enter > d + grid_walk::walk_slack(d) {
                return false;
            }
        }
        for (x, y) in m.neighbourhood(cx, cy) {
            if m.cells[m.idx(x as usize, y as usize)] != 0 {
                continue;
            }
            let (center, half) = m.wall_box(x, y);
            if let Some((dist, extra)) = test(center, half) {
                let best = closest_hit.map(|(c, d, _)| (c, d));
                if grid_walk::is_better(dist, (x, y), best) {
                    closest_hit = Some(((x, y), dist, extra));
                }
            }
        }
        true
    });

    closest_hit
}

/// The original `ray_test_props_with_normal`.
pub(super) fn ray_test_props_with_normal(m: &Maze, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<(f32, Vec3)> {
    let mut closest: Option<(f32, Vec3)> = None;

    for prop in &m.props {
        if let Some((center_offset, half_extents)) = prop.kind.bullet_hitbox() {
            let prop_world_pos = m.cell_to_world(prop.x as i32, prop.y as i32);
            let box_center = prop_world_pos + center_offset;

            if let Some((dist, normal)) = ray_hit_aabb_with_normal(origin, dir, box_center, half_extents, max_dist) {
                if closest.map_or(true, |(d, _)| dist < d) {
                    closest = Some((dist, normal));
                }
            }
        }
    }

    closest
}

/// The original `ray_test_walls_with_normal`.
pub(super) fn ray_test_walls_with_normal(
    m: &Maze,
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
) -> Option<((u8, u8), f32, Vec3)> {
    closest_wall_hit(m, origin, dir, max_dist, |center, half| {
        ray_hit_aabb_with_normal(origin, dir, center, half, max_dist)
    })
}

/// The original `ray_test_floor_ceiling_with_normal`.
pub(super) fn ray_test_floor_ceiling_with_normal(
    m: &Maze,
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
) -> Option<(f32, Vec3)> {
    let map_w = m.width as f32 * m.cell_size;
    let map_h = m.height as f32 * m.cell_size;

    let mut best: Option<(f32, Vec3)> = None;

    let floor_center = vec3(map_w * 0.5, -0.025, map_h * 0.5);
    let floor_half = vec3(map_w * 0.5, 0.025, map_h * 0.5);
    if let Some((d, n)) = ray_hit_aabb_with_normal(origin, dir, floor_center, floor_half, max_dist) {
        best = Some((d, n));
    }

    let roof_clearance = 0.05;
    let roof_bottom_y = m.wall_height + roof_clearance;
    let panel_inset = 0.03;
    let panel_thickness = 0.03;
    let panel_drop = 0.018;
    let panel_y = roof_bottom_y - panel_drop;
    let panel_half_xz = m.cell_size * (1.0 - panel_inset) * 0.5;
    let panel_half_y = panel_thickness * 0.5;

    let panel_half = vec3(panel_half_xz, panel_half_y, panel_half_xz);
    let mm = grid_walk::CLIP_MARGIN;
    let panel_span = grid_walk::slab_interval(
        origin.y,
        dir.y,
        panel_y - panel_half_y - mm,
        panel_y + panel_half_y + mm,
    );
    if let Some((a, b)) = panel_span {
        let mut panel_best: Option<((u8, u8), f32, Vec3)> = None;
        m.walk_cells(origin, dir, a.max(0.0), b.min(max_dist), |cx, cy, t_enter| {
            let bound = match (panel_best, best) {
                (Some((_, pd, _)), _) => Some(pd),
                (None, Some((fd, _))) => Some(fd),
                (None, None) => None,
            };
            if bound.map_or(false, |d| t_enter > d + grid_walk::walk_slack(d)) {
                return false;
            }
            for (x, y) in m.neighbourhood(cx, cy) {
                let idx = m.idx(x as usize, y as usize);
                if m.cells[idx] == 0 {
                    continue;
                }
                let center = vec3(
                    (x as f32 + 0.5) * m.cell_size,
                    panel_y,
                    (y as f32 + 0.5) * m.cell_size,
                );
                if let Some((d, n)) =
                    ray_hit_aabb_with_normal(origin, dir, center, panel_half, max_dist)
                {
                    let pb = panel_best.map(|(c, pd, _)| (c, pd));
                    if grid_walk::is_better(d, (x, y), pb) {
                        panel_best = Some(((x, y), d, n));
                    }
                }
            }
            true
        });
        if let Some((_, d, n)) = panel_best {
            if best.map_or(true, |(b, _)| d < b) {
                best = Some((d, n));
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

/// The original `ray_test_ceiling_lights_with_normal`.
pub(super) fn ray_test_ceiling_lights_with_normal(
    m: &Maze,
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
    destroyed_lights: &HashSet<(u8, u8)>,
) -> Option<((u8, u8), f32, Vec3)> {
    let roof_clearance = 0.05;
    let roof_bottom_y = m.wall_height + roof_clearance;

    let mut best: Option<((u8, u8), f32, Vec3)> = None;

    for info in &m.ceiling_lights {
        if destroyed_lights.contains(&(info.x, info.y)) {
            continue;
        }
        let light_pos = m.cell_to_world(info.x as i32, info.y as i32)
            + vec3(0.0, roof_bottom_y - 0.16, 0.0);
        let housing_center = light_pos + vec3(0.0, 0.02, 0.0);
        let housing_half = vec3(0.22, 0.05, 0.22);

        if let Some((d, n)) = ray_hit_aabb_with_normal(origin, dir, housing_center, housing_half, max_dist) {
            if best.map_or(true, |(_, bd, _)| d < bd) {
                best = Some(((info.x, info.y), d, n));
            }
        }
    }

    best
}

/// The surface pick `fire_hitscan` did before `Maze::ray_test_surfaces`:
/// all four tests at full range, then the closest, earlier on ties.
pub(super) fn ray_test_surfaces(
    m: &Maze,
    eye: Vec3,
    fwd: Vec3,
    world_range: f32,
    destroyed_lights: &HashSet<(u8, u8)>,
) -> Option<(f32, Vec3, Option<(u8, u8)>, Option<(u8, u8)>)> {
    let wall_hit = ray_test_walls_with_normal(m, eye, fwd, world_range);
    let prop_hit = ray_test_props_with_normal(m, eye, fwd, world_range);
    let fc_hit = ray_test_floor_ceiling_with_normal(m, eye, fwd, world_range);
    let light_hit = ray_test_ceiling_lights_with_normal(m, eye, fwd, world_range, destroyed_lights);

    let candidates: [Option<(f32, Vec3, Option<(u8, u8)>, Option<(u8, u8)>)>; 4] = [
        wall_hit.map(|((wx, wy), d, n)| (d, n, Some((wx, wy)), None)),
        prop_hit.map(|(d, n)| (d, n, None, None)),
        fc_hit.map(|(d, n)| (d, n, None, None)),
        light_hit.map(|((lx, ly), d, n)| (d, n, None, Some((lx, ly)))),
    ];

    candidates.into_iter().flatten().fold(
        None,
        |best: Option<(f32, Vec3, Option<(u8, u8)>, Option<(u8, u8)>)>, c| match best {
            Some(b) if !(c.0 < b.0) => Some(b),
            _ => Some(c),
        },
    )
}
