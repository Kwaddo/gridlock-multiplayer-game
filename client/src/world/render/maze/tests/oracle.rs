//! Independent judge of which faces of the old per-cube path can never be
//! seen, from the maze geometry alone (not from the baker's rules).
//!
//! Viewers stand in open cells between the floor (y = 0) and the wall tops.
//! A face can never be seen when, just outside it, there is only a wall's
//! cube, the outside of the maze or the space under the floor (which a
//! viewer cannot look into, since every cell covers y = 0 with a floor top
//! or a wall); when it faces away from every viewer height; or when its
//! whole cube sits inside a wall.

use macroquad::prelude::*;

use crate::types::maze::Maze;

use super::super::cells::{bake_chunk, chunk_grid};
use super::super::lighting::Lighting;
use super::quads;
use super::reference::{draw_cube_faces, Cube};

const NORMALS: [Vec3; 6] = [Vec3::Z, Vec3::NEG_Z, Vec3::Y, Vec3::NEG_Y, Vec3::X, Vec3::NEG_X];
const TOP: usize = 2;
const BOTTOM: usize = 3;

fn wall_span(m: &Maze) -> (f32, f32) {
    let centre = m.wall_height * 0.5 - 0.01;
    let half = (m.wall_height + 0.02) / 2.0;
    (centre - half, centre + half)
}

fn wall_cell(m: &Maze, p: Vec3) -> Option<(usize, usize)> {
    let (x, y) = ((p.x / m.cell_size).floor(), (p.z / m.cell_size).floor());
    if x < 0.0 || y < 0.0 || x >= m.width as f32 || y >= m.height as f32 {
        return None;
    }
    let (x, y) = (x as usize, y as usize);
    (m.cells[m.idx(x, y)] == 0).then_some((x, y))
}

fn inside_wall_cube(m: &Maze, p: Vec3) -> bool {
    let (bottom, top) = wall_span(m);
    wall_cell(m, p).is_some() && p.y > bottom && p.y < top
}

fn outside_maze(m: &Maze, p: Vec3) -> bool {
    p.x < 0.0 || p.z < 0.0 || p.x >= m.width as f32 * m.cell_size || p.z >= m.height as f32 * m.cell_size
}

pub fn face_hidden(m: &Maze, cube: &Cube, face: usize) -> bool {
    let corners = draw_cube_faces(cube)[face].corners.map(Vec3::from_array);
    let centre = (corners[0] + corners[1] + corners[2] + corners[3]) / 4.0;

    // The whole cube inside one wall cube.
    let (lo, hi) = (cube.pos - cube.size / 2.0, cube.pos + cube.size / 2.0);
    let (bottom, top) = wall_span(m);
    if let Some((x, y)) = wall_cell(m, cube.pos) {
        let (cx0, cz0) = (x as f32 * m.cell_size, y as f32 * m.cell_size);
        if lo.x > cx0 && hi.x < cx0 + m.cell_size && lo.z > cz0 && hi.z < cz0 + m.cell_size && lo.y > bottom && hi.y < top {
            return true;
        }
    }
    if face == BOTTOM && centre.y <= 0.0 {
        return true; // facing down from at or below the floor
    }
    if face == TOP && centre.y > m.wall_height {
        return true; // facing up from above every viewer
    }
    let p = centre + NORMALS[face] * 1e-3;
    outside_maze(m, p) || p.y < 0.0 || inside_wall_cube(m, p)
}

/// The premise of the under-floor rule: each open cell keeps its floor top
/// at y = 0 and each wall cube reaches below it.
pub fn assert_floor_plane_covered(m: &Maze, lighting: &Lighting) {
    let (bottom, top) = wall_span(m);
    assert!(bottom < 0.0 && top > 0.0);
    let (chunks_x, chunks_y) = chunk_grid(m);
    let baked: Vec<_> = (0..chunks_x * chunks_y)
        .flat_map(|c| quads(&bake_chunk(m, lighting, c % chunks_x, c / chunks_x)))
        .map(|(_, f)| f.corners)
        .collect();
    for y in 0..m.height as usize {
        for x in 0..m.width as usize {
            if m.cells[m.idx(x, y)] == 0 {
                continue;
            }
            let center = vec3((x as f32 + 0.5) * m.cell_size, 0.0, (y as f32 + 0.5) * m.cell_size);
            let floor = Cube {
                pos: center + vec3(0.0, -0.025, 0.0),
                size: vec3(m.cell_size * 1.000, 0.05, m.cell_size * 1.000),
                color: WHITE,
            };
            let top_face = draw_cube_faces(&floor)[TOP].corners;
            assert!(baked.contains(&top_face), "open cell ({x}, {y}) lost its floor top");
            assert!(top_face.iter().all(|c| c[1].abs() < 1e-6));
        }
    }
}
