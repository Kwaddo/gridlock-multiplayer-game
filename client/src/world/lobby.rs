use macroquad::prelude::*;

/// Each cube: (center position, size)
/// Landmark cubes + parkour platforms
pub const LANDMARK_CUBES: [(Vec3, Vec3); 4] = [
    (vec3(0.0, 0.5, -5.0), vec3(2.0, 1.0, 2.0)), // red
    (vec3(5.0, 0.5, 0.0), vec3(2.0, 1.0, 2.0)),  // green
    (vec3(-5.0, 0.5, 3.0), vec3(2.0, 1.0, 2.0)), // blue
    (vec3(0.0, 0.5, 10.0), vec3(2.0, 1.0, 2.0)), // yellow
];

/// Parkour platforms — separated stepping stones with real jump gaps.
/// Max jump height ~1.6 units, max horizontal ~4 units at walk speed.
pub const PARKOUR_CUBES: [(Vec3, Vec3); 12] = [
    // start platform — wider so you can line up
    (vec3(3.0, 0.3, -3.0), vec3(1.4, 0.4, 1.4)),
    // gap ~2.5 units, slight height increase
    (vec3(5.5, 0.7, -3.0), vec3(1.0, 0.4, 1.0)),
    // gap ~2.5 units, going forward
    (vec3(8.0, 1.2, -3.0), vec3(1.0, 0.4, 1.0)),
    // turn right — diagonal jump
    (vec3(10.0, 1.7, -1.0), vec3(1.0, 0.4, 1.0)),
    // continue turning — gap ~2.5
    (vec3(10.0, 2.2, 1.5), vec3(1.2, 0.4, 1.2)),
    // rest platform — slightly wider
    (vec3(10.0, 2.5, 4.0), vec3(1.6, 0.4, 1.6)),
    // turn back — long gap ~3 units
    (vec3(7.5, 3.0, 5.5), vec3(1.0, 0.4, 1.0)),
    // zigzag
    (vec3(5.0, 3.5, 4.0), vec3(1.0, 0.4, 1.0)),
    // back the other way
    (vec3(3.0, 4.0, 5.5), vec3(1.0, 0.4, 1.0)),
    // almost there
    (vec3(1.0, 4.5, 4.0), vec3(1.0, 0.4, 1.0)),
    // last jump — big gap
    (vec3(-1.5, 5.0, 2.5), vec3(1.0, 0.4, 1.0)),
    // victory platform — wide
    (vec3(-1.5, 5.3, 0.0), vec3(2.0, 0.4, 2.0)),
];

// Physical console placement in the lobby (player interacts to open HUD)
// bottom y is 0.0 so the console sits flush on the floor
pub const LOBBY_CONSOLE_POS: Vec3 = vec3(2.0, 0.0, -9.0);
pub const LOBBY_CONSOLE_SIZE: Vec3 = vec3(0.8, 1.2, 0.4);

pub fn draw_lobby(floor_texture: Option<&Texture2D>) {
    // Ground
    draw_cube(
        vec3(0.0, -0.05, 0.0),
        vec3(50.0, 0.1, 50.0),
        floor_texture,
        DARKGRAY,
    );

    // Grid lines on the ground
    for i in -25..=25 {
        let pos = i as f32;
        draw_line_3d(
            vec3(-25.0, 0.01, pos),
            vec3(25.0, 0.01, pos),
            Color::from_rgba(100, 100, 100, 255),
        );
        draw_line_3d(
            vec3(pos, 0.01, -25.0),
            vec3(pos, 0.01, 25.0),
            Color::from_rgba(100, 100, 100, 255),
        );
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

    // Draw the physical lobby console
    draw_cube(LOBBY_CONSOLE_POS + vec3(0.0, LOBBY_CONSOLE_SIZE.y / 2.0, 0.0), LOBBY_CONSOLE_SIZE, None, Color::from_rgba(200, 120, 60, 255));
    draw_cube_wires(LOBBY_CONSOLE_POS + vec3(0.0, LOBBY_CONSOLE_SIZE.y / 2.0, 0.0), LOBBY_CONSOLE_SIZE, BLACK);
}

