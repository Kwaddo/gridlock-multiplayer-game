use macroquad::prelude::*;

use crate::types::maze::Maze;
use crate::types::player::OtherPlayer;

pub fn draw_maze(maze: Option<&Maze>) {
    if let Some(m) = maze {
        for y in 0..m.height {
            for x in 0..m.width {
                let center = vec3(
                    (x as f32 + 0.5) * m.cell_size,
                    0.0,
                    (y as f32 + 0.5) * m.cell_size,
                );
                // thin floor tile - centered at -0.05 so top surface is at y=0.0
                draw_cube(
                    center + vec3(0.0, -0.05, 0.0),
                    vec3(m.cell_size, 0.1, m.cell_size),
                    None,
                    DARKGRAY,
                );

                if m.cells[m.idx(x as usize, y as usize)] == 0 {
                    // wall sits on floor - bottom at y=0.0, center at y=wall_height/2.0
                    draw_cube(
                        center + vec3(0.0, m.wall_height / 2.0, 0.0),
                        vec3(m.cell_size, m.wall_height, m.cell_size),
                        None,
                        BROWN,
                    );
                }
            }
        }
    } else {
        draw_cube(vec3(0.0, -1.0, 0.0), vec3(20.0, 0.1, 20.0), None, GRAY);
    }
}

pub fn draw_players(others: &[OtherPlayer]) {
    for p in others {
        let player_color = if p.skin.is_some() { WHITE } else { p.color };
        draw_cube(
            vec3(p.x, p.y + 0.6, p.z),
            vec3(0.6, 1.2, 0.4),
            None,
            player_color,
        );
        draw_cube(
            vec3(p.x, p.y + 1.4, p.z),
            vec3(0.4, 0.4, 0.4),
            None,
            player_color,
        );
    }
}

pub fn draw_nametags(
    others: &[OtherPlayer],
    position: Vec3,
    forward: Vec3,
    right: Vec3,
    pitch: f32,
) {
    for p in others {
        let head_pos = vec3(p.x, p.y + 1.8, p.z);
        let rel = head_pos - position;
        let dot = rel.dot(forward);
        if dot > 0.1 {
            let right_dot = rel.dot(right);
            let up_dot = rel.y - position.y + pitch.sin() * dot;
            let fov_scale = screen_height() / (2.0 * (45.0_f32.to_radians() / 2.0).tan());
            let screen_x = screen_width() / 2.0 + (right_dot / dot) * fov_scale;
            let screen_y = screen_height() / 2.0 - (up_dot / dot) * fov_scale;
            let name_w = measure_text(&p.username, None, 16, 1.0).width;
            draw_text(
                &p.username,
                screen_x - name_w / 2.0,
                screen_y,
                16.0,
                p.color,
            );
        }
    }
}
