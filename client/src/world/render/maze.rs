use macroquad::prelude::*;

use crate::types::maze::Maze;

use super::props::draw_props;
use super::utils::{apply_lighting, tint_color, CeilingLight};

pub fn draw_maze(maze: Option<&Maze>, viewer_pos: Vec3, render_distance: f32) {
    if let Some(m) = maze {
        let render_dist_sq = render_distance * render_distance;
        let padded_dist_sq =
            (render_distance + m.cell_size * 1.5) * (render_distance + m.cell_size * 1.5);

        // Distance-based LOD toggles to trim tiny decals and hashes on distant cells.
        let detail_far_sq = render_dist_sq * 0.82; // keep most mid/large decals
        let fine_detail_sq = render_dist_sq * 0.58; // cracks, chips, grime
        let micro_detail_sq = render_dist_sq * 0.40; // thinnest lines only when close

        // Muted tile floor and chipped plaster walls to fit an abandoned school look.
        let tile_light = Color::new(0.78, 0.76, 0.72, 1.0);
        let tile_dark = Color::new(0.70, 0.68, 0.64, 1.0);
        let grime = Color::new(0.35, 0.33, 0.31, 0.35);
        let plaster = Color::new(0.82, 0.84, 0.86, 1.0);
        let plaster_shadow = Color::new(0.74, 0.76, 0.78, 1.0);
        let baseboard = Color::new(0.24, 0.25, 0.28, 1.0);
        let accent_stripe = Color::new(0.38, 0.46, 0.54, 1.0);
        let brick = Color::new(0.63, 0.55, 0.50, 1.0);

        let map_w = m.width as f32 * m.cell_size;
        let map_h = m.height as f32 * m.cell_size;

        let ceiling_lights: Vec<CeilingLight> = m
            .ceiling_lights
            .iter()
            .map(|info| {
                let color = match info.variant % 3 {
                    0 => vec3(0.95, 0.94, 0.88),
                    1 => vec3(1.0, 0.95, 0.85),
                    _ => vec3(0.88, 0.92, 0.98),
                };
                CeilingLight {
                    pos: m.cell_to_world(info.x as i32, info.y as i32)
                        + vec3(0.0, m.wall_height - 0.15, 0.0),
                    color,
                    intensity: 0.35,
                }
            })
            .collect();
        let apply_light = |c: Color, p: Vec3| -> Color { apply_lighting(&ceiling_lights, c, p) };

        let center_cx = (viewer_pos.x / m.cell_size).floor() as i32;
        let center_cy = (viewer_pos.z / m.cell_size).floor() as i32;
        let radius_cells = (render_distance / m.cell_size).ceil() as i32 + 2; // small padding to reduce pop
        let start_x = (center_cx - radius_cells).max(0) as usize;
        let end_x = (center_cx + radius_cells + 1).min(m.width as i32) as usize;
        let start_y = (center_cy - radius_cells).max(0) as usize;
        let end_y = (center_cy + radius_cells + 1).min(m.height as i32) as usize;

        for y in start_y..end_y {
            for x in start_x..end_x {
                let center = vec3(
                    (x as f32 + 0.5) * m.cell_size,
                    0.0,
                    (y as f32 + 0.5) * m.cell_size,
                );

                let dx = center.x - viewer_pos.x;
                let dz = center.z - viewer_pos.z;
                let dist_sq = dx * dx + dz * dz;
                if dist_sq > padded_dist_sq {
                    continue;
                }

                // LOD switches
                let allow_detail = dist_sq < detail_far_sq;
                let allow_fine = dist_sq < fine_detail_sq;
                let allow_micro = dist_sq < micro_detail_sq;

                // Checker tile floor with light wear and occasional grime overlays.
                let is_dark_tile = ((x + y) & 1) == 0;
                let base_tile = if is_dark_tile { tile_dark } else { tile_light };
                let floor_tint = tint_color(base_tile, x as i32, y as i32, 0x1, 0.02, 0.01);

                // Main tile slab (overlaps wall foot slightly to hide cracks)
                draw_cube(
                    center + vec3(0.0, -0.02, 0.0),
                    vec3(m.cell_size * 1.0, 0.08, m.cell_size * 1.0),
                    None,
                    apply_light(floor_tint, center + vec3(0.0, 0.01, 0.0)),
                );

                // Slightly darker edge to fake grout and depth, deeper and wider
                let grout = Color::new(0.26, 0.27, 0.28, 1.0);
                draw_cube(
                    center + vec3(0.0, -0.08, 0.0),
                    vec3(m.cell_size * 1.03, 0.05, m.cell_size * 1.03),
                    None,
                    apply_light(grout, center - vec3(0.0, 0.04, 0.0)),
                );

                // Add thin cracks across some tiles
                let crack_hash =
                    ((x as u32).wrapping_mul(0x456789) ^ (y as u32).wrapping_mul(0xABCDEF)) & 0x1F;
                if allow_micro && crack_hash < 3 {
                    let crack_color = Color::new(0.18, 0.18, 0.19, 1.0);
                    let crack_dir = crack_hash % 2;
                    if crack_dir == 0 {
                        // Horizontal crack
                        draw_cube(
                            center + vec3(0.0, -0.005, 0.0),
                            vec3(m.cell_size * 0.85, 0.005, 0.015),
                            None,
                            apply_light(crack_color, center),
                        );
                    } else {
                        // Vertical crack
                        draw_cube(
                            center + vec3(0.0, -0.005, 0.0),
                            vec3(0.015, 0.005, m.cell_size * 0.85),
                            None,
                            apply_light(crack_color, center),
                        );
                    }
                }

                // Add chipped corners on some tiles
                let chip_hash = ((x as u32).wrapping_mul(0x9E3779B9)
                    ^ (y as u32).wrapping_mul(0x7F4A7C15))
                    & 0x3F;
                if allow_fine && chip_hash < 5 {
                    let chip_color = Color::new(0.62, 0.60, 0.56, 1.0);
                    let corner = chip_hash % 4;
                    let offset = match corner {
                        0 => vec3(-m.cell_size * 0.35, -0.01, -m.cell_size * 0.35),
                        1 => vec3(m.cell_size * 0.35, -0.01, -m.cell_size * 0.35),
                        2 => vec3(-m.cell_size * 0.35, -0.01, m.cell_size * 0.35),
                        _ => vec3(m.cell_size * 0.35, -0.01, m.cell_size * 0.35),
                    };
                    draw_cube(
                        center + offset,
                        vec3(0.12, 0.04, 0.12),
                        None,
                        apply_light(chip_color, center + offset),
                    );
                }

                // Sporadic grime patches to sell neglect without many draw calls.
                let grime_noise = tint_color(grime, x as i32, y as i32, 0x5, 0.06, 0.0);
                if allow_detail
                    && (((x as u32).wrapping_mul(73856093) ^ (y as u32).wrapping_mul(19349663))
                        & 0x7)
                        == 0
                {
                    draw_cube(
                        center + vec3(0.0, -0.012, 0.0),
                        vec3(m.cell_size * 0.48, 0.01, m.cell_size * 0.48),
                        None,
                        apply_light(grime_noise, center),
                    );
                }

                // Larger dirt accumulation in corners
                let corner_dirt_hash =
                    ((x as u32).wrapping_mul(0x1ED271) ^ (y as u32).wrapping_mul(0x2C9277)) & 0x1F;
                if allow_detail && corner_dirt_hash < 4 {
                    let dirt = Color::new(0.28, 0.27, 0.26, 0.8);
                    let corner_offset = match corner_dirt_hash % 4 {
                        0 => vec3(-m.cell_size * 0.3, -0.015, -m.cell_size * 0.3),
                        1 => vec3(m.cell_size * 0.3, -0.015, -m.cell_size * 0.3),
                        2 => vec3(-m.cell_size * 0.3, -0.015, m.cell_size * 0.3),
                        _ => vec3(m.cell_size * 0.3, -0.015, m.cell_size * 0.3),
                    };
                    draw_cube(
                        center + corner_offset,
                        vec3(0.20, 0.005, 0.20),
                        None,
                        apply_light(dirt, center),
                    );
                }

                // Scuffed strip where lockers would have stood; every few rows only.
                if allow_detail && y % 6 == 0 {
                    draw_cube(
                        center + vec3(0.0, -0.02, -m.cell_size * 0.35),
                        vec3(m.cell_size * 0.95, 0.009, m.cell_size * 0.26),
                        None,
                        apply_light(
                            tint_color(grime, x as i32, y as i32, 0x9, 0.04, 0.0),
                            center,
                        ),
                    );
                }

                // Persistent cracked rubble decal where breakable walls were destroyed
                let cx_u8 = x as u8;
                let cy_u8 = y as u8;

                if m.is_broken_wall(cx_u8, cy_u8) {
                    let rubble_base = Color::new(0.42, 0.40, 0.37, 1.0);
                    let rubble_dark = Color::new(0.20, 0.19, 0.18, 1.0);

                    draw_cube(
                        center + vec3(0.0, -0.005, 0.0),
                        vec3(m.cell_size * 0.78, 0.015, m.cell_size * 0.78),
                        None,
                        apply_light(rubble_base, center),
                    );

                    // Main cross cracks
                    draw_cube(
                        center + vec3(0.0, -0.001, 0.0),
                        vec3(m.cell_size * 0.66, 0.01, 0.03),
                        None,
                        apply_light(rubble_dark, center),
                    );
                    draw_cube(
                        center + vec3(0.0, -0.001, 0.0),
                        vec3(0.03, 0.01, m.cell_size * 0.66),
                        None,
                        apply_light(rubble_dark, center),
                    );

                    // Diagonal fracture hints
                    draw_cube(
                        center + vec3(m.cell_size * 0.18, -0.001, m.cell_size * 0.14),
                        vec3(0.22, 0.01, 0.025),
                        None,
                        apply_light(rubble_dark, center),
                    );
                    draw_cube(
                        center + vec3(-m.cell_size * 0.16, -0.001, -m.cell_size * 0.20),
                        vec3(0.18, 0.01, 0.025),
                        None,
                        apply_light(rubble_dark, center),
                    );
                }

                if m.cells[m.idx(x as usize, y as usize)] == 0 {
                    // Check if this is a breakable wall
                    let is_breakable = m.is_breakable(cx_u8, cy_u8);
                    let health_pct = if is_breakable {
                        m.wall_health_percent(cx_u8, cy_u8).unwrap_or(1.0)
                    } else {
                        1.0
                    };

                    // Single plaster wall body, sunk slightly to overlap floor and hide cracks.
                    // Breakable walls have slightly different tint
                    let base_color = if is_breakable {
                        tint_color(
                            Color::new(0.86, 0.83, 0.74, 1.0), // warmer/lighter breakable tone
                            x as i32,
                            y as i32,
                            0x3,
                            0.030,
                            0.01,
                        )
                    } else {
                        tint_color(plaster, x as i32, y as i32, 0x3, 0.035, 0.015)
                    };

                    let wall_body = base_color;
                    let wall_pos = center + vec3(0.0, m.wall_height * 0.5 - 0.02, 0.0);
                    draw_cube(
                        wall_pos,
                        vec3(m.cell_size, m.wall_height + 0.04, m.cell_size),
                        None,
                        apply_light(wall_body, wall_pos),
                    );

                    // Broad mottled plaster variation to avoid flat walls
                    if allow_detail {
                        let mottled = tint_color(
                            Color::new(0.70, 0.71, 0.73, 1.0),
                            x as i32,
                            y as i32,
                            0x31,
                            0.055,
                            0.02,
                        );
                        draw_cube(
                            center + vec3(0.0, m.wall_height * 0.55, 0.0),
                            vec3(m.cell_size * 0.94, m.wall_height * 0.26, m.cell_size * 0.94),
                            None,
                            apply_light(mottled, center + vec3(0.0, m.wall_height * 0.55, 0.0)),
                        );
                    }

                    // Distinct top/bottom bands to ensure non-flat color breakup on every wall
                    if allow_detail {
                        let upper_band_col = tint_color(
                            Color::new(0.88, 0.89, 0.90, 1.0),
                            x as i32,
                            y as i32,
                            0x77,
                            0.04,
                            0.01,
                        );
                        let lower_band_col = tint_color(
                            Color::new(0.62, 0.64, 0.67, 1.0),
                            x as i32,
                            y as i32,
                            0x79,
                            0.04,
                            0.01,
                        );
                        draw_cube(
                            center + vec3(0.0, m.wall_height * 0.78, 0.0),
                            vec3(m.cell_size * 0.93, m.wall_height * 0.18, m.cell_size * 0.93),
                            None,
                            apply_light(
                                upper_band_col,
                                center + vec3(0.0, m.wall_height * 0.78, 0.0),
                            ),
                        );
                        draw_cube(
                            center + vec3(0.0, m.wall_height * 0.18, 0.0),
                            vec3(m.cell_size * 0.93, m.wall_height * 0.16, m.cell_size * 0.93),
                            None,
                            apply_light(
                                lower_band_col,
                                center + vec3(0.0, m.wall_height * 0.18, 0.0),
                            ),
                        );
                    }

                    // Hairline horizontal seam to break the silhouette further
                    if allow_detail {
                        draw_cube(
                            center + vec3(0.0, m.wall_height * 0.68, 0.0),
                            vec3(m.cell_size * 0.95, 0.012, m.cell_size * 0.95),
                            None,
                            apply_light(
                                Color::new(0.58, 0.60, 0.62, 1.0),
                                center + vec3(0.0, m.wall_height * 0.68, 0.0),
                            ),
                        );
                    }

                    // Breakable walls always show cracks; damage increases severity
                    if is_breakable {
                        let crack_color = Color::new(0.08, 0.08, 0.08, 1.0);
                        let warning = Color::new(0.86, 0.70, 0.22, 1.0);
                        let warning_dark = Color::new(0.22, 0.18, 0.10, 1.0);
                        let damage_stage = if health_pct > 0.80 {
                            1
                        } else if health_pct > 0.40 {
                            2
                        } else {
                            3
                        };

                        // Draw damage cracks based on stage (only when close enough to see them)
                        if allow_fine {
                            for crack_idx in 0..damage_stage {
                                let crack_hash = (x as u32)
                                    .wrapping_mul(11)
                                    .wrapping_add(y as u32)
                                    .wrapping_mul(31)
                                    .wrapping_add(crack_idx as u32);
                                let crack_x_offset =
                                    ((crack_hash % 70) as f32 / 100.0 - 0.35) * m.cell_size;
                                let crack_y_offset =
                                    (((crack_hash >> 8) % 60) as f32 / 100.0) * m.wall_height;

                                // Front and side crack strokes so cracks are visible from more angles
                                draw_cube(
                                    center
                                        + vec3(crack_x_offset, crack_y_offset, m.cell_size * 0.49),
                                    vec3(0.26, 0.016, 0.018),
                                    None,
                                    apply_light(
                                        crack_color,
                                        center
                                            + vec3(
                                                crack_x_offset,
                                                crack_y_offset,
                                                m.cell_size * 0.49,
                                            ),
                                    ),
                                );
                                draw_cube(
                                    center
                                        + vec3(m.cell_size * 0.49, crack_y_offset, crack_x_offset),
                                    vec3(0.018, 0.016, 0.26),
                                    None,
                                    apply_light(
                                        crack_color,
                                        center
                                            + vec3(
                                                m.cell_size * 0.49,
                                                crack_y_offset,
                                                crack_x_offset,
                                            ),
                                    ),
                                );

                                // Add perpendicular crack line
                                let perp_length = if health_pct < 0.33 { 0.34 } else { 0.20 };
                                draw_cube(
                                    center
                                        + vec3(
                                            crack_x_offset,
                                            crack_y_offset + 0.05,
                                            m.cell_size * 0.49,
                                        ),
                                    vec3(0.016, perp_length, 0.018),
                                    None,
                                    apply_light(
                                        crack_color,
                                        center
                                            + vec3(
                                                crack_x_offset,
                                                crack_y_offset + 0.05,
                                                m.cell_size * 0.49,
                                            ),
                                    ),
                                );
                            }
                        }

                        // Landmark "already broken-looking" crack at center for clear breakable signaling
                        draw_cube(
                            center + vec3(0.0, m.wall_height * 0.45, m.cell_size * 0.49),
                            vec3(m.cell_size * 0.55, 0.02, 0.02),
                            None,
                            apply_light(
                                Color::new(0.05, 0.05, 0.05, 1.0),
                                center + vec3(0.0, m.wall_height * 0.45, m.cell_size * 0.49),
                            ),
                        );

                        // Clear breakable-wall identity: warning brace bands on all four faces
                        let brace_y = m.wall_height * 0.62;
                        draw_cube(
                            center + vec3(0.0, brace_y, m.cell_size * 0.49),
                            vec3(m.cell_size * 0.62, 0.05, 0.02),
                            None,
                            warning,
                        );
                        draw_cube(
                            center + vec3(0.0, brace_y - 0.08, m.cell_size * 0.49),
                            vec3(m.cell_size * 0.50, 0.03, 0.02),
                            None,
                            warning_dark,
                        );
                        draw_cube(
                            center + vec3(0.0, brace_y, -m.cell_size * 0.49),
                            vec3(m.cell_size * 0.62, 0.05, 0.02),
                            None,
                            warning,
                        );
                        draw_cube(
                            center + vec3(m.cell_size * 0.49, brace_y, 0.0),
                            vec3(0.02, 0.05, m.cell_size * 0.62),
                            None,
                            warning,
                        );
                        draw_cube(
                            center + vec3(-m.cell_size * 0.49, brace_y, 0.0),
                            vec3(0.02, 0.05, m.cell_size * 0.62),
                            None,
                            warning,
                        );
                    }

                    // Always-on panel seams to break up the flat surface on every wall
                    let seam_color = Color::new(0.58, 0.60, 0.63, 1.0);
                    draw_cube(
                        center + vec3(-m.cell_size * 0.48, m.wall_height * 0.5, 0.0),
                        vec3(0.014, m.wall_height * 0.92, m.cell_size * 0.98),
                        None,
                        apply_light(
                            seam_color,
                            center + vec3(-m.cell_size * 0.48, m.wall_height * 0.5, 0.0),
                        ),
                    );
                    draw_cube(
                        center + vec3(m.cell_size * 0.48, m.wall_height * 0.5, 0.0),
                        vec3(0.014, m.wall_height * 0.92, m.cell_size * 0.98),
                        None,
                        apply_light(
                            seam_color,
                            center + vec3(m.cell_size * 0.48, m.wall_height * 0.5, 0.0),
                        ),
                    );
                    draw_cube(
                        center + vec3(0.0, m.wall_height * 0.5, -m.cell_size * 0.48),
                        vec3(m.cell_size * 0.98, m.wall_height * 0.92, 0.014),
                        None,
                        apply_light(
                            seam_color,
                            center + vec3(0.0, m.wall_height * 0.5, -m.cell_size * 0.48),
                        ),
                    );
                    draw_cube(
                        center + vec3(0.0, m.wall_height * 0.5, m.cell_size * 0.48),
                        vec3(m.cell_size * 0.98, m.wall_height * 0.92, 0.014),
                        None,
                        apply_light(
                            seam_color,
                            center + vec3(0.0, m.wall_height * 0.5, m.cell_size * 0.48),
                        ),
                    );

                    // Water stains or discoloration running down walls
                    if allow_detail {
                        let stain_hash = ((x as u32).wrapping_mul(0x85EBCA6B)
                            ^ (y as u32).wrapping_mul(0xC2B2AE35))
                            & 0x1F;
                        if stain_hash < 4 {
                            let stain = Color::new(0.68, 0.70, 0.68, 0.7);
                            let stain_x_offset = ((stain_hash as f32 * 0.1) - 0.15) * m.cell_size;
                            draw_cube(
                                center + vec3(stain_x_offset, m.wall_height * 0.65, 0.0),
                                vec3(0.15, m.wall_height * 0.55, m.cell_size * 0.975),
                                None,
                                apply_light(
                                    stain,
                                    center + vec3(stain_x_offset, m.wall_height * 0.4, 0.0),
                                ),
                            );
                        }
                    }

                    // Lower plaster shadow band inset to avoid coplanar flicker.
                    let lower_band =
                        tint_color(plaster_shadow, x as i32, y as i32, 0x2, 0.02, 0.008);
                    let lower_pos = center + vec3(0.0, m.wall_height * 0.22, 0.0);
                    draw_cube(
                        lower_pos,
                        vec3(
                            m.cell_size * 0.985,
                            m.wall_height * 0.44,
                            m.cell_size * 0.985,
                        ),
                        None,
                        apply_light(lower_band, lower_pos),
                    );

                    // Dark baseboard, inset to avoid z-fighting with wall body.
                    let base_pos = center + vec3(0.0, m.wall_height * 0.015, 0.0);
                    draw_cube(
                        base_pos,
                        vec3(m.cell_size * 0.96, m.wall_height * 0.05, m.cell_size * 0.96),
                        None,
                        apply_light(baseboard, base_pos),
                    );

                    // Accent stripe inset into the wall plane.
                    let stripe_pos = center + vec3(0.0, m.wall_height * 0.5, 0.0);
                    draw_cube(
                        stripe_pos,
                        vec3(m.cell_size * 0.97, m.wall_height * 0.07, m.cell_size * 0.97),
                        None,
                        apply_light(accent_stripe, stripe_pos),
                    );

                    // Add small scuff marks and dents on walls
                    if allow_detail {
                        let scuff_hash = ((x as u32).wrapping_mul(0x6A09E667)
                            ^ (y as u32).wrapping_mul(0xBB67AE85))
                            & 0x3F;
                        if scuff_hash < 6 {
                            let scuff = Color::new(0.72, 0.74, 0.76, 1.0);
                            let scuff_height = m.wall_height * (0.15 + (scuff_hash as f32 * 0.01));
                            let scuff_side = scuff_hash % 4;
                            let scuff_offset = match scuff_side {
                                0 => vec3(-m.cell_size * 0.45, scuff_height, 0.0),
                                1 => vec3(m.cell_size * 0.45, scuff_height, 0.0),
                                2 => vec3(0.0, scuff_height, -m.cell_size * 0.45),
                                _ => vec3(0.0, scuff_height, m.cell_size * 0.45),
                            };
                            draw_cube(
                                center + scuff_offset,
                                vec3(0.18, 0.08, 0.03),
                                None,
                                apply_light(scuff, center + scuff_offset),
                            );
                        }
                    }

                    // Sparse chipped brick reveals to break up flat plaster without heavy geometry.
                    if allow_fine
                        && (((x as u32)
                            .wrapping_mul(2654435761u32)
                            .wrapping_add((y as u32).wrapping_mul(40503)))
                            & 0xF)
                            == 0
                    {
                        let chipped = tint_color(brick, x as i32, y as i32, 0x7, 0.04, 0.01);
                        let chip_pos = center + vec3(0.0, m.wall_height * 0.32, 0.0);
                        draw_cube(
                            chip_pos,
                            vec3(m.cell_size * 0.45, m.wall_height * 0.12, m.cell_size * 0.10),
                            None,
                            apply_light(chipped, chip_pos),
                        );
                    }

                    // Occasional exposed conduit or pipe fixtures
                    if allow_detail {
                        let fixture_hash = ((x as u32).wrapping_mul(0x3C6EF372)
                            ^ (y as u32).wrapping_mul(0xDAA66D2B))
                            & 0x7F;
                        if fixture_hash < 2 {
                            let pipe_metal = Color::new(0.45, 0.45, 0.47, 1.0);
                            draw_cube(
                                center + vec3(0.0, m.wall_height * 0.75, -m.cell_size * 0.48),
                                vec3(m.cell_size * 0.94, 0.04, 0.04),
                                None,
                                apply_light(
                                    pipe_metal,
                                    center + vec3(0.0, m.wall_height * 0.75, -m.cell_size * 0.48),
                                ),
                            );
                        }
                    }
                }
            }
        }

        // Simple roof to enclose the maze; thin slab across the whole map.
        let roof_color = Color::new(0.22, 0.23, 0.25, 1.0);
        let roof_pos = vec3(map_w * 0.5, m.wall_height + 0.02, map_h * 0.5);
        draw_cube(
            roof_pos,
            vec3(map_w * 1.02, 0.04, map_h * 1.02),
            None,
            apply_light(roof_color, roof_pos),
        );

        // Add ceiling panel grid lines (every 2 cells)
        let panel_line = Color::new(0.18, 0.19, 0.21, 1.0);
        let ceiling_y = m.wall_height + 0.01;

        // Horizontal panel lines
        for cy in (0..m.height).step_by(2) {
            let z_pos = cy as f32 * m.cell_size;
            draw_cube(
                vec3(map_w * 0.5, ceiling_y, z_pos),
                vec3(map_w * 1.01, 0.008, 0.02),
                None,
                apply_light(panel_line, vec3(map_w * 0.5, ceiling_y, z_pos)),
            );
        }

        // Vertical panel lines
        for cx in (0..m.width).step_by(2) {
            let x_pos = cx as f32 * m.cell_size;
            draw_cube(
                vec3(x_pos, ceiling_y, map_h * 0.5),
                vec3(0.02, 0.008, map_h * 1.01),
                None,
                apply_light(panel_line, vec3(x_pos, ceiling_y, map_h * 0.5)),
            );
        }

        // Add water stains and discoloration on ceiling
        for stain_idx in 0..12 {
            let hash_x = (stain_idx as u32).wrapping_mul(0x9E3779B9);
            let hash_y = (stain_idx as u32).wrapping_mul(0x7F4A7C15);
            let rx = ((hash_x % 1000) as f32 / 1000.0) * map_w * 0.8 + map_w * 0.1;
            let rz = ((hash_y % 1000) as f32 / 1000.0) * map_h * 0.8 + map_h * 0.1;
            let stain_size = 0.6 + ((hash_x % 100) as f32 / 100.0) * 1.2;

            let ceiling_stain = Color::new(0.18, 0.19, 0.20, 0.6);
            draw_cube(
                vec3(rx, ceiling_y - 0.005, rz),
                vec3(stain_size, 0.005, stain_size * 0.8),
                None,
                apply_light(ceiling_stain, vec3(rx, ceiling_y, rz)),
            );
        }

        // Occasional exposed beams or support structure
        for beam_idx in 0..4 {
            let beam_z = (beam_idx as f32 + 1.0) * (map_h / 5.0);
            let beam_color = Color::new(0.16, 0.17, 0.18, 1.0);
            draw_cube(
                vec3(map_w * 0.5, ceiling_y - 0.02, beam_z),
                vec3(map_w * 1.0, 0.06, 0.12),
                None,
                apply_light(beam_color, vec3(map_w * 0.5, ceiling_y - 0.02, beam_z)),
            );
        }

        // Draw ceiling light fixtures
        let fixture_housing = Color::new(0.92, 0.92, 0.94, 1.0); // Brighter to stand out from walls
        let fixture_rim = Color::new(0.55, 0.55, 0.57, 1.0); // Darker rim for contrast
        let light_diffuser = Color::new(0.98, 0.96, 0.90, 1.0);
        let light_glow = Color::new(1.0, 0.98, 0.88, 0.6);

        for light in &ceiling_lights {
            let pos = light.pos + vec3(0.0, 0.08, 0.0); // Closer to ceiling

            // Main fixture housing (recessed panel style) - slightly smaller
            draw_cube(pos, vec3(0.48, 0.07, 0.48), None, fixture_housing);

            // Dark rim around fixture
            draw_cube(
                pos + vec3(0.0, -0.035, 0.0),
                vec3(0.52, 0.015, 0.52),
                None,
                fixture_rim,
            );

            // Bright light diffuser panel
            draw_cube(
                pos + vec3(0.0, -0.05, 0.0),
                vec3(0.50, 0.01, 0.50),
                None,
                Color {
                    r: light_diffuser.r * light.color.x,
                    g: light_diffuser.g * light.color.y,
                    b: light_diffuser.b * light.color.z,
                    a: 1.0,
                },
            );

            // Soft glow to emphasize the light source
            draw_cube(
                pos + vec3(0.0, -0.20, 0.0),
                vec3(0.62, 0.01, 0.62),
                None,
                Color {
                    r: light_glow.r * light.color.x,
                    g: light_glow.g * light.color.y,
                    b: light_glow.b * light.color.z,
                    a: light_glow.a * 0.3,
                },
            );

            // Extended glow for atmosphere
            draw_cube(
                pos + vec3(0.0, -0.35, 0.0),
                vec3(0.90, 0.01, 0.90),
                None,
                Color {
                    r: light_glow.r * light.color.x,
                    g: light_glow.g * light.color.y,
                    b: light_glow.b * light.color.z,
                    a: light_glow.a * 0.12,
                },
            );
        }

        draw_props(m, &ceiling_lights, viewer_pos, render_dist_sq);
    } else {
        draw_cube(vec3(0.0, -1.0, 0.0), vec3(20.0, 0.1, 20.0), None, GRAY);
    }
}
