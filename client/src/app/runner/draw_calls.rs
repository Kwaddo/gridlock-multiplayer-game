//! How much geometry one macroquad draw call may hold.
//!
//! Every draw-call slot macroquad creates keeps a stream vertex buffer of
//! `MAX_VERTICES` * 40 B and an index buffer of `MAX_INDICES` * 2 B for the
//! rest of the run (quad_gl.rs `draw`, 0.4.14). The old 200_000 / 300_000,
//! sized for whole models through `draw_mesh`, cost 8.6 MB per slot; these
//! cost 0.7 MB.
//!
//! One `geometry()` call (one `draw_mesh`) over either limit is cut short,
//! dropping triangles, and one that reaches it logs a warning. The largest
//! single submissions in the client:
//! - loaded models (static meshes, animated parts, and the tinted and opaque
//!   copies drawn from them): split at load to at most 10_000 vertices and
//!   4_998 indices (`types::mesh_split`); static weapons and skinned bodies
//!   mostly skip the batcher now (`static_gpu`, `animated_model::gpu`).
//! - the maze's CPU fallback, one item per call: at most 3_288 vertices and
//!   4_932 indices (a prop), 1_056 / 1_584 for a pipe row on a 255-wide maze,
//!   measured over every bake of a 255x255 maze full of every prop.
//! - the login scanlines: one quad per 4 px of window height, so H vertices
//!   and 1.5 H indices for a window H logical px tall (16_383 px fits).
//! - everything else (HUD discs and rings, the colour picker, login
//!   particles, damage arcs, tracers, bullet holes, text, spheres): under
//!   5_000 of either.
//!
//! Batched calls must also stay at or under 65_536 vertices: macroquad adds
//! the call's vertex count to each u16 index, so a bigger batch wraps them.

use macroquad::window::gl_set_drawcall_buffer_capacity;

/// Room for the largest loaded mesh piece (10_000) with headroom.
const MAX_VERTICES: usize = 16_384;
/// 1.5 indices per scanline vertex, so both limits allow the same window.
const MAX_INDICES: usize = 24_576;

pub(super) fn set_capacity() {
    gl_set_drawcall_buffer_capacity(MAX_VERTICES, MAX_INDICES);
}
