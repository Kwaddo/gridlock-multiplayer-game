use macroquad::prelude::*;

use crate::ui::draw_arc;
use crate::ui::fonts::{measure_game_text, text_params};
use crate::weapons::crosshair::CrosshairType;
use crate::weapons::effects::DamageNumber;
use crate::weapons::stats::CrosshairStyle;
use crate::weapons::WeaponInventory;

use super::scope::draw_scope_vignette;

pub(super) fn draw_crosshair_and_combat_feedback(
    cx: f32,
    cy: f32,
    weapons: &WeaponInventory,
    hit_marker_timer: f32,
    hit_marker_headshot: bool,
    damage_numbers: &[DamageNumber],
    hit_marker_pellet_mask: u16,
) {
    let weapon = weapons.active();
    let stats = weapon.stats();

    let sh = screen_height();
    let xhair = &weapon.crosshair;
    let ads_t = weapon.ads_progress;

    let fsa_active = stats.fsa_time.map_or(false, |t| weapon.fsa_timer >= t);
    let color = if fsa_active && weapon.ads_progress > 0.5 {
        Color::from_rgba(180, 180, 180, 255)
    } else {
        xhair.color
    };

    let _tint = if weapon.muzzle_flash_timer > 0.04 {
        Color::from_rgba(255, 255, 200, 255)
    } else {
        WHITE
    };

    match stats.crosshair_style {
        CrosshairStyle::Standard => match xhair.crosshair_type {
            CrosshairType::FourCurves => {
                let stats = weapon.stats();
                let _base =
                    stats.hip_spread + (stats.ads_spread - stats.hip_spread) * weapon.ads_progress;

                let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - weapon.ads_progress);
                let kick_gap = (weapon.crosshair_kick * sh) / 100.0;
                let movement_gap = 0.0;
                let ads_rest_gap = if fsa_active {
                    (sh * 0.3) / 100.0
                } else {
                    ((xhair.center_gap * sh) / 100.0) * 0.4 * weapon.ads_progress
                };

                let radius = hip_gap + kick_gap + ads_rest_gap + movement_gap;
                let thick = (xhair.line_thickness * sh) / 100.0;
                let gap_angle: f32 = 20.0_f32.to_radians();
                let quarter = std::f32::consts::FRAC_PI_2;
                let segs = 32u32;

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
                let stats = weapon.stats();
                let _base =
                    stats.hip_spread + (stats.ads_spread - stats.hip_spread) * weapon.ads_progress;

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
                let arm = ((xhair.line_length / 2.0) * sh) / 100.0;

                let corners: [(f32, f32); 4] = [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)];

                for &(sx, sy) in &corners {
                    let corner_x = cx + sx * radius;
                    let corner_y = cy + sy * radius;
                    let h_end_x = corner_x - sx * arm;
                    let v_end_y = corner_y - sy * arm;

                    let chamfer = arm * 0.35;
                    let h_near = corner_x - sx * chamfer;
                    let v_near = corner_y - sy * chamfer;

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

                    let t_a = 0.10_f32;
                    let t_b = 0.90_f32;
                    let (mid_ax, mid_ay) = bez(t_a);
                    let (mid_bx, mid_by) = bez(t_b);
                    let chamfer_segs = 4;

                    let draw_corner_shape = |thickness: f32, col: Color| {
                        for i in 0..chamfer_segs {
                            let s0 = t_a * (i as f32 / chamfer_segs as f32);
                            let s1 = t_a * ((i + 1) as f32 / chamfer_segs as f32);
                            let (x0, y0) = bez(s0);
                            let (x1, y1) = bez(s1);
                            draw_line(x0, y0, x1, y1, thickness, col);
                        }
                        draw_line(mid_ax, mid_ay, mid_bx, mid_by, thickness, col);
                        for i in 0..chamfer_segs {
                            let s0 = t_b + (1.0 - t_b) * (i as f32 / chamfer_segs as f32);
                            let s1 = t_b + (1.0 - t_b) * ((i + 1) as f32 / chamfer_segs as f32);
                            let (x0, y0) = bez(s0);
                            let (x1, y1) = bez(s1);
                            draw_line(x0, y0, x1, y1, thickness, col);
                        }
                    };

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
                let stats = weapon.stats();
                let base =
                    stats.hip_spread + (stats.ads_spread - stats.hip_spread) * weapon.ads_progress;

                let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - weapon.ads_progress);
                let kick_gap = (weapon.crosshair_kick * sh) / 100.0;

                let movement_gap = ((weapon.computed_spread - base).max(0.0) * 40.0 * sh) / 100.0;

                let ads_rest_gap = if fsa_active {
                    (sh * 0.3) / 100.0
                } else {
                    ((xhair.center_gap * sh) / 100.0) * 0.7 * weapon.ads_progress
                };

                let gap = hip_gap + kick_gap + ads_rest_gap + movement_gap;

                let length = (xhair.line_length * sh) / 100.0;
                let thick = (xhair.line_thickness * sh) / 100.0;

                draw_rectangle(cx - thick / 2.0, cy - gap - length, thick, length, color);
                draw_rectangle(cx - thick / 2.0, cy + gap, thick, length, color);
                draw_rectangle(cx - gap - length, cy - thick / 2.0, length, thick, color);
                draw_rectangle(cx + gap, cy - thick / 2.0, length, thick, color);

                if xhair.dot_enabled {
                    draw_circle(cx, cy, (xhair.dot_size * sh) / 100.0, color);
                }
            }
            CrosshairType::Circle => {
                let stats = weapon.stats();
                let base =
                    stats.hip_spread + (stats.ads_spread - stats.hip_spread) * weapon.ads_progress;

                let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - weapon.ads_progress);
                let kick_gap = (weapon.crosshair_kick * sh) / 100.0;
                let movement_gap = ((weapon.computed_spread - base).max(0.0) * 40.0 * sh) / 100.0;
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
                let stats = weapon.stats();
                let base =
                    stats.hip_spread + (stats.ads_spread - stats.hip_spread) * weapon.ads_progress;

                let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - weapon.ads_progress);
                let kick_gap = (weapon.crosshair_kick * sh) / 100.0;
                let movement_gap = ((weapon.computed_spread - base).max(0.0) * 40.0 * sh) / 100.0;
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
                draw_rectangle_lines(cx - size / 2.0, cy - size / 2.0, size, size, thick, color);

                if xhair.dot_enabled {
                    draw_circle(cx, cy, (xhair.dot_size * sh) / 100.0, color);
                }
            }
            CrosshairType::Triangle => {
                let stats = weapon.stats();
                let base =
                    stats.hip_spread + (stats.ads_spread - stats.hip_spread) * weapon.ads_progress;

                let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - weapon.ads_progress);
                let kick_gap = (weapon.crosshair_kick * sh) / 100.0;
                let movement_gap = ((weapon.computed_spread - base).max(0.0) * 40.0 * sh) / 100.0;
                let ads_rest_gap = if fsa_active {
                    (sh * 0.3) / 100.0
                } else {
                    ((xhair.center_gap * sh) / 100.0) * 0.7 * weapon.ads_progress
                };

                let size = (hip_gap + kick_gap + ads_rest_gap + movement_gap) * 2.0;
                let thick = (xhair.line_thickness * sh) / 100.0;

                let height = size * 0.866;
                let v1 = (cx, cy - height * 0.67);
                let v2 = (cx - size / 2.0, cy + height * 0.33);
                let v3 = (cx + size / 2.0, cy + height * 0.33);

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
                let stats = weapon.stats();
                let _base =
                    stats.hip_spread + (stats.ads_spread - stats.hip_spread) * weapon.ads_progress;

                let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - weapon.ads_progress);
                let kick_gap = (weapon.crosshair_kick * sh) / 100.0;
                let movement_gap = 0.0;
                let ads_rest_gap = if fsa_active {
                    (sh * 0.3) / 100.0
                } else {
                    ((xhair.center_gap * sh) / 100.0) * 0.4 * weapon.ads_progress
                };

                let gap = hip_gap + kick_gap + ads_rest_gap + movement_gap;
                let arm = (xhair.line_length * sh) / 100.0;
                let thick = (xhair.line_thickness * sh) / 100.0;

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
        },
        CrosshairStyle::Scope => {
            if ads_t > 0.2 {
                let scope_t = ((ads_t - 0.2) / 0.8).min(1.0);
                let scope_alpha = (scope_t * 255.0) as u8;

                let scope_radius = screen_height().min(screen_width()) * 0.38;
                draw_scope_vignette(cx, cy, scope_radius, scope_alpha);

                let line_color = Color::from_rgba(0, 0, 0, scope_alpha);
                let thin = 1.5;
                draw_line(cx - scope_radius, cy, cx - 8.0, cy, thin, line_color);
                draw_line(cx + 8.0, cy, cx + scope_radius, cy, thin, line_color);
                draw_line(cx, cy - scope_radius, cx, cy - 8.0, thin, line_color);
                draw_line(cx, cy + 8.0, cx, cy + scope_radius, thin, line_color);
                draw_circle(cx, cy, 2.0, Color::from_rgba(255, 50, 50, scope_alpha));
            }
        }
    }

    if hit_marker_timer > 0.0 {
        let t = (hit_marker_timer / 0.35).min(1.0);
        let alpha = (t * 255.0) as u8;
        let hm_color = if hit_marker_headshot {
            Color::from_rgba(255, 215, 0, alpha)
        } else {
            Color::from_rgba(255, 255, 255, alpha)
        };

        let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - ads_t);
        let ads_rest_gap = ((xhair.center_gap * sh) / 100.0) * 0.4 * ads_t;
        let gap = hip_gap + ads_rest_gap;
        let len = (xhair.line_length * sh) / 100.0;
        let current_size = (gap + len) * 2.0;

        let current_size = if current_size < 10.0 {
            (sh * 3.0) / 100.0
        } else {
            current_size
        };

        let pop = 1.0 + 0.4 * t;
        let hm_thick = 2.5;
        let stats = weapon.stats();
        let pellet_spread = stats.pellet_spread;
        if let Some(ptrn) = pellet_spread {
            let dot_radius = 5.0;
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

    for dmg in damage_numbers {
        let alpha = (dmg.lifetime.min(1.0) * 255.0) as u8;
        let (color, size) = if dmg.is_headshot {
            (Color::from_rgba(255, 215, 0, alpha), 28)
        } else {
            (Color::from_rgba(255, 255, 255, alpha), 22)
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
}
