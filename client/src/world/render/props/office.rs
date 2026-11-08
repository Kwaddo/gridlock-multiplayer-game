use macroquad::prelude::*;

use crate::world::render::maze::MeshBuilder;

pub(super) fn draw_lamp(out: &mut MeshBuilder, c: Vec3, lit: &impl Fn(Color, Vec3) -> Color) {
    // Stylized standing lamp with diffused light
    let pole = Color::new(0.18, 0.18, 0.20, 1.0);
    let base_metal = Color::new(0.28, 0.28, 0.30, 1.0);
    let shade = Color::new(0.15, 0.15, 0.18, 1.0);
    let bulb = Color::new(0.98, 0.96, 0.82, 1.0);
    let glow_outer = Color::new(0.98, 0.94, 0.70, 0.25);
    let glow_mid = Color::new(0.98, 0.92, 0.68, 0.15);

    // Heavy base plate
    let pos = c + vec3(0.0, 0.03, 0.0);
    out.cube(pos, vec3(0.32, 0.06, 0.32), lit(base_metal, pos));

    // Slender vertical pole
    let pos = c + vec3(0.0, 0.55, 0.0);
    out.cube(pos, vec3(0.08, 1.10, 0.08), lit(pole, pos));

    // Lamp shade/hood (inverted cone approximation)
    let pos = c + vec3(0.0, 1.08, 0.0);
    out.cube(pos, vec3(0.28, 0.08, 0.28), lit(shade, pos));
    let pos = c + vec3(0.0, 1.02, 0.0);
    out.cube(pos, vec3(0.22, 0.08, 0.22), lit(shade, pos));

    // Bright bulb/light source - emissive, keep full brightness
    out.cube(c + vec3(0.0, 0.98, 0.0), vec3(0.14, 0.14, 0.14), bulb);

    // Layered glow effect for ambient lighting - emissive
    out.cube(
        c + vec3(0.0, 0.50, 0.0),
        vec3(1.8, 0.02, 1.8),
        glow_outer,
    );
    out.cube(
        c + vec3(0.0, 0.70, 0.0),
        vec3(1.2, 0.02, 1.2),
        glow_mid,
    );
    out.cube(
        c + vec3(0.0, 0.98, 0.0),
        vec3(0.65, 0.02, 0.65),
        Color::new(1.0, 0.98, 0.85, 0.35),
    );
}

pub(super) fn draw_desk(out: &mut MeshBuilder, c: Vec3, lit: &impl Fn(Color, Vec3) -> Color) {
    // Clean white office desk spanning the full cell footprint
    let desk_white = Color::new(0.92, 0.92, 0.92, 1.0);
    let desk_shadow = Color::new(0.82, 0.82, 0.82, 1.0);
    let leg = Color::new(0.75, 0.75, 0.75, 1.0);

    // Desktop surface (slightly wider to visually fill the whole maze cell) - top surface at y=0.675 (center 0.65 + half-height 0.025)
    let pos = c + vec3(0.0, 0.65, 0.0);
    out.cube(pos, vec3(1.10, 0.05, 1.00), lit(desk_white, pos));

    // Underside shadow matching the larger top
    let pos = c + vec3(0.0, 0.61, 0.0);
    out.cube(pos, vec3(1.04, 0.02, 0.94), lit(desk_shadow, pos));

    // Four legs pushed to the corners of the full footprint
    for &(x, z) in &[(-0.47, -0.44), (0.47, -0.44), (-0.47, 0.44), (0.47, 0.44)] {
        let pos = c + vec3(x, 0.31, z);
        out.cube(pos, vec3(0.06, 0.62, 0.08), lit(leg, pos));
    }
}

/// `chairs`: an office chair in the cell above, below, left and right of
/// the monitor (cached when the props are baked).
pub(super) fn draw_monitor(
    out: &mut MeshBuilder,
    chairs: [bool; 4],
    c: Vec3,
    lit: &impl Fn(Color, Vec3) -> Color,
) {
    // Modern flat-screen monitor on desk (desk top is at y=0.675)
    let screen_frame = Color::new(0.12, 0.12, 0.14, 1.0);
    let screen_on = Color::new(0.18, 0.32, 0.48, 1.0);
    let screen_glow = Color::new(0.25, 0.45, 0.65, 0.15);
    let stand = Color::new(0.15, 0.15, 0.17, 1.0);

    let [chair_up, chair_down, chair_left, chair_right] = chairs;

    let (
        offset_x,
        offset_z,
        screen_delta_x,
        screen_delta_z,
        frame_size,
        screen_size,
        glow_size,
    ) = if chair_up {
        (
            0.0,
            -0.05,
            0.0,
            -0.03,
            vec3(0.30, 0.22, 0.03),
            vec3(0.26, 0.18, 0.01),
            vec3(0.30, 0.24, 0.01),
        )
    } else if chair_down {
        (
            0.0,
            0.05,
            0.0,
            0.03,
            vec3(0.30, 0.22, 0.03),
            vec3(0.26, 0.18, 0.01),
            vec3(0.30, 0.24, 0.01),
        )
    } else if chair_left {
        (
            -0.05,
            0.0,
            -0.03,
            0.0,
            vec3(0.03, 0.22, 0.30),
            vec3(0.01, 0.18, 0.26),
            vec3(0.01, 0.24, 0.30),
        )
    } else if chair_right {
        (
            0.05,
            0.0,
            0.03,
            0.0,
            vec3(0.03, 0.22, 0.30),
            vec3(0.01, 0.18, 0.26),
            vec3(0.01, 0.24, 0.30),
        )
    } else {
        (
            0.0,
            0.06,
            0.0,
            0.02,
            vec3(0.30, 0.22, 0.03),
            vec3(0.26, 0.18, 0.01),
            vec3(0.30, 0.24, 0.01),
        )
    };

    // Stand base sitting ON desk surface (desk top at 0.675)
    let pos = c + vec3(0.0, 0.685, 0.0);
    out.cube(pos, vec3(0.18, 0.02, 0.18), lit(stand, pos));

    // Stand pole from base up to monitor
    let pos = c + vec3(0.0, 0.82, 0.0);
    out.cube(pos, vec3(0.04, 0.28, 0.04), lit(stand, pos));

    // Monitor frame (positioned based on orientation)
    let pos = c + vec3(offset_x, 1.00, offset_z);
    out.cube(pos, frame_size, lit(screen_frame, pos));

    // Active screen (facing toward the chair) - emissive
    out.cube(
        c + vec3(
            offset_x + screen_delta_x * 0.25,
            1.00,
            offset_z + screen_delta_z * 0.25,
        ),
        screen_size,
        screen_on,
    );

    // Screen glow effect - emissive
    out.cube(
        c + vec3(offset_x + screen_delta_x, 1.00, offset_z + screen_delta_z),
        glow_size,
        screen_glow,
    );
}

/// `monitors`: a monitor in the cell above, below, left and right of the
/// chair (cached when the props are baked).
pub(super) fn draw_office_chair(
    out: &mut MeshBuilder,
    monitors: [bool; 4],
    c: Vec3,
    lit: &impl Fn(Color, Vec3) -> Color,
) {
    // Simple swivel office chair
    let seat_fabric = Color::new(0.22, 0.24, 0.28, 1.0);
    let seat_edge = Color::new(0.18, 0.20, 0.24, 1.0);
    let back_fabric = Color::new(0.24, 0.26, 0.30, 1.0);
    let base = Color::new(0.15, 0.15, 0.17, 1.0);

    let [monitor_up, monitor_down, monitor_left, monitor_right] = monitors;

    // Backrest sits opposite the monitor so the chair faces the screen; rotate slab based on axis
    let (backrest_x, backrest_z, support_x, support_z, backrest_size) = if monitor_up {
        (0.0, 0.18, 0.0, 0.16, vec3(0.28, 0.32, 0.06))
    } else if monitor_down {
        (0.0, -0.18, 0.0, -0.16, vec3(0.28, 0.32, 0.06))
    } else if monitor_left {
        (0.18, 0.0, 0.16, 0.0, vec3(0.06, 0.32, 0.28))
    } else if monitor_right {
        (-0.18, 0.0, -0.16, 0.0, vec3(0.06, 0.32, 0.28))
    } else {
        (0.0, 0.18, 0.0, 0.16, vec3(0.28, 0.32, 0.06))
    };

    // Five-star base (cross shape approximation)
    let pos1 = c + vec3(0.0, 0.05, 0.0);
    out.cube(pos1, vec3(0.30, 0.02, 0.05), lit(base, pos1));
    out.cube(pos1, vec3(0.05, 0.02, 0.30), lit(base, pos1));

    // Central pole
    let pos = c + vec3(0.0, 0.25, 0.0);
    out.cube(pos, vec3(0.05, 0.40, 0.05), lit(base, pos));

    // Seat cushion
    let pos = c + vec3(0.0, 0.48, 0.0);
    out.cube(pos, vec3(0.30, 0.08, 0.30), lit(seat_fabric, pos));
    let pos = c + vec3(0.0, 0.44, 0.0);
    out.cube(pos, vec3(0.32, 0.02, 0.32), lit(seat_edge, pos));

    // Backrest (positioned based on orientation)
    let pos = c + vec3(backrest_x, 0.68, backrest_z);
    out.cube(pos, backrest_size, lit(back_fabric, pos));

    // Backrest support
    let pos = c + vec3(support_x, 0.56, support_z);
    out.cube(pos, vec3(0.04, 0.20, 0.04), lit(base, pos));
}
