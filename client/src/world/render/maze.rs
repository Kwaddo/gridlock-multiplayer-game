use macroquad::prelude::*;

use crate::types::maze::Maze;

use super::props::draw_props;
use super::utils::{cell_noise, tint_color};

pub fn draw_maze(maze: Option<&Maze>) {
    if let Some(m) = maze {
        let base_floor_color = Color::new(0.20, 0.22, 0.26, 1.0);
        let top_floor_color = Color::new(0.25, 0.27, 0.31, 1.0);
        let base_wall_color = Color::new(0.54, 0.42, 0.26, 1.0);

        for y in 0..m.height {
            for x in 0..m.width {
                let center = vec3(
                    (x as f32 + 0.5) * m.cell_size,
                    0.0,
                    (y as f32 + 0.5) * m.cell_size,
                );

                // Floor with very subtle per-cell gradient variation
                let floor_bottom =
                    tint_color(base_floor_color, x as i32, y as i32, 0x1, 0.02, 0.01);
                let floor_top = tint_color(top_floor_color, x as i32, y as i32, 0x1, 0.02, 0.01);
                // Bottom layer
                draw_cube(
                    center + vec3(0.0, -0.07, 0.0),
                    vec3(m.cell_size, 0.06, m.cell_size),
                    None,
                    floor_bottom,
                );
                // Top layer
                draw_cube(
                    center + vec3(0.0, -0.03, 0.0),
                    vec3(m.cell_size, 0.06, m.cell_size),
                    None,
                    floor_top,
                );

                if m.cells[m.idx(x as usize, y as usize)] == 0 {
                    // Wall: single column color with very subtle vertical gradient
                    let wall_column = tint_color(base_wall_color, x as i32, 0, 0x2, 0.03, 0.015);
                    let wall_bottom = wall_column;
                    let wall_top = Color {
                        r: (wall_column.r + 0.01).clamp(0.0, 1.0),
                        g: (wall_column.g + 0.01).clamp(0.0, 1.0),
                        b: (wall_column.b + 0.01).clamp(0.0, 1.0),
                        a: wall_column.a,
                    };
                    // Bottom half
                    draw_cube(
                        center + vec3(0.0, m.wall_height * 0.25, 0.0),
                        vec3(m.cell_size, m.wall_height * 0.5, m.cell_size),
                        None,
                        wall_bottom,
                    );
                    // Top half
                    draw_cube(
                        center + vec3(0.0, m.wall_height * 0.75, 0.0),
                        vec3(m.cell_size, m.wall_height * 0.5, m.cell_size),
                        None,
                        wall_top,
                    );
                }
            }
        }

        draw_props(m);
    } else {
        draw_cube(vec3(0.0, -1.0, 0.0), vec3(20.0, 0.1, 20.0), None, GRAY);
    }
}
