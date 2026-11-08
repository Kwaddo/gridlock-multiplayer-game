use std::collections::HashSet;

use macroquad::prelude::*;

use crate::types::maze::Maze;

use super::super::utils::{apply_lighting, CeilingLight};
use super::constants::{light_color, LIGHT_DROP, LIGHT_INTENSITY, ROOF_CLEARANCE};

/// The ceiling lights still working, which colour every baked cube.
pub(super) struct Lighting {
    pub lights: Vec<CeilingLight>,
}

impl Lighting {
    pub fn new(m: &Maze, destroyed_lights: &HashSet<(u8, u8)>) -> Self {
        let roof_bottom_y = m.wall_height + ROOF_CLEARANCE;
        let lights = m
            .ceiling_lights
            .iter()
            .filter(|info| !destroyed_lights.contains(&(info.x, info.y)))
            .map(|info| CeilingLight {
                pos: m.cell_to_world(info.x as i32, info.y as i32)
                    + vec3(0.0, roof_bottom_y - LIGHT_DROP, 0.0),
                color: light_color(info.variant),
                intensity: LIGHT_INTENSITY,
            })
            .collect();
        Self { lights }
    }

    /// `c` lit at `p`, the colour the old per-frame path computed.
    pub fn lit(&self, c: Color, p: Vec3) -> Color {
        apply_lighting(&self.lights, c, p)
    }
}
