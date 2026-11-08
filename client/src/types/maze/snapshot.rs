//! A copy of everything a cached view of the maze (baked meshes, minimap
//! cells) is built from, so a cache can tell each frame whether the maze it
//! was built for changed, and how. Comparing is a few kilobytes of memcmp,
//! and it catches every mutation, not only the ones made through `Maze`
//! methods.

use super::Maze;

/// What changed since the snapshot was last synced.
#[derive(Debug, PartialEq, Eq)]
pub enum MazeChange {
    Unchanged,
    /// Only these cells flipped between wall and open (a wall broke).
    Cells(Vec<(u8, u8)>),
    /// A different maze: dimensions, props, lights, rooms or walls changed.
    Layout,
}

#[derive(Clone, PartialEq)]
struct Layout {
    width: u8,
    height: u8,
    cell_size: u32,
    wall_height: u32,
    teleporter: (u8, u8),
    props: Vec<(u8, u8, u8)>,
    lights: Vec<(u8, u8, u8)>,
    rooms: Vec<(u8, u8, u8, u8)>,
}

impl Layout {
    fn of(m: &Maze) -> Self {
        Self {
            width: m.width,
            height: m.height,
            cell_size: m.cell_size.to_bits(),
            wall_height: m.wall_height.to_bits(),
            teleporter: m.teleporter,
            props: m.props.iter().map(|p| (p.kind as u8, p.x, p.y)).collect(),
            lights: m.ceiling_lights.iter().map(|l| (l.x, l.y, l.variant)).collect(),
            rooms: m.rooms.iter().map(|r| (r.x, r.y, r.w, r.h)).collect(),
        }
    }

    fn matches(&self, m: &Maze) -> bool {
        self.width == m.width
            && self.height == m.height
            && self.cell_size == m.cell_size.to_bits()
            && self.wall_height == m.wall_height.to_bits()
            && self.teleporter == m.teleporter
            && self.props.len() == m.props.len()
            && self.props.iter().zip(&m.props).all(|(a, p)| *a == (p.kind as u8, p.x, p.y))
            && self.lights.len() == m.ceiling_lights.len()
            && self.lights.iter().zip(&m.ceiling_lights).all(|(a, l)| *a == (l.x, l.y, l.variant))
            && self.rooms.len() == m.rooms.len()
            && self.rooms.iter().zip(&m.rooms).all(|(a, r)| *a == (r.x, r.y, r.w, r.h))
    }
}

pub struct MazeSnapshot {
    layout: Layout,
    cells: Vec<u8>,
    breakable_count: usize,
}

impl MazeSnapshot {
    pub fn capture(m: &Maze) -> Self {
        Self {
            layout: Layout::of(m),
            cells: m.cells.clone(),
            breakable_count: m.breakable_walls.len(),
        }
    }

    /// Compare with `m`, then remember `m` as the new state.
    pub fn sync(&mut self, m: &Maze) -> MazeChange {
        if !self.layout.matches(m) || self.cells.len() != m.cells.len() {
            *self = Self::capture(m);
            return MazeChange::Layout;
        }
        if self.cells != m.cells {
            let w = m.width.max(1) as usize;
            let changed = self
                .cells
                .iter()
                .zip(&m.cells)
                .enumerate()
                .filter(|(_, (a, b))| a != b)
                .map(|(i, _)| ((i % w) as u8, (i / w) as u8))
                .collect();
            self.cells.copy_from_slice(&m.cells);
            self.breakable_count = m.breakable_walls.len();
            return MazeChange::Cells(changed);
        }
        if self.breakable_count != m.breakable_walls.len() {
            // Breakable walls changed without any cell opening: not something
            // the game does, so rebuild everything rather than guess.
            self.breakable_count = m.breakable_walls.len();
            return MazeChange::Layout;
        }
        MazeChange::Unchanged
    }
}
