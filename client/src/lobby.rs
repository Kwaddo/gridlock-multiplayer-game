use macroquad::prelude::*;

/// Each cube: (center position, size)
/// Landmark cubes + parkour platforms
pub const LANDMARK_CUBES: [(Vec3, Vec3); 4] = [
    (vec3(0.0, 0.5, -5.0), vec3(2.0, 1.0, 2.0)),    // red
    (vec3(5.0, 0.5, 0.0), vec3(2.0, 1.0, 2.0)),      // green
    (vec3(-5.0, 0.5, 3.0), vec3(2.0, 1.0, 2.0)),     // blue
    (vec3(0.0, 0.5, 10.0), vec3(2.0, 1.0, 2.0)),     // yellow
];

/// Parkour platforms — stepping-stone path with increasing height
pub const PARKOUR_CUBES: [(Vec3, Vec3); 10] = [
    // step 1: low & small
    (vec3(2.0, 0.3, -2.0), vec3(1.6, 0.6, 1.6)),
    // step 2: slightly higher
    (vec3(4.0, 0.7, -2.0), vec3(1.6, 0.6, 1.6)),
    // step 3
    (vec3(6.0, 1.1, -2.0), vec3(1.8, 0.6, 1.8)),
    // step 4: turn corner
    (vec3(7.5, 1.5, -0.5), vec3(1.8, 0.6, 1.8)),
    // step 5
    (vec3(8.5, 1.9, 1.2), vec3(2.0, 0.6, 2.0)),
    // step 6: wider platform to rest
    (vec3(8.5, 2.3, 3.5), vec3(2.4, 0.6, 2.4)),
    // step 7
    (vec3(6.5, 2.7, 5.0), vec3(2.0, 0.6, 2.0)),
    // step 8
    (vec3(4.5, 3.1, 6.0), vec3(2.0, 0.6, 2.0)),
    // step 9: final tall platform
    (vec3(2.5, 3.5, 6.0), vec3(2.4, 0.6, 2.4)),
    // step 10: victory platform
    (vec3(0.5, 3.9, 5.0), vec3(3.0, 0.6, 3.0)),
];

pub fn draw_lobby(floor_texture: Option<&Texture2D>) {
    // Ground
    draw_cube(vec3(0.0, -0.05, 0.0), vec3(50.0, 0.1, 50.0), floor_texture, DARKGRAY);

    // Grid lines on the ground
    for i in -25..=25 {
        let pos = i as f32;
        draw_line_3d(vec3(-25.0, 0.01, pos), vec3(25.0, 0.01, pos), Color::from_rgba(100, 100, 100, 255));
        draw_line_3d(vec3(pos, 0.01, -25.0), vec3(pos, 0.01, 25.0), Color::from_rgba(100, 100, 100, 255));
    }

    // Landmark cubes
    let colors = [RED, GREEN, BLUE, YELLOW];
    for (i, (center, size)) in LANDMARK_CUBES.iter().enumerate() {
        draw_cube(*center, *size, None, colors[i]);
        draw_cube_wires(*center, *size, BLACK);
    }

    // Parkour cubes — gradient from cyan to magenta
    for (i, (center, size)) in PARKOUR_CUBES.iter().enumerate() {
        let t = i as f32 / (PARKOUR_CUBES.len() - 1) as f32;
        let color = Color::new(0.2 + 0.6 * t, 0.8 - 0.5 * t, 0.9 - 0.3 * t, 1.0);
        draw_cube(*center, *size, None, color);
        draw_cube_wires(*center, *size, BLACK);
    }
}