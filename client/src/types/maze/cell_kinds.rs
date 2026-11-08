//! Per-cell classification (wall, breakable wall, corridor, room floor) kept
//! up to date with the maze, so per-frame views like the minimap read one
//! byte per cell instead of walking rooms and the breakable-wall map.

use super::snapshot::{MazeChange, MazeSnapshot};
use super::Maze;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellKind {
    Wall,
    BreakableWall,
    Floor,
    RoomFloor,
}

/// Classify every cell of `m`, row by row.
pub fn classify_cells(m: &Maze) -> Vec<CellKind> {
    let w = m.width as usize;
    let len = w * m.height as usize;
    let mut in_room = vec![false; len];
    for room in &m.rooms {
        for ry in room.y..room.y.saturating_add(room.h) {
            for rx in room.x..room.x.saturating_add(room.w) {
                let idx = ry as usize * w + rx as usize;
                if idx < len {
                    in_room[idx] = true;
                }
            }
        }
    }
    (0..len)
        .map(|i| {
            let (x, y) = ((i % w) as u8, (i / w) as u8);
            if m.cells.get(i) == Some(&1) {
                if in_room[i] {
                    CellKind::RoomFloor
                } else {
                    CellKind::Floor
                }
            } else if m.is_breakable(x, y) {
                CellKind::BreakableWall
            } else {
                CellKind::Wall
            }
        })
        .collect()
}

#[derive(Default)]
pub struct CellKindCache {
    snapshot: Option<MazeSnapshot>,
    width: i32,
    height: i32,
    kinds: Vec<CellKind>,
}

impl CellKindCache {
    pub const fn new() -> Self {
        Self { snapshot: None, width: 0, height: 0, kinds: Vec::new() }
    }

    /// Reclassify if `m` changed since the last call (cheap when it did not).
    pub fn sync(&mut self, m: &Maze) {
        let changed = match &mut self.snapshot {
            Some(s) => s.sync(m) != MazeChange::Unchanged,
            None => {
                self.snapshot = Some(MazeSnapshot::capture(m));
                true
            }
        };
        if changed {
            self.width = m.width as i32;
            self.height = m.height as i32;
            self.kinds = classify_cells(m);
        }
    }

    /// Kind of cell (`cx`, `cy`); outside the maze counts as wall.
    pub fn kind_at(&self, cx: i32, cy: i32) -> CellKind {
        if cx < 0 || cy < 0 || cx >= self.width || cy >= self.height {
            return CellKind::Wall;
        }
        self.kinds
            .get((cy * self.width + cx) as usize)
            .copied()
            .unwrap_or(CellKind::Wall)
    }
}
