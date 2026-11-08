use macroquad::prelude::*;

use crate::world::render::maze::MeshBuilder;

pub(super) fn draw_crate(out: &mut MeshBuilder, c: Vec3, lit: &impl Fn(Color, Vec3) -> Color) {
    // Wooden shipping crate with corner reinforcements and planks
    let wood_base = Color::new(0.42, 0.30, 0.18, 1.0);
    let wood_light = Color::new(0.52, 0.38, 0.24, 1.0);
    let metal = Color::new(0.25, 0.25, 0.25, 1.0);

    // Main body - smaller and tighter
    let pos = c + vec3(0.0, 0.35, 0.0);
    out.cube(pos, vec3(0.65, 0.70, 0.65), lit(wood_base, pos));

    // Plank details (horizontal bands)
    for offset in [0.15, 0.35, 0.55] {
        let pos = c + vec3(0.0, offset, 0.0);
        out.cube(pos, vec3(0.68, 0.08, 0.68), lit(wood_light, pos));
    }

    // Corner metal reinforcements
    for &(x, z) in &[(-0.28, -0.28), (0.28, -0.28), (-0.28, 0.28), (0.28, 0.28)] {
        let pos = c + vec3(x, 0.35, z);
        out.cube(pos, vec3(0.06, 0.75, 0.06), lit(metal, pos));
    }
}

pub(super) fn draw_barrel(out: &mut MeshBuilder, c: Vec3, lit: &impl Fn(Color, Vec3) -> Color) {
    // Industrial metal barrel with hoops - cylindrical appearance
    let barrel_body = Color::new(0.44, 0.24, 0.14, 1.0);
    let barrel_top = Color::new(0.36, 0.20, 0.12, 1.0);
    let metal_band = Color::new(0.28, 0.28, 0.28, 1.0);

    // Bottom segment
    let pos = c + vec3(0.0, 0.20, 0.0);
    out.cube(pos, vec3(0.52, 0.40, 0.52), lit(barrel_body, pos));
    // Bulge (middle wider)
    let pos = c + vec3(0.0, 0.45, 0.0);
    out.cube(pos, vec3(0.56, 0.30, 0.56), lit(barrel_body, pos));
    // Top segment
    let pos = c + vec3(0.0, 0.70, 0.0);
    out.cube(pos, vec3(0.52, 0.40, 0.52), lit(barrel_top, pos));

    // Metal hoops/bands
    for height in [0.25, 0.50, 0.75] {
        let pos = c + vec3(0.0, height, 0.0);
        out.cube(pos, vec3(0.58, 0.06, 0.58), lit(metal_band, pos));
    }

    // Lid with subtle rim
    let pos = c + vec3(0.0, 0.92, 0.0);
    out.cube(pos, vec3(0.50, 0.04, 0.50), lit(metal_band, pos));
}

pub(super) fn draw_supply_crate(out: &mut MeshBuilder, c: Vec3, lit: &impl Fn(Color, Vec3) -> Color) {
    // Futuristic military supply crate with tech details
    let crate_dark = Color::new(0.16, 0.20, 0.26, 1.0);
    let crate_panel = Color::new(0.24, 0.28, 0.34, 1.0);
    let cyan_glow = Color::new(0.08, 0.82, 0.92, 1.0);
    let cyan_bright = Color::new(0.15, 0.95, 1.0, 1.0);
    let corner_bracket = Color::new(0.45, 0.45, 0.48, 1.0); // Brighter metallic

    // Main body - compact and angular
    let pos = c + vec3(0.0, 0.35, 0.0);
    out.cube(pos, vec3(0.68, 0.70, 0.68), lit(crate_dark, pos));

    // Recessed panels on sides
    let pos = c + vec3(0.0, 0.40, 0.30);
    out.cube(pos, vec3(0.50, 0.50, 0.02), lit(crate_panel, pos));
    let pos = c + vec3(0.0, 0.40, -0.30);
    out.cube(pos, vec3(0.50, 0.50, 0.02), lit(crate_panel, pos));

    // Glowing power strip (center band) - emit light, less affected by lighting
    let pos = c + vec3(0.0, 0.40, 0.0);
    out.cube(
        pos,
        vec3(0.60, 0.12, 0.60),
        cyan_glow, // Emissive, keep full brightness
    );
    // Bright accent line
    out.cube(
        pos,
        vec3(0.58, 0.04, 0.58),
        cyan_bright, // Emissive, keep full brightness
    );

    // Corner brackets/locks - positioned outside the main body to avoid z-fighting
    for &(x, z) in &[(-0.35, -0.35), (0.35, -0.35), (-0.35, 0.35), (0.35, 0.35)] {
        // Lower bracket
        let pos1 = c + vec3(x, 0.10, z);
        out.cube(
            pos1,
            vec3(0.09, 0.20, 0.09),
            lit(corner_bracket, pos1),
        );

        // Upper bracket
        let pos2 = c + vec3(x, 0.60, z);
        out.cube(
            pos2,
            vec3(0.09, 0.20, 0.09),
            lit(corner_bracket, pos2),
        );
    }

    // Top lid with handle detail
    let pos = c + vec3(0.0, 0.72, 0.0);
    out.cube(pos, vec3(0.66, 0.04, 0.66), lit(crate_panel, pos));
    let pos = c + vec3(0.0, 0.77, 0.0);
    out.cube(pos, vec3(0.20, 0.06, 0.10), lit(corner_bracket, pos));
}
