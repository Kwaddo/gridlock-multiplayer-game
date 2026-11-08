//! The baked maze, kept until the maze or its lights change; then only what
//! changed is baked again.

use std::collections::HashSet;
use std::ops::Range;

use macroquad::prelude::*;

use crate::types::maze::{Maze, MazeChange, MazeSnapshot};
use crate::world::render::props::{bake_prop, PropIndex};

use super::cells::{bake_chunk, chunk_grid, chunk_of};
use super::constants::{CHUNK_CELLS, PROPS_PER_GROUP, RELIGHT_BUDGET_SECS};
use super::cull::{visible, DistanceCull, Frustum};
use super::globals::bake_globals;
use super::gpu::{release, store, StoredMesh};
use super::lighting::Lighting;
use super::mesh::{MeshBuilder, MeshData};

/// Chunks whose geometry depends on the `changed` cells: each cell's own
/// chunk and those of its four neighbours, whose wall faces toward it
/// appear or disappear.
pub(super) fn chunks_touched(m: &Maze, changed: &[(u8, u8)]) -> Vec<usize> {
    let (chunks_x, _) = chunk_grid(m);
    let mut out: Vec<usize> = Vec::new();
    for &(x, y) in changed {
        for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
            if nx < 0 || ny < 0 || nx >= m.width as i32 || ny >= m.height as i32 {
                continue;
            }
            let chunk = chunk_of(nx as usize, ny as usize, chunks_x);
            if !out.contains(&chunk) {
                out.push(chunk);
            }
        }
    }
    out
}

/// Props split into runs drawn together: consecutive in the prop list (the
/// order they always drew in, which matters for their see-through glows)
/// and in the same chunk, so each run can be frustum-culled as a whole.
/// Props are never distance-culled: they always drew at any distance, so
/// at low render distances they stay visible past the culled floor.
pub(super) fn prop_groups(m: &Maze) -> Vec<Range<usize>> {
    let (chunks_x, _) = chunk_grid(m);
    let chunk = |i: usize| chunk_of(m.props[i].x as usize, m.props[i].y as usize, chunks_x);
    let mut groups: Vec<Range<usize>> = Vec::new();
    for i in 0..m.props.len() {
        match groups.last_mut() {
            Some(g) if g.len() < PROPS_PER_GROUP && chunk(g.start) == chunk(i) => g.end = i + 1,
            _ => groups.push(i..i + 1),
        }
    }
    groups
}

pub(super) fn bake_prop_group(m: &Maze, index: &PropIndex, lighting: &Lighting, group: Range<usize>) -> Vec<MeshData> {
    // Not cullable by distance (only by the frustum).
    let mut out = MeshBuilder::new(false);
    let lit = |c: Color, p: Vec3| lighting.lit(c, p);
    for prop in &m.props[group] {
        let c = m.cell_to_world(prop.x as i32, prop.y as i32);
        out.begin_item(vec2(c.x, c.z));
        bake_prop(&mut out, m, index, prop, &lit);
    }
    out.finish()
}

/// One piece of the maze waiting to be recoloured after a light change.
#[derive(Clone, Copy)]
enum Relight {
    Chunk(usize),
    Props(usize),
}

pub(super) struct MazeMeshes {
    snapshot: MazeSnapshot,
    destroyed_lights: HashSet<(u8, u8)>,
    chunks: Vec<Vec<StoredMesh>>,
    globals: Vec<StoredMesh>,
    props: Vec<Vec<StoredMesh>>,
    /// The prop runs behind `props`, index for index.
    prop_ranges: Vec<Range<usize>>,
    /// Pieces still lit by the old lights, farthest from the change first
    /// (popped from the end, so the area around the light updates first).
    relight: Vec<Relight>,
}

impl MazeMeshes {
    pub fn new(m: &Maze, destroyed_lights: &HashSet<(u8, u8)>) -> Self {
        let mut meshes = Self {
            snapshot: MazeSnapshot::capture(m),
            destroyed_lights: destroyed_lights.clone(),
            chunks: Vec::new(),
            globals: Vec::new(),
            props: Vec::new(),
            prop_ranges: Vec::new(),
            relight: Vec::new(),
        };
        meshes.bake_all(m);
        meshes
    }

    /// Bake again what `m` or the destroyed lights changed since last time.
    pub fn sync(&mut self, m: &Maze, destroyed_lights: &HashSet<(u8, u8)>) {
        let change = self.snapshot.sync(m);
        if matches!(change, MazeChange::Layout) {
            self.destroyed_lights = destroyed_lights.clone();
            self.bake_all(m);
            return;
        }
        if *destroyed_lights != self.destroyed_lights {
            self.start_relight(m, destroyed_lights);
        }
        match change {
            MazeChange::Unchanged | MazeChange::Layout => {}
            MazeChange::Cells(cells) => {
                let lighting = Lighting::new(m, &self.destroyed_lights);
                let (chunks_x, _) = chunk_grid(m);
                for chunk in chunks_touched(m, &cells) {
                    let fresh = store(bake_chunk(m, &lighting, chunk % chunks_x, chunk / chunks_x));
                    release(std::mem::replace(&mut self.chunks[chunk], fresh));
                }
            }
        }
        self.continue_relight(m);
    }

    /// Every light reaches every cube, so a light going out recolours the
    /// whole maze. Baking all of it in one frame hitches mid-match, so the
    /// fixtures switch now and the rest is queued, nearest the change first.
    fn start_relight(&mut self, m: &Maze, destroyed_lights: &HashSet<(u8, u8)>) {
        let changed: Vec<Vec3> = destroyed_lights
            .symmetric_difference(&self.destroyed_lights)
            .map(|&(x, y)| m.cell_to_world(x as i32, y as i32))
            .collect();
        self.destroyed_lights = destroyed_lights.clone();
        let lighting = Lighting::new(m, &self.destroyed_lights);
        let fresh = store(bake_globals(m, &lighting, &self.destroyed_lights));
        release(std::mem::replace(&mut self.globals, fresh));

        let (chunks_x, _) = chunk_grid(m);
        let half = (CHUNK_CELLS / 2) as i32;
        let near = |p: Vec3| {
            changed
                .iter()
                .map(|c| (vec2(c.x, c.z) - vec2(p.x, p.z)).length_squared())
                .fold(f32::INFINITY, f32::min)
        };
        let mut jobs: Vec<(f32, Relight)> = (0..self.chunks.len())
            .map(|c| {
                let x = ((c % chunks_x) * CHUNK_CELLS) as i32 + half;
                let y = ((c / chunks_x) * CHUNK_CELLS) as i32 + half;
                (near(m.cell_to_world(x, y)), Relight::Chunk(c))
            })
            .chain(self.prop_ranges.iter().enumerate().map(|(i, g)| {
                let prop = &m.props[g.start];
                (near(m.cell_to_world(prop.x as i32, prop.y as i32)), Relight::Props(i))
            }))
            .collect();
        jobs.sort_by(|a, b| b.0.total_cmp(&a.0));
        self.relight = jobs.into_iter().map(|(_, job)| job).collect();
    }

    /// Recolour queued pieces for at most `RELIGHT_BUDGET_SECS` this frame
    /// (always at least one), the old colours drawing until each is swapped.
    fn continue_relight(&mut self, m: &Maze) {
        if self.relight.is_empty() {
            return;
        }
        let lighting = Lighting::new(m, &self.destroyed_lights);
        let (chunks_x, _) = chunk_grid(m);
        let mut index: Option<PropIndex> = None;
        let start = get_time();
        while let Some(job) = self.relight.pop() {
            match job {
                Relight::Chunk(c) => {
                    let fresh = store(bake_chunk(m, &lighting, c % chunks_x, c / chunks_x));
                    release(std::mem::replace(&mut self.chunks[c], fresh));
                }
                Relight::Props(i) => {
                    let index = index.get_or_insert_with(|| PropIndex::new(m));
                    let fresh = store(bake_prop_group(m, index, &lighting, self.prop_ranges[i].clone()));
                    release(std::mem::replace(&mut self.props[i], fresh));
                }
            }
            if get_time() - start >= RELIGHT_BUDGET_SECS {
                break;
            }
        }
    }

    fn bake_all(&mut self, m: &Maze) {
        self.release_all();
        self.relight.clear();
        let lighting = Lighting::new(m, &self.destroyed_lights);
        let (chunks_x, chunks_y) = chunk_grid(m);
        self.chunks = (0..chunks_x * chunks_y)
            .map(|c| store(bake_chunk(m, &lighting, c % chunks_x, c / chunks_x)))
            .collect();
        self.globals = store(bake_globals(m, &lighting, &self.destroyed_lights));
        let index = PropIndex::new(m);
        self.prop_ranges = prop_groups(m);
        self.props = self
            .prop_ranges
            .iter()
            .map(|g| store(bake_prop_group(m, &index, &lighting, g.clone())))
            .collect();
    }

    fn release_all(&mut self) {
        for chunk in self.chunks.drain(..).chain(self.props.drain(..)) {
            release(chunk);
        }
        release(std::mem::take(&mut self.globals));
    }

    /// Meshes that may show this frame, in the order the old path drew
    /// them: cells, then roof, pipes and fixtures, then props.
    pub fn visible(&self, frustum: &Frustum, dist: &DistanceCull) -> Vec<&StoredMesh> {
        self.chunks
            .iter()
            .flatten()
            .chain(&self.globals)
            .chain(self.props.iter().flatten())
            .filter(|m| visible(m.bounds(), frustum, dist))
            .collect()
    }
}
