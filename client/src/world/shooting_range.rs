use crate::weapons::combat::ray_hit_aabb_with_normal;
use macroquad::prelude::*;

pub const RANGE_EXT_CENTER: Vec3 = vec3(33.0, -0.05, 0.0);
pub const RANGE_EXT_SIZE: Vec3 = vec3(16.0, 0.1, 20.0);

// Knee-height firing line barrier.
pub const RANGE_LINE_CENTER: Vec3 = vec3(27.0, 0.22, 0.0);
pub const RANGE_LINE_SIZE: Vec3 = vec3(0.25, 0.44, 16.0);

pub const RANGE_SIDE_LEFT_CENTER: Vec3 = vec3(33.0, 1.0, -10.0);
pub const RANGE_SIDE_RIGHT_CENTER: Vec3 = vec3(33.0, 1.0, 10.0);
pub const RANGE_SIDE_SIZE: Vec3 = vec3(16.0, 2.0, 0.3);

pub const RANGE_BACK_CENTER: Vec3 = vec3(41.0, 1.0, 0.0);
pub const RANGE_BACK_SIZE: Vec3 = vec3(0.3, 2.0, 20.0);

pub const RANGE_START_BUTTON_POS: Vec3 = vec3(26.0, 0.0, 9.0);
pub const RANGE_START_BUTTON_SIZE: Vec3 = vec3(0.6, 1.0, 0.6);

pub const SHOOTING_RANGE_COLLIDERS: [(Vec3, Vec3); 5] = [
    (RANGE_LINE_CENTER, RANGE_LINE_SIZE),
    (RANGE_SIDE_LEFT_CENTER, RANGE_SIDE_SIZE),
    (RANGE_SIDE_RIGHT_CENTER, RANGE_SIDE_SIZE),
    (RANGE_BACK_CENTER, RANGE_BACK_SIZE),
    (
        vec3(
            RANGE_START_BUTTON_POS.x,
            RANGE_START_BUTTON_POS.y + RANGE_START_BUTTON_SIZE.y * 0.5,
            RANGE_START_BUTTON_POS.z,
        ),
        RANGE_START_BUTTON_SIZE,
    ),
];

pub fn draw_shooting_range(floor_texture: Option<&Texture2D>) {
    draw_cube(RANGE_EXT_CENTER, RANGE_EXT_SIZE, floor_texture, DARKGRAY);

    draw_cube(
        RANGE_LINE_CENTER,
        RANGE_LINE_SIZE,
        None,
        Color::from_rgba(90, 95, 105, 255),
    );
    draw_cube(
        RANGE_SIDE_LEFT_CENTER,
        RANGE_SIDE_SIZE,
        None,
        Color::from_rgba(70, 75, 85, 255),
    );
    draw_cube(
        RANGE_SIDE_RIGHT_CENTER,
        RANGE_SIDE_SIZE,
        None,
        Color::from_rgba(70, 75, 85, 255),
    );
    draw_cube(
        RANGE_BACK_CENTER,
        RANGE_BACK_SIZE,
        None,
        Color::from_rgba(80, 85, 95, 255),
    );

    draw_cube_wires(RANGE_LINE_CENTER, RANGE_LINE_SIZE, BLACK);
    draw_cube_wires(RANGE_SIDE_LEFT_CENTER, RANGE_SIDE_SIZE, BLACK);
    draw_cube_wires(RANGE_SIDE_RIGHT_CENTER, RANGE_SIDE_SIZE, BLACK);
    draw_cube_wires(RANGE_BACK_CENTER, RANGE_BACK_SIZE, BLACK);

    // Start button pedestal near the line.
    let button_center = vec3(
        RANGE_START_BUTTON_POS.x,
        RANGE_START_BUTTON_POS.y + RANGE_START_BUTTON_SIZE.y * 0.5,
        RANGE_START_BUTTON_POS.z,
    );
    draw_cube(
        button_center,
        RANGE_START_BUTTON_SIZE,
        None,
        Color::from_rgba(40, 45, 55, 255),
    );
    draw_cube_wires(button_center, RANGE_START_BUTTON_SIZE, BLACK);
    draw_cube(
        button_center + vec3(0.0, 0.35, 0.0),
        vec3(0.36, 0.10, 0.36),
        None,
        Color::from_rgba(255, 107, 53, 255),
    );
}

pub fn ray_test_shooting_range_with_normal(
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
) -> Option<(f32, Vec3)> {
    let mut best: Option<(f32, Vec3)> = None;
    for (center, size) in [(RANGE_EXT_CENTER, RANGE_EXT_SIZE)]
        .into_iter()
        .chain(SHOOTING_RANGE_COLLIDERS)
    {
        let half = size * 0.5;
        if let Some((d, n)) = ray_hit_aabb_with_normal(origin, dir, center, half, max_dist) {
            if best.map_or(true, |(b, _)| d < b) {
                best = Some((d, n));
            }
        }
    }
    best
}
