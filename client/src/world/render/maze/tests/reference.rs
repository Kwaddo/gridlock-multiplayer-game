//! The per-frame path this module replaced, kept verbatim except that
//! `draw_cube` records its cube instead of drawing it, plus macroquad's
//! `draw_cube` vertex layout. The baked meshes are checked against these.

use std::collections::HashSet;

use macroquad::prelude::*;

use crate::types::maze::Maze;
use crate::world::render::utils::{apply_lighting, tint_color, CeilingLight};

/// A recorded `draw_cube(position, size, None, color)`.
#[derive(Clone, Copy, Debug)]
pub struct Cube {
    pub pos: Vec3,
    pub size: Vec3,
    pub color: Color,
}

/// The ceiling lights the old path built each frame.
pub fn old_lights(m: &Maze, destroyed_lights: &HashSet<(u8, u8)>) -> Vec<CeilingLight> {
    let roof_bottom_y = m.wall_height + 0.05;
    m.ceiling_lights
        .iter()
        .filter(|info| !destroyed_lights.contains(&(info.x, info.y)))
        .map(|info| {
            let color = match info.variant % 3 {
                0 => vec3(0.95, 0.94, 0.88),
                1 => vec3(1.0, 0.95, 0.85),
                _ => vec3(0.88, 0.92, 0.98),
            };
            CeilingLight {
                pos: m.cell_to_world(info.x as i32, info.y as i32) + vec3(0.0, roof_bottom_y - 0.16, 0.0),
                color,
                intensity: 0.38,
            }
        })
        .collect()
}

/// Cubes the old loop drew for cell (`x`, `y`), in order.
pub fn old_cell_cubes(m: &Maze, ceiling_lights: &[CeilingLight], x: usize, y: usize) -> Vec<Cube> {
    let mut rec = Vec::new();
    let mut draw_cube = |pos: Vec3, size: Vec3, _: Option<&()>, color: Color| rec.push(Cube { pos, size, color });
    let apply_light = |c: Color, p: Vec3| -> Color { apply_lighting(ceiling_lights, c, p) };
    let floor_light = Color::new(0.74, 0.72, 0.69, 1.0);
    let floor_dark = Color::new(0.66, 0.64, 0.60, 1.0);
    let wall_color = Color::new(0.80, 0.82, 0.84, 1.0);
    let wall_breakable = Color::new(0.85, 0.82, 0.74, 1.0);
    let ceiling_panel_light = Color::new(0.30, 0.31, 0.34, 1.0);
    let ceiling_panel_dark = Color::new(0.26, 0.27, 0.30, 1.0);
    let roof_bottom_y = m.wall_height + 0.05;
    let panel_inset = 0.03;
    let panel_thickness = 0.03;
    let panel_drop = 0.018;

    let center = vec3((x as f32 + 0.5) * m.cell_size, 0.0, (y as f32 + 0.5) * m.cell_size);
    let is_dark_tile = ((x + y) & 1) == 0;
    let tile_base = if is_dark_tile { floor_dark } else { floor_light };
    let tile_tint = tint_color(tile_base, x as i32, y as i32, 0x11, 0.018, 0.008);
    draw_cube(
        center + vec3(0.0, -0.025, 0.0),
        vec3(m.cell_size * 1.000, 0.05, m.cell_size * 1.000),
        None,
        apply_light(tile_tint, center),
    );
    let is_wall = m.cells[m.idx(x, y)] == 0;
    if !is_wall {
        let panel_base = if is_dark_tile { ceiling_panel_dark } else { ceiling_panel_light };
        let panel_tint = tint_color(panel_base, x as i32, y as i32, 0x51, 0.016, 0.006);
        let panel_pos = center + vec3(0.0, roof_bottom_y - panel_drop, 0.0);
        draw_cube(
            panel_pos,
            vec3(m.cell_size * (1.0 - panel_inset), panel_thickness, m.cell_size * (1.0 - panel_inset)),
            None,
            apply_light(panel_tint, panel_pos),
        );
    }
    let cx_u8 = x as u8;
    let cy_u8 = y as u8;
    if is_wall {
        let base = if m.is_breakable(cx_u8, cy_u8) { wall_breakable } else { wall_color };
        let wall_tint = tint_color(base, x as i32, y as i32, 0x33, 0.026, 0.008);
        let wall_pos = center + vec3(0.0, m.wall_height * 0.5 - 0.01, 0.0);
        draw_cube(
            wall_pos,
            vec3(m.cell_size * 1.00, m.wall_height + 0.02, m.cell_size * 1.00),
            None,
            apply_light(wall_tint, wall_pos),
        );
        if m.is_breakable(cx_u8, cy_u8) {
            let brace = Color::new(0.86, 0.70, 0.22, 1.0);
            draw_cube(
                center + vec3(0.0, m.wall_height * 0.62, m.cell_size * 0.49),
                vec3(m.cell_size * 0.62, 0.045, 0.02),
                None,
                apply_light(brace, center + vec3(0.0, m.wall_height * 0.62, 0.0)),
            );
        }
    }
    rec
}

/// Roof, pipes and light fixtures as the old path drew them, in order.
pub fn old_global_cubes(m: &Maze, ceiling_lights: &[CeilingLight], destroyed_lights: &HashSet<(u8, u8)>) -> Vec<Cube> {
    let mut rec = Vec::new();
    let mut draw_cube = |pos: Vec3, size: Vec3, _: Option<&()>, color: Color| rec.push(Cube { pos, size, color });
    let apply_light = |c: Color, p: Vec3| -> Color { apply_lighting(ceiling_lights, c, p) };
    let roof_color = Color::new(0.23, 0.24, 0.26, 1.0);
    let pipe_color = Color::new(0.50, 0.53, 0.56, 1.0);
    let pipe_joint_color = Color::new(0.38, 0.40, 0.44, 1.0);
    let map_w = m.width as f32 * m.cell_size;
    let map_h = m.height as f32 * m.cell_size;
    let roof_thickness = 0.06;
    let roof_clearance = 0.05;
    let roof_bottom_y = m.wall_height + roof_clearance;
    let roof_top_y = roof_bottom_y + roof_thickness;
    let roof_pos = vec3(map_w * 0.5, (roof_bottom_y + roof_top_y) * 0.5, map_h * 0.5);
    let pipe_row_spacing = 6usize;
    let pipe_col_spacing = 6usize;
    let pipe_y = roof_bottom_y - 0.035;

    draw_cube(roof_pos, vec3(map_w * 1.01, roof_thickness, map_h * 1.01), None, apply_light(roof_color, roof_pos));
    let pipe_drop = (roof_bottom_y - pipe_y).max(0.03);
    for row in (2..m.height as i32).step_by(pipe_row_spacing) {
        let z = (row as f32 + 0.5) * m.cell_size;
        let main_pipe_pos = vec3(map_w * 0.5, pipe_y, z);
        draw_cube(main_pipe_pos, vec3(map_w * 0.94, 0.07, 0.07), None, apply_light(pipe_color, main_pipe_pos));
        for col in (1..m.width as i32).step_by(pipe_col_spacing) {
            let x = (col as f32 + 0.5) * m.cell_size;
            let hanger_pos = vec3(x, roof_bottom_y - pipe_drop * 0.5, z);
            draw_cube(hanger_pos, vec3(0.03, pipe_drop, 0.03), None, apply_light(pipe_joint_color, hanger_pos));
        }
    }
    for col in (1..m.width as i32).step_by(pipe_col_spacing) {
        let x = (col as f32 + 0.5) * m.cell_size;
        let cross_pipe_pos = vec3(x, pipe_y - 0.03, map_h * 0.5);
        draw_cube(cross_pipe_pos, vec3(0.06, 0.06, map_h * 0.90), None, apply_light(pipe_joint_color, cross_pipe_pos));
    }
    for info in &m.ceiling_lights {
        let is_destroyed = destroyed_lights.contains(&(info.x, info.y));
        let light_pos = m.cell_to_world(info.x as i32, info.y as i32) + vec3(0.0, roof_bottom_y - 0.16, 0.0);
        let mount_pos = vec3(light_pos.x, roof_bottom_y - 0.015, light_pos.z);
        let stem_len = (mount_pos.y - light_pos.y).max(0.03);
        let stem_pos = vec3(light_pos.x, mount_pos.y - stem_len * 0.5, light_pos.z);
        let housing_pos = light_pos + vec3(0.0, 0.02, 0.0);
        let mount_to_pipe = (mount_pos.y - pipe_y).abs();
        if mount_to_pipe > 0.01 {
            let feeder_pos = vec3(light_pos.x, (mount_pos.y + pipe_y) * 0.5, light_pos.z);
            draw_cube(feeder_pos, vec3(0.03, mount_to_pipe, 0.03), None, apply_light(pipe_joint_color, feeder_pos));
        }
        let mut best_z: Option<f32> = None;
        let mut best_dist = f32::INFINITY;
        for row in (2..m.height as i32).step_by(pipe_row_spacing) {
            let z = (row as f32 + 0.5) * m.cell_size;
            let d = (z - light_pos.z).abs();
            if d < best_dist {
                best_dist = d;
                best_z = Some(z);
            }
        }
        if let Some(nearest_pipe_z) = best_z {
            let conduit_len = (nearest_pipe_z - light_pos.z).abs();
            if conduit_len > 0.06 {
                let conduit_pos = vec3(light_pos.x, pipe_y, (nearest_pipe_z + light_pos.z) * 0.5);
                draw_cube(conduit_pos, vec3(0.035, 0.035, conduit_len), None, apply_light(pipe_joint_color, conduit_pos));
            }
        }
        draw_cube(mount_pos, vec3(0.30, 0.03, 0.30), None, apply_light(Color::new(0.72, 0.74, 0.78, 1.0), mount_pos));
        draw_cube(stem_pos, vec3(0.05, stem_len, 0.05), None, apply_light(Color::new(0.60, 0.62, 0.66, 1.0), stem_pos));
        if is_destroyed {
            draw_cube(housing_pos, vec3(0.44, 0.06, 0.44), None, apply_light(Color::new(0.30, 0.30, 0.32, 1.0), housing_pos));
            draw_cube(housing_pos + vec3(0.0, -0.04, 0.0), vec3(0.38, 0.02, 0.38), None, Color::new(0.15, 0.14, 0.12, 1.0));
        } else {
            let light_color = match info.variant % 3 {
                0 => vec3(0.95, 0.94, 0.88),
                1 => vec3(1.0, 0.95, 0.85),
                _ => vec3(0.88, 0.92, 0.98),
            };
            draw_cube(housing_pos, vec3(0.44, 0.06, 0.44), None, apply_light(Color::new(0.86, 0.86, 0.88, 1.0), housing_pos));
            draw_cube(
                housing_pos + vec3(0.0, -0.04, 0.0),
                vec3(0.38, 0.02, 0.38),
                None,
                Color { r: 0.98 * light_color.x, g: 0.96 * light_color.y, b: 0.90 * light_color.z, a: 1.0 },
            );
        }
    }
    rec
}

/// One face as macroquad's `draw_cube` emits it: corners and byte colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Face {
    pub corners: [[f32; 3]; 4],
    pub color: [u8; 4],
}

/// The six faces of a cube, verbatim from macroquad 0.4's `draw_cube`
/// (front, back, top, bottom, right, left).
pub fn draw_cube_faces(cube: &Cube) -> [Face; 6] {
    let (x, y, z) = (cube.pos.x, cube.pos.y, cube.pos.z);
    let (width, height, length) = (cube.size.x, cube.size.y, cube.size.z);
    let color: [u8; 4] = cube.color.into();
    let face = |c: [Vec3; 4]| Face { corners: c.map(|v| v.to_array()), color };
    [
        face([
            vec3(x - width / 2., y - height / 2., z + length / 2.),
            vec3(x + width / 2., y - height / 2., z + length / 2.),
            vec3(x + width / 2., y + height / 2., z + length / 2.),
            vec3(x - width / 2., y + height / 2., z + length / 2.),
        ]),
        face([
            vec3(x - width / 2., y - height / 2., z - length / 2.),
            vec3(x + width / 2., y - height / 2., z - length / 2.),
            vec3(x + width / 2., y + height / 2., z - length / 2.),
            vec3(x - width / 2., y + height / 2., z - length / 2.),
        ]),
        face([
            vec3(x - width / 2., y + height / 2., z - length / 2.),
            vec3(x - width / 2., y + height / 2., z + length / 2.),
            vec3(x + width / 2., y + height / 2., z + length / 2.),
            vec3(x + width / 2., y + height / 2., z - length / 2.),
        ]),
        face([
            vec3(x - width / 2., y - height / 2., z - length / 2.),
            vec3(x - width / 2., y - height / 2., z + length / 2.),
            vec3(x + width / 2., y - height / 2., z + length / 2.),
            vec3(x + width / 2., y - height / 2., z - length / 2.),
        ]),
        face([
            vec3(x + width / 2., y - height / 2., z - length / 2.),
            vec3(x + width / 2., y + height / 2., z - length / 2.),
            vec3(x + width / 2., y + height / 2., z + length / 2.),
            vec3(x + width / 2., y - height / 2., z + length / 2.),
        ]),
        face([
            vec3(x - width / 2., y - height / 2., z - length / 2.),
            vec3(x - width / 2., y + height / 2., z - length / 2.),
            vec3(x - width / 2., y + height / 2., z + length / 2.),
            vec3(x - width / 2., y - height / 2., z + length / 2.),
        ]),
    ]
}
