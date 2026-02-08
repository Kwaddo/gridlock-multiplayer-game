use macroquad::prelude::*;

pub fn draw_lobby(floor_texture: Option<&Texture2D>) {
    // Ground
    draw_cube(vec3(0.0, -0.05, 0.0), vec3(50.0, 0.1, 50.0), floor_texture, DARKGRAY);

    // Grid lines on the ground
    for i in -25..=25 {
        let pos = i as f32;
        draw_line_3d(vec3(-25.0, 0.01, pos), vec3(25.0, 0.01, pos), Color::from_rgba(100, 100, 100, 255));
        draw_line_3d(vec3(pos, 0.01, -25.0), vec3(pos, 0.01, 25.0), Color::from_rgba(100, 100, 100, 255));
    }

    // Landmark cubes so you can orient yourself
    draw_cube(vec3(0.0, 0.5, -5.0), vec3(1.0, 1.0, 1.0), None, RED);
    draw_cube(vec3(5.0, 0.5, 0.0), vec3(1.0, 1.0, 1.0), None, GREEN);
    draw_cube(vec3(-5.0, 0.5, 3.0), vec3(1.0, 1.0, 1.0), None, BLUE);
    draw_cube(vec3(0.0, 0.5, 10.0), vec3(1.0, 1.0, 1.0), None, YELLOW);
}