use macroquad::prelude::*;

use crate::types::maze::{Maze, PropKind};

pub(super) fn draw_props(maze: &Maze) {
    for prop in &maze.props {
        let c = maze.cell_to_world(prop.x as i32, prop.y as i32);
        match prop.kind {
            PropKind::Crate => {
                // Wooden shipping crate with corner reinforcements and planks
                let wood_base = Color::new(0.42, 0.30, 0.18, 1.0);
                let wood_light = Color::new(0.52, 0.38, 0.24, 1.0);
                let metal = Color::new(0.25, 0.25, 0.25, 1.0);

                // Main body - smaller and tighter
                draw_cube(
                    c + vec3(0.0, 0.35, 0.0),
                    vec3(0.65, 0.70, 0.65),
                    None,
                    wood_base,
                );

                // Plank details (horizontal bands)
                for offset in [0.15, 0.35, 0.55] {
                    draw_cube(
                        c + vec3(0.0, offset, 0.0),
                        vec3(0.68, 0.08, 0.68),
                        None,
                        wood_light,
                    );
                }

                // Corner metal reinforcements
                for &(x, z) in &[(-0.28, -0.28), (0.28, -0.28), (-0.28, 0.28), (0.28, 0.28)] {
                    draw_cube(c + vec3(x, 0.35, z), vec3(0.06, 0.75, 0.06), None, metal);
                }
            }
            PropKind::Barrel => {
                // Industrial metal barrel with hoops - cylindrical appearance
                let barrel_body = Color::new(0.44, 0.24, 0.14, 1.0);
                let barrel_top = Color::new(0.36, 0.20, 0.12, 1.0);
                let metal_band = Color::new(0.28, 0.28, 0.28, 1.0);

                // Bottom segment
                draw_cube(
                    c + vec3(0.0, 0.20, 0.0),
                    vec3(0.52, 0.40, 0.52),
                    None,
                    barrel_body,
                );
                // Bulge (middle wider)
                draw_cube(
                    c + vec3(0.0, 0.45, 0.0),
                    vec3(0.56, 0.30, 0.56),
                    None,
                    barrel_body,
                );
                // Top segment
                draw_cube(
                    c + vec3(0.0, 0.70, 0.0),
                    vec3(0.52, 0.40, 0.52),
                    None,
                    barrel_top,
                );

                // Metal hoops/bands
                for height in [0.25, 0.50, 0.75] {
                    draw_cube(
                        c + vec3(0.0, height, 0.0),
                        vec3(0.58, 0.06, 0.58),
                        None,
                        metal_band,
                    );
                }

                // Lid with subtle rim
                draw_cube(
                    c + vec3(0.0, 0.92, 0.0),
                    vec3(0.50, 0.04, 0.50),
                    None,
                    metal_band,
                );
            }
            PropKind::SupplyCrate => {
                // Futuristic military supply crate with tech details
                let crate_dark = Color::new(0.16, 0.20, 0.26, 1.0);
                let crate_panel = Color::new(0.24, 0.28, 0.34, 1.0);
                let cyan_glow = Color::new(0.08, 0.82, 0.92, 1.0);
                let cyan_bright = Color::new(0.15, 0.95, 1.0, 1.0);
                let corner_bracket = Color::new(0.35, 0.35, 0.38, 1.0);

                // Main body - compact and angular
                draw_cube(
                    c + vec3(0.0, 0.35, 0.0),
                    vec3(0.68, 0.70, 0.68),
                    None,
                    crate_dark,
                );

                // Recessed panels on sides
                draw_cube(
                    c + vec3(0.0, 0.40, 0.30),
                    vec3(0.50, 0.50, 0.02),
                    None,
                    crate_panel,
                );
                draw_cube(
                    c + vec3(0.0, 0.40, -0.30),
                    vec3(0.50, 0.50, 0.02),
                    None,
                    crate_panel,
                );

                // Glowing power strip (center band)
                draw_cube(
                    c + vec3(0.0, 0.40, 0.0),
                    vec3(0.60, 0.12, 0.60),
                    None,
                    cyan_glow,
                );
                // Bright accent line
                draw_cube(
                    c + vec3(0.0, 0.40, 0.0),
                    vec3(0.58, 0.04, 0.58),
                    None,
                    cyan_bright,
                );

                // Corner brackets/locks
                for &(x, z) in &[(-0.30, -0.30), (0.30, -0.30), (-0.30, 0.30), (0.30, 0.30)] {
                    draw_cube(
                        c + vec3(x, 0.10, z),
                        vec3(0.08, 0.18, 0.08),
                        None,
                        corner_bracket,
                    );
                    draw_cube(
                        c + vec3(x, 0.60, z),
                        vec3(0.08, 0.18, 0.08),
                        None,
                        corner_bracket,
                    );
                }

                // Top lid with handle detail
                draw_cube(
                    c + vec3(0.0, 0.72, 0.0),
                    vec3(0.66, 0.04, 0.66),
                    None,
                    crate_panel,
                );
                draw_cube(
                    c + vec3(0.0, 0.77, 0.0),
                    vec3(0.20, 0.06, 0.10),
                    None,
                    corner_bracket,
                );
            }
            PropKind::Lamp => {
                // Stylized standing lamp with diffused light
                let pole = Color::new(0.18, 0.18, 0.20, 1.0);
                let base_metal = Color::new(0.28, 0.28, 0.30, 1.0);
                let shade = Color::new(0.15, 0.15, 0.18, 1.0);
                let bulb = Color::new(0.98, 0.96, 0.82, 1.0);
                let glow_outer = Color::new(0.98, 0.94, 0.70, 0.25);
                let glow_mid = Color::new(0.98, 0.92, 0.68, 0.15);

                // Heavy base plate
                draw_cube(
                    c + vec3(0.0, 0.03, 0.0),
                    vec3(0.32, 0.06, 0.32),
                    None,
                    base_metal,
                );

                // Slender vertical pole
                draw_cube(c + vec3(0.0, 0.55, 0.0), vec3(0.08, 1.10, 0.08), None, pole);

                // Lamp shade/hood (inverted cone approximation)
                draw_cube(
                    c + vec3(0.0, 1.08, 0.0),
                    vec3(0.28, 0.08, 0.28),
                    None,
                    shade,
                );
                draw_cube(
                    c + vec3(0.0, 1.02, 0.0),
                    vec3(0.22, 0.08, 0.22),
                    None,
                    shade,
                );

                // Bright bulb/light source
                draw_cube(c + vec3(0.0, 0.98, 0.0), vec3(0.14, 0.14, 0.14), None, bulb);

                // Layered glow effect for ambient lighting
                draw_cube(
                    c + vec3(0.0, 0.50, 0.0),
                    vec3(1.8, 0.02, 1.8),
                    None,
                    glow_outer,
                );
                draw_cube(
                    c + vec3(0.0, 0.70, 0.0),
                    vec3(1.2, 0.02, 1.2),
                    None,
                    glow_mid,
                );
                draw_cube(
                    c + vec3(0.0, 0.98, 0.0),
                    vec3(0.65, 0.02, 0.65),
                    None,
                    Color::new(1.0, 0.98, 0.85, 0.35),
                );
            }
            PropKind::Desk => {
                // Clean white office desk (now square to match grid footprint)
                let desk_white = Color::new(0.92, 0.92, 0.92, 1.0);
                let desk_shadow = Color::new(0.82, 0.82, 0.82, 1.0);
                let leg = Color::new(0.75, 0.75, 0.75, 1.0);

                // Desktop surface (square slab) - top surface at y=0.675 (center 0.65 + half-height 0.025)
                draw_cube(
                    c + vec3(0.0, 0.65, 0.0),
                    vec3(0.70, 0.05, 0.70),
                    None,
                    desk_white,
                );

                // Underside shadow
                draw_cube(
                    c + vec3(0.0, 0.61, 0.0),
                    vec3(0.66, 0.02, 0.66),
                    None,
                    desk_shadow,
                );

                // Four slim legs (height 0.62 to reach just below desktop)
                for &(x, z) in &[(-0.30, -0.30), (0.30, -0.30), (-0.30, 0.30), (0.30, 0.30)] {
                    draw_cube(c + vec3(x, 0.31, z), vec3(0.06, 0.62, 0.06), None, leg);
                }
            }
            PropKind::Monitor => {
                // Modern flat-screen monitor on desk (desk top is at y=0.675)
                let screen_frame = Color::new(0.12, 0.12, 0.14, 1.0);
                let screen_on = Color::new(0.18, 0.32, 0.48, 1.0);
                let screen_glow = Color::new(0.25, 0.45, 0.65, 0.15);
                let stand = Color::new(0.15, 0.15, 0.17, 1.0);

                let px = prop.x as i32;
                let py = prop.y as i32;
                let has_chair = |dx: i32, dy: i32| -> bool {
                    let nx = px + dx;
                    let ny = py + dy;
                    if nx < 0 || ny < 0 {
                        return false;
                    }
                    maze.props.iter().any(|p| {
                        p.x as i32 == nx
                            && p.y as i32 == ny
                            && matches!(p.kind, PropKind::OfficeChair)
                    })
                };

                let chair_up = has_chair(0, -1);
                let chair_down = has_chair(0, 1);
                let chair_left = has_chair(-1, 0);
                let chair_right = has_chair(1, 0);

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
                draw_cube(
                    c + vec3(0.0, 0.685, 0.0),
                    vec3(0.18, 0.02, 0.18),
                    None,
                    stand,
                );

                // Stand pole from base up to monitor
                draw_cube(
                    c + vec3(0.0, 0.82, 0.0),
                    vec3(0.04, 0.28, 0.04),
                    None,
                    stand,
                );

                // Monitor frame (positioned based on orientation)
                draw_cube(
                    c + vec3(offset_x, 1.00, offset_z),
                    frame_size,
                    None,
                    screen_frame,
                );

                // Active screen (facing toward the chair)
                draw_cube(
                    c + vec3(
                        offset_x + screen_delta_x * 0.25,
                        1.00,
                        offset_z + screen_delta_z * 0.25,
                    ),
                    screen_size,
                    None,
                    screen_on,
                );

                // Screen glow effect
                draw_cube(
                    c + vec3(offset_x + screen_delta_x, 1.00, offset_z + screen_delta_z),
                    glow_size,
                    None,
                    screen_glow,
                );
            }
            PropKind::OfficeChair => {
                // Simple swivel office chair
                let seat_fabric = Color::new(0.22, 0.24, 0.28, 1.0);
                let seat_edge = Color::new(0.18, 0.20, 0.24, 1.0);
                let back_fabric = Color::new(0.24, 0.26, 0.30, 1.0);
                let base = Color::new(0.15, 0.15, 0.17, 1.0);

                let px = prop.x as i32;
                let py = prop.y as i32;
                let has_monitor = |dx: i32, dy: i32| -> bool {
                    let nx = px + dx;
                    let ny = py + dy;
                    if nx < 0 || ny < 0 {
                        return false;
                    }
                    maze.props.iter().any(|p| {
                        p.x as i32 == nx && p.y as i32 == ny && matches!(p.kind, PropKind::Monitor)
                    })
                };

                let monitor_up = has_monitor(0, -1);
                let monitor_down = has_monitor(0, 1);
                let monitor_left = has_monitor(-1, 0);
                let monitor_right = has_monitor(1, 0);

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
                draw_cube(c + vec3(0.0, 0.05, 0.0), vec3(0.30, 0.02, 0.05), None, base);
                draw_cube(c + vec3(0.0, 0.05, 0.0), vec3(0.05, 0.02, 0.30), None, base);

                // Central pole
                draw_cube(c + vec3(0.0, 0.25, 0.0), vec3(0.05, 0.40, 0.05), None, base);

                // Seat cushion
                draw_cube(
                    c + vec3(0.0, 0.48, 0.0),
                    vec3(0.30, 0.08, 0.30),
                    None,
                    seat_fabric,
                );
                draw_cube(
                    c + vec3(0.0, 0.44, 0.0),
                    vec3(0.32, 0.02, 0.32),
                    None,
                    seat_edge,
                );

                // Backrest (positioned based on orientation)
                draw_cube(
                    c + vec3(backrest_x, 0.68, backrest_z),
                    backrest_size,
                    None,
                    back_fabric,
                );

                // Backrest support
                draw_cube(
                    c + vec3(support_x, 0.56, support_z),
                    vec3(0.04, 0.20, 0.04),
                    None,
                    base,
                );
            }
        }
    }
}
