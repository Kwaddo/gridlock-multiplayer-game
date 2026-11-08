//! Floor, wall and ceiling-panel geometry of the maze cells, baked per chunk
//! with faces that can never be seen left out.

use macroquad::prelude::*;

use crate::types::maze::Maze;

use super::super::utils::tint_color;
use super::constants::*;
use super::lighting::Lighting;
use super::mesh::{face, MeshBuilder, MeshData};

/// Index of the chunk holding cell (`x`, `y`) in a grid `chunks_x` wide.
pub(super) fn chunk_of(x: usize, y: usize, chunks_x: usize) -> usize {
    (y / CHUNK_CELLS) * chunks_x + x / CHUNK_CELLS
}

pub(super) fn chunk_grid(m: &Maze) -> (usize, usize) {
    (
        (m.width as usize).div_ceil(CHUNK_CELLS),
        (m.height as usize).div_ceil(CHUNK_CELLS),
    )
}

fn inside(m: &Maze, x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && x < m.width as i32 && y < m.height as i32
}

/// An open cell (what a viewer can stand in).
fn open(m: &Maze, x: i32, y: i32) -> bool {
    inside(m, x, y) && m.cells[m.idx(x as usize, y as usize)] != 0
}

/// The side faces of a cube in cell (`x`, `y`) whose neighbour passes `keep`.
fn sides(x: i32, y: i32, keep: impl Fn(i32, i32) -> bool) -> u8 {
    [(0, 1, face::FRONT), (0, -1, face::BACK), (1, 0, face::RIGHT), (-1, 0, face::LEFT)]
        .iter()
        .filter(|(dx, dy, _)| keep(x + dx, y + dy))
        .fold(0, |acc, (_, _, f)| acc | f)
}

/// The breakable-wall brace sits inside its wall's cube at the usual cell
/// size, so it is never seen; only bake it when it would poke out.
fn brace_enclosed(m: &Maze) -> bool {
    let cs = m.cell_size;
    let half = cs * 1.00 / 2.0;
    let z_max = cs * 0.49 + 0.02 / 2.0;
    let x_max = cs * 0.62 / 2.0;
    let y_min = m.wall_height * 0.62 - 0.045 / 2.0;
    let y_max = m.wall_height * 0.62 + 0.045 / 2.0;
    let wall_bottom = m.wall_height * 0.5 - 0.01 - (m.wall_height + 0.02) / 2.0;
    let wall_top = m.wall_height * 0.5 - 0.01 + (m.wall_height + 0.02) / 2.0;
    z_max < half && x_max < half && y_min > wall_bottom && y_max < wall_top
}

/// Bake the cells of chunk (`chunk_x`, `chunk_y`), row by row.
pub(super) fn bake_chunk(m: &Maze, lighting: &Lighting, chunk_x: usize, chunk_y: usize) -> Vec<MeshData> {
    let mut out = MeshBuilder::new(true);
    let braces = !brace_enclosed(m);
    let y_end = ((chunk_y + 1) * CHUNK_CELLS).min(m.height as usize);
    let x_end = ((chunk_x + 1) * CHUNK_CELLS).min(m.width as usize);
    for y in chunk_y * CHUNK_CELLS..y_end {
        for x in chunk_x * CHUNK_CELLS..x_end {
            bake_cell(&mut out, m, lighting, x, y, braces);
        }
    }
    out.finish()
}

/// One cell: what the old per-frame loop drew for it, minus faces that can
/// never be seen.
///
/// Left out: every bottom face (below the floor or the ceiling panels'
/// view), panel tops (above every viewer), wall and floor sides facing the
/// outside of the maze, floor sides facing a wall, and the brace inside a
/// breakable wall. Kept although hidden: neighbouring cubes do not share
/// their edge vertices exactly (cell centres times 1.8 round differently),
/// so the surfaces have sub-pixel cracks, and what a crack pixel shows is
/// one of these faces: the wall sides between two walls, the floor tops
/// under walls and the floor sides between open cells. Dropping them
/// changed a few such pixels per frame (some to the background colour).
pub(super) fn bake_cell(out: &mut MeshBuilder, m: &Maze, lighting: &Lighting, x: usize, y: usize, braces: bool) {
    let center = vec3((x as f32 + 0.5) * m.cell_size, 0.0, (y as f32 + 0.5) * m.cell_size);
    out.begin_item(vec2(center.x, center.z));
    let roof_bottom_y = m.wall_height + ROOF_CLEARANCE;
    let (xi, yi) = (x as i32, y as i32);

    let is_dark_tile = ((x + y) & 1) == 0;
    let is_wall = m.cells[m.idx(x, y)] == 0;

    // Floor slab: its top, and its sides toward open cells.
    let tile_base = if is_dark_tile { FLOOR_DARK } else { FLOOR_LIGHT };
    let tile_tint = tint_color(tile_base, xi, yi, 0x11, 0.018, 0.008);
    let floor_faces = face::TOP | sides(xi, yi, |nx, ny| open(m, nx, ny));
    out.cube_faces(
        center + vec3(0.0, -0.025, 0.0),
        vec3(m.cell_size * 1.000, 0.05, m.cell_size * 1.000),
        lighting.lit(tile_tint, center),
        floor_faces,
    );

    if !is_wall {
        // Ceiling panel: seen from below and the sides, never from above.
        let panel_base = if is_dark_tile { CEILING_PANEL_DARK } else { CEILING_PANEL_LIGHT };
        let panel_tint = tint_color(panel_base, xi, yi, 0x51, 0.016, 0.006);
        let panel_pos = center + vec3(0.0, roof_bottom_y - PANEL_DROP, 0.0);
        out.cube_faces(
            panel_pos,
            vec3(m.cell_size * (1.0 - PANEL_INSET), PANEL_THICKNESS, m.cell_size * (1.0 - PANEL_INSET)),
            lighting.lit(panel_tint, panel_pos),
            face::ALL & !face::TOP,
        );
        return;
    }

    let (cx_u8, cy_u8) = (x as u8, y as u8);
    let breakable = m.is_breakable(cx_u8, cy_u8);
    let base = if breakable { WALL_BREAKABLE } else { WALL_COLOR };
    let wall_tint = tint_color(base, xi, yi, 0x33, 0.026, 0.008);
    let wall_pos = center + vec3(0.0, m.wall_height * 0.5 - 0.01, 0.0);
    // All but the bottom and the sides facing the outside.
    out.cube_faces(
        wall_pos,
        vec3(m.cell_size * 1.00, m.wall_height + 0.02, m.cell_size * 1.00),
        lighting.lit(wall_tint, wall_pos),
        face::TOP | sides(xi, yi, |nx, ny| inside(m, nx, ny)),
    );

    if breakable && braces {
        out.cube(
            center + vec3(0.0, m.wall_height * 0.62, m.cell_size * 0.49),
            vec3(m.cell_size * 0.62, 0.045, 0.02),
            lighting.lit(BRACE_COLOR, center + vec3(0.0, m.wall_height * 0.62, 0.0)),
        );
    }
}
