use macroquad::prelude::*;
use std::collections::HashMap;

use crate::physics::movement::MovementState;
use crate::ui::draw_arc;
use crate::ui::fonts::{measure_game_text, text_params};
use crate::ui::OtherPlayer;
use crate::weapons::crosshair::CrosshairType;
use crate::weapons::effects::DamageNumber;
use crate::weapons::stats::CrosshairStyle;
use crate::weapons::WeaponInventory;

pub fn draw_hud(
    username: &str,
    player_id: u32,
    position: Vec3,
    forward: Vec3,
    right: Vec3,
    pitch: f32,
    movement_state: &MovementState,
    other_players: &[OtherPlayer],
    weapons: &WeaponInventory,
    assets: &HashMap<String, Texture2D>,
    hit_marker_timer: f32,
    hit_marker_headshot: bool,
    damage_numbers: &[DamageNumber],
    hit_marker_pellet_mask: u16,
    hp: f32,
    max_hp: f32,
    shield: f32,
    max_shield: f32,
    kill_feed: &[crate::screens::playing::KillFeedEntry],
) {
    let cx = screen_width() / 2.0;
    let cy = screen_height() / 2.0;

    // sprint sway — crosshair bobs side-to-side when sprinting
    let sprint_sway = if movement_state.is_sprinting {
        let freq = 8.0;
        let amp = screen_height() * 0.4 / 100.0;
        (macroquad::time::get_time() as f32 * freq).sin() * amp
    } else {
        0.0
    };
    let cx = cx + sprint_sway;

    let weapon = weapons.active();
    let stats = weapon.stats();

    // ── DYNAMIC CROSSHAIR ──────────────────────────
    // visual config comes from weapon.crosshair (runtime), gameplay from stats (const)
    let sh = screen_height();
    let xhair = &weapon.crosshair;
    let ads_t = weapon.ads_progress;

    // FSA tint: gray when FSA is ready AND player is aiming (hip-fire shouldn't look gray)
    let fsa_active = stats.fsa_time.map_or(false, |t| weapon.fsa_timer >= t);
    let color = if fsa_active && weapon.ads_progress > 0.5 {
        Color::from_rgba(180, 180, 180, 255)
    } else {
        xhair.color
    };

    // flash tint briefly on shot
    let _tint = if weapon.muzzle_flash_timer > 0.04 {
        Color::from_rgba(255, 255, 200, 255)
    } else {
        WHITE
    };

    match stats.crosshair_style {
        CrosshairStyle::Standard => {
            match xhair.crosshair_type {
                CrosshairType::FourCurves => {
                    // 4-curves crosshair (shotgun)
                    let stats = weapon.stats();
                    let _base = stats.hip_spread
                        + (stats.ads_spread - stats.hip_spread) * weapon.ads_progress;

                    let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - weapon.ads_progress);
                    let kick_gap = (weapon.crosshair_kick * sh) / 100.0;
                    let movement_gap = 0.0; // for shotguns, we won't scale the crosshair with movement bloom, since its a run and gun weapon
                    let ads_rest_gap = if fsa_active {
                        (sh * 0.3) / 100.0 // small fixed gap when FSA is active (~3px at 1080p)
                    } else {
                        ((xhair.center_gap * sh) / 100.0) * 0.4 * weapon.ads_progress
                    };

                    let radius = hip_gap + kick_gap + ads_rest_gap + movement_gap;
                    let thick = (xhair.line_thickness * sh) / 100.0;
                    let gap_angle: f32 = 20.0_f32.to_radians(); // gap at cardinal directions
                    let quarter = std::f32::consts::FRAC_PI_2;
                    let segs = 32u32;

                    // outline
                    if xhair.outline_enabled {
                        let ot = thick + (xhair.outline_thickness * sh) / 100.0 * 2.0;
                        let oc = Color::from_rgba(0, 0, 0, 180);
                        for q in 0..4u32 {
                            let base_angle = q as f32 * quarter;
                            draw_arc(
                                cx,
                                cy,
                                radius,
                                base_angle + gap_angle,
                                base_angle + quarter - gap_angle,
                                ot,
                                segs,
                                oc,
                            );
                        }
                    }

                    // 4 arcs (one per quadrant, gaps at top/bottom/left/right)
                    for q in 0..4u32 {
                        let base_angle = q as f32 * quarter;
                        draw_arc(
                            cx,
                            cy,
                            radius,
                            base_angle + gap_angle,
                            base_angle + quarter - gap_angle,
                            thick,
                            segs,
                            color,
                        );
                    }

                    // center dot (visible at hip, fades out when ADS)
                    if xhair.dot_enabled {
                        let dot_alpha = ((1.0 - weapon.ads_progress) * 255.0) as u8;
                        let dot_color = Color::from_rgba(
                            (color.r * 255.0) as u8,
                            (color.g * 255.0) as u8,
                            (color.b * 255.0) as u8,
                            dot_alpha,
                        );
                        draw_circle(cx, cy, (xhair.dot_size * sh) / 100.0, dot_color);
                    }
                }
                CrosshairType::Chevron => {
                    // L-bracket crosshair with bezier-chamfered corners
                    let stats = weapon.stats();
                    let _base = stats.hip_spread
                        + (stats.ads_spread - stats.hip_spread) * weapon.ads_progress;

                    let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - weapon.ads_progress);
                    let kick_gap = (weapon.crosshair_kick * sh) / 100.0;
                    let movement_gap = 0.0;
                    let ads_rest_gap = if fsa_active {
                        (sh * 0.3) / 100.0
                    } else {
                        ((xhair.center_gap * sh) / 100.0) * 0.7 * weapon.ads_progress
                    };

                    let radius = hip_gap + kick_gap + ads_rest_gap + movement_gap;
                    let thick = (xhair.line_thickness * sh) / 100.0;

                    // 4 corner brackets (L-shapes)
                    let arm = ((xhair.line_length / 2.0) * sh) / 100.0;

                    let corners: [(f32, f32); 4] = [
                        (-1.0, -1.0), // top-left
                        (1.0, -1.0),  // top-right
                        (-1.0, 1.0),  // bottom-left
                        (1.0, 1.0),   // bottom-right
                    ];

                    for &(sx, sy) in &corners {
                        let corner_x = cx + sx * radius;
                        let corner_y = cy + sy * radius;
                        let h_end_x = corner_x - sx * arm;
                        let v_end_y = corner_y - sy * arm;

                        // chamfer: cut the corner with a curved diagonal
                        let chamfer = arm * 0.35; // smaller chamfer = longer straight diagonal

                        let h_near = corner_x - sx * chamfer;
                        let v_near = corner_y - sy * chamfer;

                        // quadratic bezier: P0 = (h_near, corner_y), P1 = (corner_x, corner_y), P2 = (corner_x, v_near)
                        let p0x = h_near;
                        let p0y = corner_y;
                        let p1x = corner_x;
                        let p1y = corner_y;
                        let p2x = corner_x;
                        let p2y = v_near;
                        let bez = |t: f32| -> (f32, f32) {
                            let u = 1.0 - t;
                            (
                                u * u * p0x + 2.0 * u * t * p1x + t * t * p2x,
                                u * u * p0y + 2.0 * u * t * p1y + t * t * p2y,
                            )
                        };

                        // chamfer curves at ends, long straight diagonal in the middle
                        let t_a = 0.10_f32; // end of chamfer1 / start of straight
                        let t_b = 0.90_f32; // end of straight / start of chamfer2
                        let (mid_ax, mid_ay) = bez(t_a);
                        let (mid_bx, mid_by) = bez(t_b);
                        let chamfer_segs = 4;

                        let draw_corner_shape = |thickness: f32, col: Color| {
                            // chamfer 1: curve from t=0 to t=t_a
                            for i in 0..chamfer_segs {
                                let s0 = t_a * (i as f32 / chamfer_segs as f32);
                                let s1 = t_a * ((i + 1) as f32 / chamfer_segs as f32);
                                let (x0, y0) = bez(s0);
                                let (x1, y1) = bez(s1);
                                draw_line(x0, y0, x1, y1, thickness, col);
                            }
                            // straight diagonal connector (longer now)
                            draw_line(mid_ax, mid_ay, mid_bx, mid_by, thickness, col);
                            // chamfer 2: curve from t=t_b to t=1.0
                            for i in 0..chamfer_segs {
                                let s0 = t_b + (1.0 - t_b) * (i as f32 / chamfer_segs as f32);
                                let s1 = t_b + (1.0 - t_b) * ((i + 1) as f32 / chamfer_segs as f32);
                                let (x0, y0) = bez(s0);
                                let (x1, y1) = bez(s1);
                                draw_line(x0, y0, x1, y1, thickness, col);
                            }
                        };

                        // outline
                        if xhair.outline_enabled {
                            let ot = thick + (xhair.outline_thickness * sh) / 100.0 * 2.0;
                            let oc = Color::from_rgba(0, 0, 0, 180);
                            draw_line(h_near, corner_y, h_end_x, corner_y, ot, oc);
                            draw_line(corner_x, v_near, corner_x, v_end_y, ot, oc);
                            draw_corner_shape(ot, oc);
                        }

                        draw_line(h_near, corner_y, h_end_x, corner_y, thick, color);
                        draw_line(corner_x, v_near, corner_x, v_end_y, thick, color);
                        draw_corner_shape(thick, color);
                    }

                    // center dot (visible at hip, fades out when ADS)
                    if xhair.dot_enabled {
                        let dot_alpha = ((1.0 - weapon.ads_progress) * 255.0) as u8;
                        let dot_color = Color::from_rgba(
                            (color.r * 255.0) as u8,
                            (color.g * 255.0) as u8,
                            (color.b * 255.0) as u8,
                            dot_alpha,
                        );
                        draw_circle(cx, cy, (xhair.dot_size * sh) / 100.0, dot_color);
                    }
                }
                CrosshairType::Cross => {
                    // cross crosshair (pistol, AR, SMG)
                    // all values are % of screen height so it scales with resolution
                    let stats = weapon.stats();
                    let base = stats.hip_spread
                        + (stats.ads_spread - stats.hip_spread) * weapon.ads_progress;

                    let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - weapon.ads_progress);
                    let kick_gap = (weapon.crosshair_kick * sh) / 100.0;

                    let movement_gap =
                        ((weapon.computed_spread - base).max(0.0) * 40.0 * sh) / 100.0;

                    // resting ads gap, this is so there is ALWAYS a gap unless FSA is active
                    let ads_rest_gap = if fsa_active {
                        (sh * 0.3) / 100.0 // small fixed gap when FSA is active (~3px at 1080p)
                    } else {
                        ((xhair.center_gap * sh) / 100.0) * 0.7 * weapon.ads_progress
                        // 40% of center gap as minimum gap when fully ADS, scales with ads_progress
                    };

                    let gap = hip_gap + kick_gap + ads_rest_gap + movement_gap;

                    let length = (xhair.line_length * sh) / 100.0;
                    let thick = (xhair.line_thickness * sh) / 100.0;

                    // 4 lines: up, down, left, right
                    draw_rectangle(cx - thick / 2.0, cy - gap - length, thick, length, color); // up
                    draw_rectangle(cx - thick / 2.0, cy + gap, thick, length, color); // down
                    draw_rectangle(cx - gap - length, cy - thick / 2.0, length, thick, color); // left
                    draw_rectangle(cx + gap, cy - thick / 2.0, length, thick, color); // right

                    // center dot
                    if xhair.dot_enabled {
                        draw_circle(cx, cy, (xhair.dot_size * sh) / 100.0, color);
                    }
                }
                CrosshairType::Circle => {
                    // Simple circle crosshair
                    let stats = weapon.stats();
                    let base = stats.hip_spread
                        + (stats.ads_spread - stats.hip_spread) * weapon.ads_progress;

                    let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - weapon.ads_progress);
                    let kick_gap = (weapon.crosshair_kick * sh) / 100.0;
                    let movement_gap =
                        ((weapon.computed_spread - base).max(0.0) * 40.0 * sh) / 100.0;
                    let ads_rest_gap = if fsa_active {
                        (sh * 0.3) / 100.0
                    } else {
                        ((xhair.center_gap * sh) / 100.0) * 0.7 * weapon.ads_progress
                    };

                    let radius = hip_gap + kick_gap + ads_rest_gap + movement_gap;
                    let thick = (xhair.line_thickness * sh) / 100.0;

                    if xhair.outline_enabled {
                        let ot = thick + (xhair.outline_thickness * sh) / 100.0 * 2.0;
                        draw_circle_lines(cx, cy, radius, ot, Color::from_rgba(0, 0, 0, 180));
                    }
                    draw_circle_lines(cx, cy, radius, thick, color);

                    if xhair.dot_enabled {
                        draw_circle(cx, cy, (xhair.dot_size * sh) / 100.0, color);
                    }
                }
                CrosshairType::Square => {
                    // Simple square crosshair
                    let stats = weapon.stats();
                    let base = stats.hip_spread
                        + (stats.ads_spread - stats.hip_spread) * weapon.ads_progress;

                    let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - weapon.ads_progress);
                    let kick_gap = (weapon.crosshair_kick * sh) / 100.0;
                    let movement_gap =
                        ((weapon.computed_spread - base).max(0.0) * 40.0 * sh) / 100.0;
                    let ads_rest_gap = if fsa_active {
                        (sh * 0.3) / 100.0
                    } else {
                        ((xhair.center_gap * sh) / 100.0) * 0.7 * weapon.ads_progress
                    };

                    let size = (hip_gap + kick_gap + ads_rest_gap + movement_gap) * 2.0;
                    let thick = (xhair.line_thickness * sh) / 100.0;

                    if xhair.outline_enabled {
                        let ot = thick + (xhair.outline_thickness * sh) / 100.0 * 2.0;
                        draw_rectangle_lines(
                            cx - size / 2.0,
                            cy - size / 2.0,
                            size,
                            size,
                            ot,
                            Color::from_rgba(0, 0, 0, 180),
                        );
                    }
                    draw_rectangle_lines(
                        cx - size / 2.0,
                        cy - size / 2.0,
                        size,
                        size,
                        thick,
                        color,
                    );

                    if xhair.dot_enabled {
                        draw_circle(cx, cy, (xhair.dot_size * sh) / 100.0, color);
                    }
                }
                CrosshairType::Triangle => {
                    // Simple triangle crosshair (pointing up)
                    let stats = weapon.stats();
                    let base = stats.hip_spread
                        + (stats.ads_spread - stats.hip_spread) * weapon.ads_progress;

                    let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - weapon.ads_progress);
                    let kick_gap = (weapon.crosshair_kick * sh) / 100.0;
                    let movement_gap =
                        ((weapon.computed_spread - base).max(0.0) * 40.0 * sh) / 100.0;
                    let ads_rest_gap = if fsa_active {
                        (sh * 0.3) / 100.0
                    } else {
                        ((xhair.center_gap * sh) / 100.0) * 0.7 * weapon.ads_progress
                    };

                    let size = (hip_gap + kick_gap + ads_rest_gap + movement_gap) * 2.0;
                    let thick = (xhair.line_thickness * sh) / 100.0;

                    // Equilateral triangle vertices
                    let height = size * 0.866; // sqrt(3)/2
                    let v1 = (cx, cy - height * 0.67); // top
                    let v2 = (cx - size / 2.0, cy + height * 0.33); // bottom left
                    let v3 = (cx + size / 2.0, cy + height * 0.33); // bottom right

                    if xhair.outline_enabled {
                        let ot = thick + (xhair.outline_thickness * sh) / 100.0 * 2.0;
                        let oc = Color::from_rgba(0, 0, 0, 180);
                        draw_line(v1.0, v1.1, v2.0, v2.1, ot, oc);
                        draw_line(v2.0, v2.1, v3.0, v3.1, ot, oc);
                        draw_line(v3.0, v3.1, v1.0, v1.1, ot, oc);
                    }
                    draw_line(v1.0, v1.1, v2.0, v2.1, thick, color);
                    draw_line(v2.0, v2.1, v3.0, v3.1, thick, color);
                    draw_line(v3.0, v3.1, v1.0, v1.1, thick, color);

                    if xhair.dot_enabled {
                        draw_circle(cx, cy, (xhair.dot_size * sh) / 100.0, color);
                    }
                }
                CrosshairType::AngleBrackets => {
                    // < . > shotgun-style angle brackets
                    let stats = weapon.stats();
                    let _base = stats.hip_spread
                        + (stats.ads_spread - stats.hip_spread) * weapon.ads_progress;

                    let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - weapon.ads_progress);
                    let kick_gap = (weapon.crosshair_kick * sh) / 100.0;
                    let movement_gap = 0.0; // shotgun style - no movement gap
                    let ads_rest_gap = if fsa_active {
                        (sh * 0.3) / 100.0
                    } else {
                        ((xhair.center_gap * sh) / 100.0) * 0.4 * weapon.ads_progress
                    };

                    let gap = hip_gap + kick_gap + ads_rest_gap + movement_gap;
                    let arm = ((xhair.line_length) * sh) / 100.0;
                    let thick = (xhair.line_thickness * sh) / 100.0;

                    // Left bracket <
                    let left_tip_x = cx - gap - arm;
                    let left_base_x = cx - gap;
                    let top_y = cy - arm / 2.0;
                    let bot_y = cy + arm / 2.0;

                    if xhair.outline_enabled {
                        let ot = thick + (xhair.outline_thickness * sh) / 100.0 * 2.0;
                        let oc = Color::from_rgba(0, 0, 0, 180);
                        draw_line(left_base_x, top_y, left_tip_x, cy, ot, oc);
                        draw_line(left_tip_x, cy, left_base_x, bot_y, ot, oc);
                    }
                    draw_line(left_base_x, top_y, left_tip_x, cy, thick, color);
                    draw_line(left_tip_x, cy, left_base_x, bot_y, thick, color);

                    // Right bracket >
                    let right_tip_x = cx + gap + arm;
                    let right_base_x = cx + gap;

                    if xhair.outline_enabled {
                        let ot = thick + (xhair.outline_thickness * sh) / 100.0 * 2.0;
                        let oc = Color::from_rgba(0, 0, 0, 180);
                        draw_line(right_base_x, top_y, right_tip_x, cy, ot, oc);
                        draw_line(right_tip_x, cy, right_base_x, bot_y, ot, oc);
                    }
                    draw_line(right_base_x, top_y, right_tip_x, cy, thick, color);
                    draw_line(right_tip_x, cy, right_base_x, bot_y, thick, color);

                    if xhair.dot_enabled {
                        let dot_alpha = ((1.0 - weapon.ads_progress) * 255.0) as u8;
                        let dot_color = Color::from_rgba(
                            (color.r * 255.0) as u8,
                            (color.g * 255.0) as u8,
                            (color.b * 255.0) as u8,
                            dot_alpha,
                        );
                        draw_circle(cx, cy, (xhair.dot_size * sh) / 100.0, dot_color);
                    }
                }
            }
        }
        CrosshairStyle::Scope => {
            // No hip-fire crosshair for sniper — forces you to scope

            if ads_t > 0.2 {
                // Scope overlay
                let scope_t = ((ads_t - 0.2) / 0.8).min(1.0); // 0→1 over the last 80% of ADS
                let scope_alpha = (scope_t * 255.0) as u8;

                // Black vignette with circular cutout
                let scope_radius = screen_height().min(screen_width()) * 0.38;
                draw_scope_vignette(cx, cy, scope_radius, scope_alpha);

                // Procedural scope reticle — guaranteed centered, no texture offset issues
                let line_color = Color::from_rgba(0, 0, 0, scope_alpha);
                let thin = 1.5;
                // horizontal line
                draw_line(cx - scope_radius, cy, cx - 8.0, cy, thin, line_color);
                draw_line(cx + 8.0, cy, cx + scope_radius, cy, thin, line_color);
                // vertical line
                draw_line(cx, cy - scope_radius, cx, cy - 8.0, thin, line_color);
                draw_line(cx, cy + 8.0, cx, cy + scope_radius, thin, line_color);
                // center dot
                draw_circle(cx, cy, 2.0, Color::from_rgba(255, 50, 50, scope_alpha));
            }
        }
    }

    // -- HIT MARKER (scales with crosshair, shotgun shows pellet dots)
    if hit_marker_timer > 0.0 {
        let t = (hit_marker_timer / 0.35).min(1.0);
        let alpha = (t * 255.0) as u8;
        let hm_color = if hit_marker_headshot {
            Color::from_rgba(255, 215, 0, alpha) // gold for headshot
        } else {
            Color::from_rgba(255, 255, 255, alpha) // white for body
        };

        // scale hit marker to current crosshair size
        // compute effective crosshair size based on gap + line length
        let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - ads_t);
        let ads_rest_gap = ((xhair.center_gap * sh) / 100.0) * 0.4 * ads_t;
        let gap = hip_gap + ads_rest_gap;
        let len = (xhair.line_length * sh) / 100.0;
        let current_size = (gap + len) * 2.0; // full diameter

        // fallback for sniper scope (crosshair size is 0)
        let current_size = if current_size < 10.0 {
            (sh * 3.0) / 100.0
        } else {
            current_size
        };

        let pop = 1.0 + 0.4 * t;
        let hm_thick = 2.5; // size of each line in the hit marker X
        let stats = weapon.stats();
        let pellet_spread = stats.pellet_spread;
        if let Some(ptrn) = pellet_spread {
            // SHOTGUN: dot per pellet, like fortnite
            let dot_radius = 5.0; // size of each pellet dot in the hit marker
            let ring_radius = current_size * 0.3 * pop;
            for (i, &(angle, radius)) in ptrn.iter().enumerate() {
                if hit_marker_pellet_mask & (1 << i) != 0 {
                    let r = ring_radius * radius;
                    let dx = cx + angle.cos() * r;
                    let dy = cy - angle.sin() * r;
                    draw_circle(dx, dy, dot_radius, RED);
                }
            }
        } else {
            // NORMAL: X shape scaled to crosshair
            let hm_size = current_size * 0.4 * pop;
            let hm_gap = current_size * 0.1 * pop;
            draw_line(
                cx - hm_size,
                cy - hm_size,
                cx - hm_gap,
                cy - hm_gap,
                hm_thick,
                hm_color,
            );
            draw_line(
                cx + hm_gap,
                cy - hm_gap,
                cx + hm_size,
                cy - hm_size,
                hm_thick,
                hm_color,
            );
            draw_line(
                cx - hm_size,
                cy + hm_size,
                cx - hm_gap,
                cy + hm_gap,
                hm_thick,
                hm_color,
            );
            draw_line(
                cx + hm_gap,
                cy + hm_gap,
                cx + hm_size,
                cy + hm_size,
                hm_thick,
                hm_color,
            );
        }
    }

    // -- DAMAGE NUMBERS
    for dmg in damage_numbers {
        let alpha = (dmg.lifetime.min(1.0) * 255.0) as u8;
        let (color, size) = if dmg.is_headshot {
            (Color::from_rgba(255, 215, 0, alpha), 28) // gold, large
        } else {
            (Color::from_rgba(255, 255, 255, alpha), 22) // white, normal
        };
        let text = format!("{}", dmg.amount as u32);
        let tw = measure_game_text(&text, size);
        draw_text_ex(
            &text,
            dmg.screen_x - tw / 2.0,
            dmg.screen_y,
            text_params(size, color),
        );
    }

    // -- HUD TEXT
    draw_text_ex("Multiplayer FPS", 20.0, 20.0, text_params(30, WHITE));
    draw_text_ex(
        &format!("Player: {} (ID: {})", username, player_id),
        20.0,
        50.0,
        text_params(20, WHITE),
    );
    draw_text_ex(
        &format!(
            "Pos: {:.2}, {:.2}, {:.2}",
            position.x, position.y, position.z
        ),
        20.0,
        70.0,
        text_params(20, WHITE),
    );

    // -- HEALTH BAR
    let hp_bar_x = 20.0;
    let hp_bar_y = screen_height() - 120.0;
    let hp_bar_w = 200.0;
    let hp_bar_h = 20.0;
    let hp_frac = (hp / max_hp).clamp(0.0, 1.0);

    // Background
    draw_rectangle(
        hp_bar_x,
        hp_bar_y,
        hp_bar_w,
        hp_bar_h,
        Color::from_rgba(40, 40, 40, 200),
    );

    // Health fill (green -> yellow -> red gradient)
    let hp_color = if hp_frac > 0.5 {
        Color::from_rgba(80, 200, 80, 255) // green
    } else if hp_frac > 0.25 {
        Color::from_rgba(220, 180, 40, 255) // yellow
    } else {
        RED // critical
    };
    draw_rectangle(hp_bar_x, hp_bar_y, hp_bar_w * hp_frac, hp_bar_h, hp_color);

    // Border
    draw_rectangle_lines(hp_bar_x, hp_bar_y, hp_bar_w, hp_bar_h, 2.0, WHITE);

    // HP text
    let hp_text = format!("{:.0} HP", hp);
    draw_text_ex(
        &hp_text,
        hp_bar_x + 5.0,
        hp_bar_y + 15.0,
        text_params(16, WHITE),
    );

    // -- SHIELD BAR (only if player has shield capacity)
    if max_shield > 0.0 {
        let shield_bar_y = hp_bar_y - 25.0;
        let shield_frac = (shield / max_shield).clamp(0.0, 1.0);

        // Background
        draw_rectangle(
            hp_bar_x,
            shield_bar_y,
            hp_bar_w,
            hp_bar_h,
            Color::from_rgba(40, 40, 40, 200),
        );

        // Shield fill (cyan/blue)
        draw_rectangle(
            hp_bar_x,
            shield_bar_y,
            hp_bar_w * shield_frac,
            hp_bar_h,
            Color::from_rgba(80, 180, 255, 255),
        );

        // Border
        draw_rectangle_lines(hp_bar_x, shield_bar_y, hp_bar_w, hp_bar_h, 2.0, WHITE);

        // Shield text
        let shield_text = format!("{:.0} SHIELD", shield);
        draw_text_ex(
            &shield_text,
            hp_bar_x + 5.0,
            shield_bar_y + 15.0,
            text_params(16, WHITE),
        );
    }

    // -- STAMINA BAR
    let bar_width = 200.0;
    let bar_height = 8.0;
    let bar_x = screen_width() - bar_width - 20.0;
    let bar_y = screen_height() - 40.0;
    draw_rectangle(
        bar_x,
        bar_y,
        bar_width,
        bar_height,
        Color::from_rgba(50, 50, 50, 255),
    );
    let fill = movement_state.stamina / 100.0;
    let bar_color = if movement_state.stamina < 20.0 {
        RED
    } else {
        Color::from_rgba(80, 180, 80, 255)
    };
    draw_rectangle(bar_x, bar_y, bar_width * fill, bar_height, bar_color);
    draw_rectangle_lines(bar_x, bar_y, bar_width, bar_height, 1.0, GRAY);

    // -- CROUCH INDICATOR
    if movement_state.is_crouching {
        draw_text_ex(
            "CROUCHING",
            20.0,
            screen_height() - 20.0,
            text_params(16, Color::from_rgba(150, 150, 150, 255)),
        );
    }

    // -- NAMETAGS
    for p in other_players {
        let head_pos = p.render_position() + vec3(0.0, 1.8, 0.0);
        let rel = head_pos - position;
        let dot = rel.dot(forward);
        if dot > 0.1 {
            let right_dot = rel.dot(right);
            let up_dot = rel.y - position.y + pitch.sin() * dot;
            let fov_scale = screen_height() / (2.0 * ((45.0_f32).to_radians() / 2.0).tan());
            let screen_x = screen_width() / 2.0 + (right_dot / dot) * fov_scale;
            let screen_y = screen_height() / 2.0 - (up_dot / dot) * fov_scale;
            let name_w = measure_game_text(&p.username, 16);
            draw_text_ex(
                &p.username,
                screen_x - name_w / 2.0,
                screen_y,
                text_params(16, p.color),
            );
        }
    }

    // -- PLAYER LIST
    draw_text_ex(
        &format!("Players online: {}", other_players.len() + 1),
        20.0,
        100.0,
        text_params(20, YELLOW),
    );
    for (i, p) in other_players.iter().enumerate() {
        draw_text_ex(
            &format!("   {} (ID: {})", p.username, p.id),
            20.0,
            120.0 + (i as f32) * 20.0,
            text_params(18, p.color),
        );
    }

    // ── WEAPON INFO (sprite + name + ammo) ──────────────────────────
    let ammo_text = if weapon.is_reloading {
        "RELOADING...".to_string()
    } else {
        format!("{} / {}", weapon.ammo, weapon.reserve)
    };

    let ammo_w = measure_game_text(&ammo_text, 24);
    draw_text_ex(
        &ammo_text,
        screen_width() - ammo_w - 20.0,
        screen_height() - 60.0,
        text_params(
            24,
            if weapon.ammo == 0 && !weapon.is_reloading {
                RED
            } else {
                WHITE
            },
        ),
    );

    let name_w = measure_game_text(stats.name, 16);
    let name_x = screen_width() - name_w - 20.0;
    let name_y = screen_height() - 80.0;
    draw_text_ex(
        stats.name,
        name_x,
        name_y,
        text_params(16, Color::from_rgba(180, 180, 180, 255)),
    );

    // weapon sprite: sits to the left of the weapon name
    let icon_w = stats.icon_width;
    let icon_h = stats.icon_height;
    let icon_x = name_x - icon_w - 8.0;
    let icon_y = name_y - icon_h + 4.0; // align bottom of icon with text baseline

    // recoil kick on fire
    let kick_y = if weapon.muzzle_flash_timer > 0.0 {
        (weapon.muzzle_flash_timer / 0.08) * 6.0
    } else {
        0.0
    };

    if let Some(tex) = assets.get(stats.icon_asset_key) {
        draw_texture_ex(
            tex,
            icon_x,
            icon_y + kick_y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(icon_w, icon_h)),
                flip_x: stats.icon_flip_x,
                ..Default::default()
            },
        );
    } else {
        draw_rectangle(
            icon_x,
            icon_y + kick_y,
            icon_w,
            icon_h,
            Color::from_rgba(80, 80, 80, 150),
        );
        draw_rectangle_lines(icon_x, icon_y + kick_y, icon_w, icon_h, 1.0, GRAY);
    }

    // muzzle flash at muzzle tip of the sprite
    if weapon.muzzle_flash_timer > 0.0 {
        let t = weapon.muzzle_flash_timer / 0.08;
        let alpha = (t * 255.0) as u8;
        let flash_x = if stats.icon_flip_x {
            icon_x + 6.0
        } else {
            icon_x + icon_w - 6.0
        };
        let flash_y = icon_y + kick_y + 5.0;

        draw_circle(
            flash_x,
            flash_y,
            14.0 * t,
            Color::from_rgba(255, 180, 50, alpha / 2),
        );
        draw_circle(
            flash_x,
            flash_y,
            6.0 * t,
            Color::from_rgba(255, 240, 180, alpha),
        );
        for i in 0..4 {
            let angle = (i as f32) * 0.8 + t * 3.0;
            let len = 10.0 * t;
            draw_line(
                flash_x,
                flash_y,
                flash_x + angle.cos() * len,
                flash_y + angle.sin() * len,
                1.5,
                Color::from_rgba(255, 200, 80, alpha),
            );
        }
    }

    // -- KILL FEED (top right)
    let feed_x = screen_width() - 400.0;
    let mut feed_y = 20.0;
    for entry in kill_feed.iter().rev().take(5) {
        // Show last 5 kills, newest at top
        let alpha = ((entry.timer / 5.0).min(1.0) * 255.0) as u8;
        let text_color = Color::from_rgba(255, 255, 255, alpha);

        // Find player names from other_players list
        let killer_name = other_players
            .iter()
            .find(|p| p.id == entry.killer_id)
            .map(|p| p.username.as_str())
            .unwrap_or("Player");
        let victim_name = other_players
            .iter()
            .find(|p| p.id == entry.victim_id)
            .map(|p| p.username.as_str())
            .unwrap_or("Player");

        let mut x_offset = feed_x;

        // Draw killer name
        draw_text_ex(killer_name, x_offset, feed_y, text_params(14, text_color));
        x_offset += measure_game_text(killer_name, 14) + 5.0;

        // Draw weapon icon (16x16)
        let weapon_icon_key = format!("{}_icon", entry.weapon.to_lowercase());
        if let Some(icon_tex) = assets.get(&weapon_icon_key) {
            draw_texture_ex(
                icon_tex,
                x_offset,
                feed_y - 12.0,
                Color::from_rgba(255, 255, 255, alpha),
                DrawTextureParams {
                    dest_size: Some(vec2(16.0, 16.0)),
                    ..Default::default()
                },
            );
        } else {
            // Fallback: draw weapon name in brackets
            let weapon_text = format!("[{}]", entry.weapon);
            draw_text_ex(&weapon_text, x_offset, feed_y, text_params(12, text_color));
        }
        x_offset += 20.0;

        // TODO: Draw skull icon here (death icon)
        // if let Some(skull_tex) = assets.get("skull_icon") {
        //     draw_texture_ex(...);
        // }

        // Draw victim name
        draw_text_ex(victim_name, x_offset, feed_y, text_params(14, text_color));

        feed_y += 20.0;
    }
}

fn draw_scope_vignette(cx: f32, cy: f32, radius: f32, alpha: u8) {
    let color = Color::from_rgba(0, 0, 0, alpha);
    let far = screen_width().max(screen_height());
    let steps = 64;

    for i in 0..steps {
        let a0 = ((i as f32) / (steps as f32)) * std::f32::consts::TAU;
        let a1 = (((i + 1) as f32) / (steps as f32)) * std::f32::consts::TAU;

        let c0 = vec2(cx + a0.cos() * radius, cy + a0.sin() * radius);
        let c1 = vec2(cx + a1.cos() * radius, cy + a1.sin() * radius);

        let e0 = vec2(cx + a0.cos() * far, cy + a0.sin() * far);
        let e1 = vec2(cx + a1.cos() * far, cy + a1.sin() * far);

        draw_triangle(c0, c1, e0, color);
        draw_triangle(c1, e0, e1, color);
    }

    draw_circle_lines(cx, cy, radius, 3.0, Color::from_rgba(40, 40, 40, alpha));
    draw_circle_lines(
        cx,
        cy,
        radius - 1.5,
        1.0,
        Color::from_rgba(80, 80, 80, alpha),
    );
    draw_circle(
        cx,
        cy,
        radius - 2.0,
        Color::from_rgba(0, 10, 20, ((alpha as f32) * 0.12) as u8),
    );
}
