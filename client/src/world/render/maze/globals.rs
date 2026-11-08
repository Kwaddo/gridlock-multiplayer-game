//! Geometry spanning the whole maze, drawn whatever the distance: the roof,
//! the pipe grid under it and every ceiling light fixture.

use std::collections::HashSet;

use macroquad::prelude::*;

use crate::types::maze::Maze;

use super::constants::*;
use super::lighting::Lighting;
use super::mesh::{MeshBuilder, MeshData};

pub(super) fn bake_globals(m: &Maze, lighting: &Lighting, destroyed_lights: &HashSet<(u8, u8)>) -> Vec<MeshData> {
    let mut out = MeshBuilder::new(false);
    let apply_light = |c: Color, p: Vec3| lighting.lit(c, p);

    let map_w = m.width as f32 * m.cell_size;
    let map_h = m.height as f32 * m.cell_size;
    let roof_bottom_y = m.wall_height + ROOF_CLEARANCE;
    let roof_top_y = roof_bottom_y + ROOF_THICKNESS;
    let roof_pos = vec3(map_w * 0.5, (roof_bottom_y + roof_top_y) * 0.5, map_h * 0.5);
    let pipe_y = roof_bottom_y - 0.035;

    out.begin_item(Vec2::ZERO);
    out.cube(
        roof_pos,
        vec3(map_w * 1.01, ROOF_THICKNESS, map_h * 1.01),
        apply_light(ROOF_COLOR, roof_pos),
    );

    let pipe_drop = (roof_bottom_y - pipe_y).max(0.03);

    // Main pipe rows and their hangers
    for row in (2..m.height as i32).step_by(PIPE_ROW_SPACING) {
        out.begin_item(Vec2::ZERO);
        let z = (row as f32 + 0.5) * m.cell_size;
        let main_pipe_pos = vec3(map_w * 0.5, pipe_y, z);
        out.cube(main_pipe_pos, vec3(map_w * 0.94, 0.07, 0.07), apply_light(PIPE_COLOR, main_pipe_pos));

        for col in (1..m.width as i32).step_by(PIPE_COL_SPACING) {
            let x = (col as f32 + 0.5) * m.cell_size;
            let hanger_pos = vec3(x, roof_bottom_y - pipe_drop * 0.5, z);
            out.cube(hanger_pos, vec3(0.03, pipe_drop, 0.03), apply_light(PIPE_JOINT_COLOR, hanger_pos));
        }
    }

    // Cross pipes
    out.begin_item(Vec2::ZERO);
    for col in (1..m.width as i32).step_by(PIPE_COL_SPACING) {
        let x = (col as f32 + 0.5) * m.cell_size;
        let cross_pipe_pos = vec3(x, pipe_y - 0.03, map_h * 0.5);
        out.cube(
            cross_pipe_pos,
            vec3(0.06, 0.06, map_h * 0.90),
            apply_light(PIPE_JOINT_COLOR, cross_pipe_pos),
        );
    }

    // Every ceiling light fixture, working or shot out
    for info in &m.ceiling_lights {
        out.begin_item(Vec2::ZERO);
        let is_destroyed = destroyed_lights.contains(&(info.x, info.y));
        let light_pos =
            m.cell_to_world(info.x as i32, info.y as i32) + vec3(0.0, roof_bottom_y - LIGHT_DROP, 0.0);
        let mount_pos = vec3(light_pos.x, roof_bottom_y - 0.015, light_pos.z);
        let stem_len = (mount_pos.y - light_pos.y).max(0.03);
        let stem_pos = vec3(light_pos.x, mount_pos.y - stem_len * 0.5, light_pos.z);
        let housing_pos = light_pos + vec3(0.0, 0.02, 0.0);
        let mount_to_pipe = (mount_pos.y - pipe_y).abs();

        if mount_to_pipe > 0.01 {
            let feeder_pos = vec3(light_pos.x, (mount_pos.y + pipe_y) * 0.5, light_pos.z);
            out.cube(feeder_pos, vec3(0.03, mount_to_pipe, 0.03), apply_light(PIPE_JOINT_COLOR, feeder_pos));
        }

        // Conduit to the nearest pipe row
        let mut best_z: Option<f32> = None;
        let mut best_dist = f32::INFINITY;
        for row in (2..m.height as i32).step_by(PIPE_ROW_SPACING) {
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
                out.cube(
                    conduit_pos,
                    vec3(0.035, 0.035, conduit_len),
                    apply_light(PIPE_JOINT_COLOR, conduit_pos),
                );
            }
        }

        out.cube(mount_pos, vec3(0.30, 0.03, 0.30), apply_light(LIGHT_MOUNT_COLOR, mount_pos));
        out.cube(stem_pos, vec3(0.05, stem_len, 0.05), apply_light(LIGHT_STEM_COLOR, stem_pos));

        if is_destroyed {
            // Dark housing and dead bulb, no glow
            out.cube(housing_pos, vec3(0.44, 0.06, 0.44), apply_light(HOUSING_DEAD_COLOR, housing_pos));
            out.cube(housing_pos + vec3(0.0, -0.04, 0.0), vec3(0.38, 0.02, 0.38), BULB_DEAD_COLOR);
        } else {
            let light_color = light_color(info.variant);
            out.cube(housing_pos, vec3(0.44, 0.06, 0.44), apply_light(HOUSING_ON_COLOR, housing_pos));
            out.cube(
                housing_pos + vec3(0.0, -0.04, 0.0),
                vec3(0.38, 0.02, 0.38),
                Color { r: 0.98 * light_color.x, g: 0.96 * light_color.y, b: 0.90 * light_color.z, a: 1.0 },
            );
        }
    }
    out.finish()
}
