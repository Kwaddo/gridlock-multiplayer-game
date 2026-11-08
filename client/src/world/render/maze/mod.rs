//! The maze is baked once into static meshes (cells in chunks, the roof,
//! pipes and light fixtures, props in runs), each cube coloured by the
//! ceiling lights at its centre exactly as the per-frame path did. Meshes
//! live on the GPU; each frame only culls them and issues one draw call per
//! visible mesh. Only cells are culled by distance; the roof, pipes,
//! fixtures and props draw at any distance, as before. A wall breaking re-bakes the chunks around it; a light
//! going out re-bakes everything, since every light reaches every cube, a
//! few pieces per frame (nearest the light first) so it never hitches.

mod cache;
mod cells;
mod constants;
mod cull;
mod globals;
mod gpu;
mod lighting;
mod mesh;
#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::collections::HashSet;

use macroquad::prelude::*;
use macroquad::window::get_internal_gl;

use crate::types::maze::Maze;

use super::props::draw_prop_hitboxes;
use cache::MazeMeshes;
use constants::{MIN_RENDER_CELLS, PLACEHOLDER_COLOR, RENDER_SLACK_CELLS};
use cull::{DistanceCull, Frustum};
pub(in crate::world::render) use mesh::MeshBuilder;

thread_local! {
    static MESHES: RefCell<Option<MazeMeshes>> = const { RefCell::new(None) };
}

/// Put macroquad's batcher in a known state (triangles, depth test, no
/// texture), as debug draws may have left it otherwise.
fn reset_batch_state() {
    let gl = unsafe { get_internal_gl() };
    gl.quad_gl.draw_mode(DrawMode::Triangles);
    gl.quad_gl.depth_test(true);
    gl.quad_gl.texture(None);
}

pub fn draw_maze(
    maze: Option<&Maze>,
    viewer_pos: Vec3,
    render_distance: f32,
    show_hitboxes: bool,
    destroyed_lights: &HashSet<(u8, u8)>,
) {
    let Some(m) = maze else {
        draw_cube(vec3(0.0, -1.0, 0.0), vec3(20.0, 0.1, 20.0), None, PLACEHOLDER_COLOR);
        return;
    };
    reset_batch_state();

    let effective_render = render_distance.max(m.cell_size * MIN_RENDER_CELLS);
    let reach = effective_render + m.cell_size * RENDER_SLACK_CELLS;
    let dist = DistanceCull { viewer: vec2(viewer_pos.x, viewer_pos.z), max_dist_sq: reach * reach };
    let frustum = {
        let gl = unsafe { get_internal_gl() };
        Frustum::new(gl.quad_gl.get_projection_matrix())
    };

    MESHES.with(|cell| {
        let mut slot = cell.borrow_mut();
        match slot.as_mut() {
            Some(meshes) => meshes.sync(m, destroyed_lights),
            None => *slot = Some(MazeMeshes::new(m, destroyed_lights)),
        }
        if let Some(meshes) = slot.as_ref() {
            gpu::draw(&meshes.visible(&frustum, &dist), &dist);
        }
    });

    if show_hitboxes {
        draw_prop_hitboxes(m);
    }
    reset_batch_state();
}
