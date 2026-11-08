//! Per-frame work of the old and baked paths on a match-sized maze
//! (`cargo test -p client maze_frame_cost -- --nocapture` prints it).

use std::collections::{HashMap, HashSet};

use macroquad::prelude::*;

use crate::types::maze::{CeilingLightInfo, Maze, Prop, PropKind};
use crate::world::render::props::PropIndex;

use super::super::cache::{bake_prop_group, prop_groups};
use super::super::cells::{bake_chunk, chunk_grid};
use super::super::globals::bake_globals;
use super::super::lighting::Lighting;
use super::reference::{old_cell_cubes, old_global_cubes, old_lights};

/// A 41 x 41 depth-first maze (the size of an 8-12 player match), lights
/// every 4 open cells and a prop in every 9th open cell.
fn match_maze() -> Maze {
    let side = 41usize;
    let mut cells = vec![0u8; side * side];
    let mut seed = 0x2545_f491u32;
    let mut rand = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    let mut stack = vec![(1usize, 1usize)];
    cells[side + 1] = 1;
    while let Some(&(x, y)) = stack.last() {
        let dirs: Vec<(usize, usize)> = [(2i32, 0i32), (-2, 0), (0, 2), (0, -2)]
            .iter()
            .map(|&(dx, dy)| ((x as i32 + dx) as usize, (y as i32 + dy) as usize))
            .filter(|&(nx, ny)| nx > 0 && ny > 0 && nx < side - 1 && ny < side - 1 && cells[ny * side + nx] == 0)
            .collect();
        if dirs.is_empty() {
            stack.pop();
            continue;
        }
        let (nx, ny) = dirs[rand() as usize % dirs.len()];
        cells[((y + ny) / 2) * side + (x + nx) / 2] = 1;
        cells[ny * side + nx] = 1;
        stack.push((nx, ny));
    }
    let open: Vec<(u8, u8)> = (0..side * side).filter(|&i| cells[i] == 1).map(|i| ((i % side) as u8, (i / side) as u8)).collect();
    let kinds = [PropKind::Crate, PropKind::Barrel, PropKind::Desk, PropKind::Lamp, PropKind::Foosball];
    let mut m = Maze {
        width: side as u8,
        height: side as u8,
        cells,
        cell_size: 1.8,
        wall_height: 2.4,
        teleporter: open[0],
        rooms: Vec::new(),
        props: open.iter().step_by(20).enumerate().map(|(i, &(x, y))| Prop { kind: kinds[i % kinds.len()], x, y }).collect(),
        ceiling_lights: open.iter().step_by(10).map(|&(x, y)| CeilingLightInfo { x, y, variant: x % 3 }).collect(),
        breakable_walls: HashMap::new(),
        broken_wall_scars: HashSet::new(),
    };
    m.spawn_breakable_walls();
    m
}

#[test]
fn maze_frame_cost() {
    let m = match_maze();
    let none = HashSet::new();
    let lights = old_lights(&m, &none);
    let lighting = Lighting::new(&m, &none);

    let cell_cubes: usize = (0..m.height as usize)
        .flat_map(|y| (0..m.width as usize).map(move |x| (x, y)))
        .map(|(x, y)| old_cell_cubes(&m, &lights, x, y).len())
        .sum();
    let global_cubes = old_global_cubes(&m, &lights, &none).len();

    let started = std::time::Instant::now();
    let (chunks_x, chunks_y) = chunk_grid(&m);
    let chunks: Vec<_> = (0..chunks_x * chunks_y).flat_map(|c| bake_chunk(&m, &lighting, c % chunks_x, c / chunks_x)).collect();
    let globals = bake_globals(&m, &lighting, &none);
    let index = PropIndex::new(&m);
    let props: Vec<_> = prop_groups(&m).into_iter().flat_map(|g| bake_prop_group(&m, &index, &lighting, g)).collect();
    eprintln!("baking everything took {:?} (CPU side, without the GPU upload)", started.elapsed());
    let quads = |meshes: &[super::super::mesh::MeshData]| meshes.iter().map(|m| m.vertices.len() / 4).sum::<usize>();
    let prop_quads = quads(&props);
    let prop_cubes = prop_quads / 6;

    let old_cubes = cell_cubes + global_cubes + prop_cubes;
    let new_quads = quads(&chunks) + quads(&globals) + prop_quads;
    let new_draws = chunks.len() + globals.len() + props.len();
    eprintln!(
        "{}x{} maze, {} lights, {} props\n\
         old, every frame: {} draw_cube calls, {} vertices built on the CPU, {} light evaluations\n\
         baked, every frame (nothing culled): {} draw calls, {} vertices on the GPU, 0 light evaluations\n\
         baked once per layout or light change: {} light evaluations",
        m.width,
        m.height,
        lights.len(),
        m.props.len(),
        old_cubes,
        old_cubes * 24,
        old_cubes * lights.len(),
        new_draws,
        new_quads * 4,
        old_cubes * lights.len(),
    );
    eprintln!("cells alone: {} cubes ({} faces) before, {} faces baked", cell_cubes, cell_cubes * 6, quads(&chunks));
    assert!(quads(&chunks) * 3 < cell_cubes * 6 * 2, "baking should drop over a third of the cell faces");
}
