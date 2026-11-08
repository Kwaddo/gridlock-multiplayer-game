use std::collections::HashSet;

use macroquad::prelude::*;

use crate::types::maze::{Maze, Prop, PropKind};
use crate::world::render::maze::MeshBuilder;

mod foosball;
mod office;
mod storage;

use foosball::draw_foosball;
use office::{draw_desk, draw_lamp, draw_monitor, draw_office_chair};
use storage::{draw_barrel, draw_crate, draw_supply_crate};

/// Where the office chairs and monitors are, so each monitor and chair can
/// turn toward its partner without scanning every prop.
pub(in crate::world::render) struct PropIndex {
    chairs: HashSet<(i32, i32)>,
    monitors: HashSet<(i32, i32)>,
}

impl PropIndex {
    pub fn new(maze: &Maze) -> Self {
        let cells_of = |kind: fn(PropKind) -> bool| {
            maze.props
                .iter()
                .filter(|p| kind(p.kind))
                .map(|p| (p.x as i32, p.y as i32))
                .collect()
        };
        Self {
            chairs: cells_of(|k| matches!(k, PropKind::OfficeChair)),
            monitors: cells_of(|k| matches!(k, PropKind::Monitor)),
        }
    }

    /// Whether `set` has a prop in the cell above, below, left and right of `prop`.
    fn around(set: &HashSet<(i32, i32)>, prop: &Prop) -> [bool; 4] {
        let (px, py) = (prop.x as i32, prop.y as i32);
        [(0, -1), (0, 1), (-1, 0), (1, 0)].map(|(dx, dy)| {
            let (nx, ny) = (px + dx, py + dy);
            nx >= 0 && ny >= 0 && set.contains(&(nx, ny))
        })
    }

    pub fn chairs_around(&self, prop: &Prop) -> [bool; 4] {
        Self::around(&self.chairs, prop)
    }

    pub fn monitors_around(&self, prop: &Prop) -> [bool; 4] {
        Self::around(&self.monitors, prop)
    }
}

/// Bake `prop` into `out`, coloured by `lit` (the maze lighting).
pub(in crate::world::render) fn bake_prop(
    out: &mut MeshBuilder,
    maze: &Maze,
    index: &PropIndex,
    prop: &Prop,
    lit: &impl Fn(Color, Vec3) -> Color,
) {
    let c = maze.cell_to_world(prop.x as i32, prop.y as i32);
    match prop.kind {
        PropKind::Crate => draw_crate(out, c, lit),
        PropKind::Barrel => draw_barrel(out, c, lit),
        PropKind::SupplyCrate => draw_supply_crate(out, c, lit),
        PropKind::Lamp => draw_lamp(out, c, lit),
        PropKind::Desk => draw_desk(out, c, lit),
        PropKind::Monitor => draw_monitor(out, index.chairs_around(prop), c, lit),
        PropKind::OfficeChair => draw_office_chair(out, index.monitors_around(prop), c, lit),
        PropKind::Foosball => draw_foosball(out, c, lit),
    }
}

/// Debug overlay: each prop's centre and bullet hitbox.
pub(in crate::world::render) fn draw_prop_hitboxes(maze: &Maze) {
    for prop in &maze.props {
        let c = maze.cell_to_world(prop.x as i32, prop.y as i32);
        // small center marker so we can tell if the prop is being culled
        draw_cube_wires(c, vec3(0.12, 0.12, 0.12), YELLOW);
        if let Some((center_offset, half_extents)) = prop.kind.bullet_hitbox() {
            draw_cube_wires(c + center_offset, half_extents * 2.0, RED);
        }
    }
}
