use crate::weapons::combat::ray_hit_aabb_with_normal;
use macroquad::prelude::*;
use super::shooting_range;

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
    shooting_range::draw_shooting_range(floor_texture);

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

    // Draw the improved lobby console (multi-part model)
    draw_lobby_console_model();
}

/// Improved multi-part console model: pedestal base, angled screen, accent trim.
fn draw_lobby_console_model() {
    let base_pos = LOBBY_CONSOLE_POS;

    // pedestal base (dark metal)
    let pedestal_h = 0.7;
    let pedestal_size = vec3(0.9, pedestal_h, 0.5);
    let pedestal_center = base_pos + vec3(0.0, pedestal_h / 2.0, 0.0);
    draw_cube(pedestal_center, pedestal_size, None, Color::from_rgba(50, 52, 58, 255));
    draw_cube_wires(pedestal_center, pedestal_size, Color::from_rgba(30, 30, 35, 255));

    // screen housing (slightly angled look via two stacked sections)
    let screen_base_h = 0.45;
    let screen_base_size = vec3(0.75, screen_base_h, 0.35);
    let screen_base_center = base_pos + vec3(0.0, pedestal_h + screen_base_h / 2.0, 0.0);
    draw_cube(screen_base_center, screen_base_size, None, Color::from_rgba(40, 42, 48, 255));
    draw_cube_wires(screen_base_center, screen_base_size, Color::from_rgba(25, 25, 30, 255));

    // glowing screen face (front face highlight — orange accent matching UI theme)
    let screen_h = 0.30;
    let screen_w = 0.60;
    let screen_depth = 0.02;
    let screen_center = base_pos + vec3(0.0, pedestal_h + 0.10 + screen_h / 2.0, -0.17);
    draw_cube(screen_center, vec3(screen_w, screen_h, screen_depth), None, Color::from_rgba(255, 107, 53, 240));

    // screen border glow
    draw_cube_wires(screen_center, vec3(screen_w + 0.04, screen_h + 0.04, screen_depth + 0.01), Color::from_rgba(255, 140, 80, 180));

    // top accent strip (orange trim)
    let trim_h = 0.04;
    let trim_center = base_pos + vec3(0.0, pedestal_h + screen_base_h + trim_h / 2.0, 0.0);
    draw_cube(trim_center, vec3(0.80, trim_h, 0.38), None, Color::from_rgba(255, 107, 53, 200));

    // base accent strip
    let base_trim_center = base_pos + vec3(0.0, 0.02, 0.0);
    draw_cube(base_trim_center, vec3(0.95, 0.04, 0.55), None, Color::from_rgba(255, 107, 53, 150));
}

/// Ground plane as an AABB for ray testing (matches the draw_cube in draw_lobby).
const GROUND_CENTER: Vec3 = vec3(0.0, -0.05, 0.0);
const GROUND_SIZE: Vec3 = vec3(50.0, 0.1, 50.0);

/// Ray-test all lobby geometry (cubes, console, ground). Returns closest hit distance.
pub fn ray_test_lobby(origin: Vec3, dir: Vec3, max_dist: f32) -> Option<f32> {
    ray_test_lobby_with_normal(origin, dir, max_dist).map(|(d, _)| d)
}

/// Ray-test all lobby geometry. Returns closest hit distance and the surface normal.
pub fn ray_test_lobby_with_normal(origin: Vec3, dir: Vec3, max_dist: f32) -> Option<(f32, Vec3)> {
    let mut best: Option<(f32, Vec3)> = None;

    // console bounding box (pedestal + screen housing combined)
    let console_center = LOBBY_CONSOLE_POS + vec3(0.0, LOBBY_CONSOLE_SIZE.y / 2.0, 0.0);
    let console_half = LOBBY_CONSOLE_SIZE * 0.5;
    if let Some((d, n)) = ray_hit_aabb_with_normal(origin, dir, console_center, console_half, max_dist) {
        best = Some((d, n));
    }

    // landmark + parkour cubes
    for (center, size) in LANDMARK_CUBES.iter().chain(PARKOUR_CUBES.iter()) {
        let half = *size * 0.5;
        if let Some((d, n)) = ray_hit_aabb_with_normal(origin, dir, *center, half, max_dist) {
            if best.map_or(true, |(b, _)| d < b) {
                best = Some((d, n));
            }
        }
    }

    // ground plane
    let ground_half = GROUND_SIZE * 0.5;
    if let Some((d, n)) = ray_hit_aabb_with_normal(origin, dir, GROUND_CENTER, ground_half, max_dist) {
        if best.map_or(true, |(b, _)| d < b) {
            best = Some((d, n));
        }
    }

    if let Some((d, n)) = shooting_range::ray_test_shooting_range_with_normal(origin, dir, max_dist) {
        if best.map_or(true, |(b, _)| d < b) {
            best = Some((d, n));
        }
    }

    best
}

