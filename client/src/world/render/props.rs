use macroquad::prelude::*;

use super::utils::{apply_lighting, CeilingLight};
use crate::types::maze::{Maze, PropKind};

mod foosball;
mod office;
mod storage;

use foosball::draw_foosball;
use office::{draw_desk, draw_lamp, draw_monitor, draw_office_chair};
use storage::{draw_barrel, draw_crate, draw_supply_crate};

pub(super) fn draw_props(maze: &Maze, lights: &[CeilingLight], show_hitboxes: bool) {
    for prop in &maze.props {
        // Ensure GL is in a known-good state before drawing *each* prop. Some
        // earlier draw paths (wireframes, tracers, viewmodel) can modify
        // global GL state and forgetting to restore it can make individual
        // props vanish depending on render order.
        {
            use macroquad::window::get_internal_gl;
            let gl = unsafe { get_internal_gl() };
            gl.quad_gl.draw_mode(DrawMode::Triangles);
            gl.quad_gl.depth_test(true);
            gl.quad_gl.texture(None);
        }

        let c = maze.cell_to_world(prop.x as i32, prop.y as i32);

        // Always render props regardless of horizontal distance — nearby disappearance occurs
        // due to incorrect culling in some cases, so skip the distance-based early out here.

        // Helper to apply lighting to colors at specific positions
        let lit = |color: Color, pos: Vec3| apply_lighting(lights, color, pos);

        match prop.kind {
            PropKind::Crate => draw_crate(c, &lit),
            PropKind::Barrel => draw_barrel(c, &lit),
            PropKind::SupplyCrate => draw_supply_crate(c, &lit),
            PropKind::Lamp => draw_lamp(c, &lit),
            PropKind::Desk => draw_desk(c, &lit),
            PropKind::Monitor => draw_monitor(maze, prop, c, &lit),
            PropKind::OfficeChair => draw_office_chair(maze, prop, c, &lit),
            PropKind::Foosball => draw_foosball(c, &lit),
        }

        // Debug overlay: show prop center and bullet hitbox when enabled
        if show_hitboxes {
            // small center marker so we can tell if the prop is being culled
            draw_cube_wires(c, vec3(0.12, 0.12, 0.12), YELLOW);

            // draw bullet hitbox (if any)
            if let Some((center_offset, half_extents)) = prop.kind.bullet_hitbox() {
                draw_cube_wires(c + center_offset, half_extents * 2.0, RED);
            }
        } else {
            // WORKAROUND: the debugger path (wireframe draw) changes GL state in a
            // way that makes some props render correctly.  Call the same wireframe
            // helper with fully-transparent color so we get the same GL-side
            // effects *without visible wires* — this reproduces the 'debugger
            // fixes it' behavior while keeping normal visuals.
            let invisible = Color::new(0.0, 0.0, 0.0, 0.0);
            draw_cube_wires(c, vec3(0.12, 0.12, 0.12), invisible);
            if let Some((center_offset, half_extents)) = prop.kind.bullet_hitbox() {
                draw_cube_wires(c + center_offset, half_extents * 2.0, invisible);
            }
        }

        // Restore GL state after debug/invis-wires so rendering for following
        // code is deterministic.
        {
            use macroquad::window::get_internal_gl;
            let gl = unsafe { get_internal_gl() };
            gl.quad_gl.draw_mode(DrawMode::Triangles);
            gl.quad_gl.depth_test(true);
            gl.quad_gl.texture(None);
        }
    }
}
