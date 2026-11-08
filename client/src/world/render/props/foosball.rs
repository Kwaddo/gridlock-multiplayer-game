use macroquad::prelude::*;

use crate::world::render::maze::MeshBuilder;

pub(super) fn draw_foosball(out: &mut MeshBuilder, c: Vec3, lit: &impl Fn(Color, Vec3) -> Color) {
    // Realistic foosball table
    let wood_body = Color::new(0.30, 0.18, 0.08, 1.0);
    let wood_trim = Color::new(0.38, 0.24, 0.12, 1.0);
    let wood_dark = Color::new(0.22, 0.13, 0.06, 1.0);
    let leg_color = Color::new(0.24, 0.15, 0.07, 1.0);
    let field_green = Color::new(0.12, 0.42, 0.15, 1.0);
    let field_line = Color::new(0.85, 0.85, 0.85, 1.0);
    let rod_chrome = Color::new(0.65, 0.65, 0.68, 1.0);
    let handle_black = Color::new(0.10, 0.10, 0.12, 1.0);
    let player_red = Color::new(0.75, 0.12, 0.12, 1.0);
    let player_blue = Color::new(0.10, 0.20, 0.70, 1.0);
    let goal_white = Color::new(0.80, 0.80, 0.82, 1.0);
    let score_rail = Color::new(0.55, 0.55, 0.58, 1.0);
    let ball_white = Color::new(0.92, 0.92, 0.90, 1.0);

    // Table is oriented along the X-axis (long side) — players on the Z sides

    // === LEGS (4 sturdy corner legs) ===
    for &(lx, lz) in &[(-0.48, -0.28), (0.48, -0.28), (-0.48, 0.28), (0.48, 0.28)] {
        // Main leg post
        let pos = c + vec3(lx, 0.28, lz);
        out.cube(pos, vec3(0.08, 0.56, 0.08), lit(leg_color, pos));
        // Foot pad
        let pos = c + vec3(lx, 0.02, lz);
        out.cube(pos, vec3(0.10, 0.04, 0.10), lit(handle_black, pos));
        // Leg brace (cross brace between legs on same side)
    }
    // Cross braces along Z-axis (connecting front-back legs)
    for &lx in &[-0.48, 0.48] {
        let pos = c + vec3(lx, 0.12, 0.0);
        out.cube(pos, vec3(0.05, 0.05, 0.48), lit(wood_dark, pos));
    }

    // === TABLE BODY (cabinet) ===
    // Main body frame
    let pos = c + vec3(0.0, 0.58, 0.0);
    out.cube(pos, vec3(1.10, 0.06, 0.68), lit(wood_body, pos));
    // Top rim/rail on all four sides
    let pos = c + vec3(0.0, 0.64, 0.0);
    out.cube(pos, vec3(1.14, 0.06, 0.72), lit(wood_trim, pos));
    // Side walls (long sides along X — the "bumpers")
    let pos = c + vec3(0.0, 0.72, -0.34);
    out.cube(pos, vec3(1.12, 0.10, 0.04), lit(wood_trim, pos));
    let pos = c + vec3(0.0, 0.72, 0.34);
    out.cube(pos, vec3(1.12, 0.10, 0.04), lit(wood_trim, pos));
    // End walls (short sides along Z — goal ends) with goal openings
    // Left end wall (two pieces with goal gap)
    let pos = c + vec3(-0.56, 0.72, -0.22);
    out.cube(pos, vec3(0.04, 0.10, 0.20), lit(wood_trim, pos));
    let pos = c + vec3(-0.56, 0.72, 0.22);
    out.cube(pos, vec3(0.04, 0.10, 0.20), lit(wood_trim, pos));
    // Right end wall
    let pos = c + vec3(0.56, 0.72, -0.22);
    out.cube(pos, vec3(0.04, 0.10, 0.20), lit(wood_trim, pos));
    let pos = c + vec3(0.56, 0.72, 0.22);
    out.cube(pos, vec3(0.04, 0.10, 0.20), lit(wood_trim, pos));

    // Corner posts (top edge detail)
    for &(cx2, cz) in &[(-0.55, -0.34), (0.55, -0.34), (-0.55, 0.34), (0.55, 0.34)] {
        let pos = c + vec3(cx2, 0.72, cz);
        out.cube(pos, vec3(0.06, 0.12, 0.06), lit(wood_dark, pos));
    }

    // === PLAYING FIELD (green felt surface) ===
    let pos = c + vec3(0.0, 0.68, 0.0);
    out.cube(pos, vec3(1.04, 0.02, 0.60), lit(field_green, pos));

    // Field markings — center line
    let pos = c + vec3(0.0, 0.692, 0.0);
    out.cube(pos, vec3(0.01, 0.004, 0.58), lit(field_line, pos));
    // Center circle (approximated with cross)
    out.cube(c + vec3(0.0, 0.692, 0.0), vec3(0.10, 0.004, 0.01), lit(field_line, c));
    out.cube(c + vec3(0.0, 0.692, 0.0), vec3(0.01, 0.004, 0.10), lit(field_line, c));
    // Penalty boxes
    for &side in &[-0.42_f32, 0.42] {
        let pos = c + vec3(side, 0.692, 0.0);
        out.cube(pos, vec3(0.14, 0.004, 0.28), lit(field_line, pos));
        // Outline only — inner cutout to make a box outline
        let pos = c + vec3(side, 0.694, 0.0);
        out.cube(pos, vec3(0.12, 0.004, 0.24), lit(field_green, pos));
    }

    // === GOALS (recessed goal areas at each end) ===
    for &gx in &[-0.56_f32, 0.56] {
        // Goal frame (white)
        let pos = c + vec3(gx, 0.70, 0.0);
        out.cube(pos, vec3(0.04, 0.08, 0.16), lit(goal_white, pos));
        // Goal back
        let sign = if gx < 0.0 { -1.0 } else { 1.0 };
        let pos = c + vec3(gx + sign * 0.04, 0.70, 0.0);
        out.cube(pos, vec3(0.02, 0.06, 0.14), lit(handle_black, pos));
    }

    // === RODS (8 rods across, chrome with handles) ===
    // Rod positions along X-axis: goalie, defense, midfield, attack (mirrored)
    let rod_positions = [-0.44, -0.30, -0.14, -0.04, 0.04, 0.14, 0.30, 0.44_f32];
    // Number of players on each rod (standard layout)
    let rod_players = [1, 2, 5, 3, 3, 5, 2, 1_usize];
    // Which side: false = red (left team), true = blue (right team)
    let rod_team = [false, false, false, false, true, true, true, true];

    let rod_y = 0.82;

    for (i, &rx) in rod_positions.iter().enumerate() {
        // Chrome rod spanning the full width (Z direction) + handles
        let pos = c + vec3(rx, rod_y, 0.0);
        out.cube(pos, vec3(0.018, 0.018, 0.74), lit(rod_chrome, pos));

        // Handles on both ends
        let pos = c + vec3(rx, rod_y, -0.40);
        out.cube(pos, vec3(0.028, 0.028, 0.10), lit(handle_black, pos));
        let pos = c + vec3(rx, rod_y, 0.40);
        out.cube(pos, vec3(0.028, 0.028, 0.10), lit(handle_black, pos));

        // Players on this rod
        let num = rod_players[i];
        let team_color = if rod_team[i] { player_blue } else { player_red };
        let spacing = if num > 1 {
            0.52 / (num as f32)
        } else {
            0.0
        };
        let start_z = if num > 1 {
            -(spacing * (num as f32 - 1.0)) / 2.0
        } else {
            0.0
        };

        for p in 0..num {
            let pz = start_z + p as f32 * spacing;
            // Player body (torso)
            let pos = c + vec3(rx, rod_y - 0.06, pz);
            out.cube(pos, vec3(0.04, 0.10, 0.04), lit(team_color, pos));
            // Player head
            let pos = c + vec3(rx, rod_y + 0.02, pz);
            out.cube(pos, vec3(0.03, 0.03, 0.03), lit(team_color, pos));
            // Player foot (kicker)
            let pos = c + vec3(rx, rod_y - 0.12, pz);
            out.cube(pos, vec3(0.03, 0.04, 0.05), lit(team_color, pos));
        }
    }

    // === SCORE COUNTERS (rails on both long sides) ===
    for &sz in &[-0.36_f32, 0.36] {
        let pos = c + vec3(0.0, 0.78, sz);
        out.cube(pos, vec3(0.40, 0.012, 0.014), lit(score_rail, pos));
        // Score beads (5 per side)
        for bead in 0..5 {
            let bx = -0.16 + bead as f32 * 0.08;
            let pos = c + vec3(bx, 0.78, sz);
            out.cube(pos, vec3(0.02, 0.02, 0.02), lit(player_red, pos));
        }
    }

    // === BALL (sitting on the field) ===
    let pos = c + vec3(0.0, 0.71, 0.0);
    out.cube(pos, vec3(0.025, 0.025, 0.025), ball_white);
}
