use crate::ui::draw_arc;
use crate::ui::fonts::text_params;
use crate::weapons::crosshair::CrosshairType;
use crate::weapons::stats::WeaponCategory;
use crate::weapons::WeaponState;
use macroquad::prelude::*;
use std::collections::HashMap;

use super::super::shared::CrosshairSettingsPreview;

pub(super) fn draw_crosshair_preview(
    weapon: &WeaponState,
    state: &CrosshairSettingsPreview,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    assets: &HashMap<String, Texture2D>,
    s: f32,
) {
    const PREVIEW_BG_TEXTURE_KEY: &str = "crosshair_preview_bg";

    if let Some(bg_texture) = assets.get(PREVIEW_BG_TEXTURE_KEY) {
        draw_texture_ex(
            bg_texture,
            x,
            y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(w, h)),
                ..Default::default()
            },
        );
    } else {
        let bg_color = if state.preview_light_bg {
            Color::from_rgba(200, 200, 200, 255)
        } else {
            Color::from_rgba(20, 20, 20, 255)
        };
        draw_rectangle(x, y, w, h, bg_color);
    }
    draw_rectangle_lines(x, y, w, h, 2.0, WHITE);

    let cx = x + w / 2.0;
    let cy = y + h / 2.0;

    if weapon.id.category() == WeaponCategory::Sniper {
        let fs = (14.0 * s) as u16;
        draw_text_ex(
            "Scope preview coming soon",
            cx - 80.0 * s,
            cy,
            text_params(fs, YELLOW),
        );
        return;
    }

    let xhair = &weapon.crosshair;
    let sh = screen_height();
    let ads_progress = if state.preview_ads { 1.0 } else { 0.0 };
    let color = xhair.color;

    let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - ads_progress);
    let ads_rest_gap = ((xhair.center_gap * sh) / 100.0) * 0.7 * ads_progress;

    match xhair.crosshair_type {
        CrosshairType::FourCurves => {
            let radius = hip_gap + ads_rest_gap;
            let thick = (xhair.line_thickness * sh) / 100.0;
            let gap_angle: f32 = 20.0_f32.to_radians();
            let quarter = std::f32::consts::FRAC_PI_2;
            let segs = 32;

            if xhair.outline_enabled {
                let ot = thick + (xhair.outline_thickness * sh) / 100.0 * 2.0;
                let oc = Color::from_rgba(0, 0, 0, 180);
                for q in 0..4 {
                    let base_angle = q as f32 * quarter;
                    draw_arc(cx, cy, radius, base_angle + gap_angle, base_angle + quarter - gap_angle, ot, segs, oc);
                }
            }
            for q in 0..4 {
                let base_angle = q as f32 * quarter;
                draw_arc(cx, cy, radius, base_angle + gap_angle, base_angle + quarter - gap_angle, thick, segs, color);
            }
            if xhair.dot_enabled {
                let dot_alpha = ((1.0 - ads_progress) * 255.0) as u8;
                let dot_color = Color::from_rgba((color.r * 255.0) as u8, (color.g * 255.0) as u8, (color.b * 255.0) as u8, dot_alpha);
                draw_circle(cx, cy, (xhair.dot_size * sh) / 100.0, dot_color);
            }
        }
        CrosshairType::Chevron => {
            let radius = hip_gap + ads_rest_gap;
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
                let p0x = h_near; let p0y = corner_y;
                let p1x = corner_x; let p1y = corner_y;
                let p2x = corner_x; let p2y = v_near;
                let bez = |t: f32| -> (f32, f32) {
                    let u = 1.0 - t;
                    (u * u * p0x + 2.0 * u * t * p1x + t * t * p2x, u * u * p0y + 2.0 * u * t * p1y + t * t * p2y)
                };
                let t_a = 0.10_f32; let t_b = 0.90_f32;
                let (mid_ax, mid_ay) = bez(t_a);
                let (mid_bx, mid_by) = bez(t_b);
                let chamfer_segs = 4;
                let draw_corner_shape = |thickness: f32, col: Color| {
                    for i in 0..chamfer_segs {
                        let s0 = t_a * (i as f32 / chamfer_segs as f32);
                        let s1 = t_a * ((i + 1) as f32 / chamfer_segs as f32);
                        let (x0, y0) = bez(s0); let (x1, y1) = bez(s1);
                        draw_line(x0, y0, x1, y1, thickness, col);
                    }
                    draw_line(mid_ax, mid_ay, mid_bx, mid_by, thickness, col);
                    for i in 0..chamfer_segs {
                        let s0 = t_b + (1.0 - t_b) * (i as f32 / chamfer_segs as f32);
                        let s1 = t_b + (1.0 - t_b) * ((i + 1) as f32 / chamfer_segs as f32);
                        let (x0, y0) = bez(s0); let (x1, y1) = bez(s1);
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
                let dot_alpha = ((1.0 - ads_progress) * 255.0) as u8;
                let dot_color = Color::from_rgba((color.r * 255.0) as u8, (color.g * 255.0) as u8, (color.b * 255.0) as u8, dot_alpha);
                draw_circle(cx, cy, (xhair.dot_size * sh) / 100.0, dot_color);
            }
        }
        CrosshairType::Cross => {
            let gap = hip_gap + ads_rest_gap;
            let len = (xhair.line_length * sh) / 100.0;
            let thick = (xhair.line_thickness * sh) / 100.0;
            if xhair.outline_enabled {
                let ot = thick + (xhair.outline_thickness * sh) / 100.0 * 2.0;
                let oc = Color::from_rgba(0, 0, 0, 180);
                draw_rectangle(cx - ot / 2.0, cy - gap - len, ot, len, oc);
                draw_rectangle(cx - ot / 2.0, cy + gap, ot, len, oc);
                draw_rectangle(cx - gap - len, cy - ot / 2.0, len, ot, oc);
                draw_rectangle(cx + gap, cy - ot / 2.0, len, ot, oc);
            }
            draw_rectangle(cx - thick / 2.0, cy - gap - len, thick, len, color);
            draw_rectangle(cx - thick / 2.0, cy + gap, thick, len, color);
            draw_rectangle(cx - gap - len, cy - thick / 2.0, len, thick, color);
            draw_rectangle(cx + gap, cy - thick / 2.0, len, thick, color);
            if xhair.dot_enabled {
                let dot_alpha = ((1.0 - ads_progress) * 255.0) as u8;
                let dot_color = Color::from_rgba((color.r * 255.0) as u8, (color.g * 255.0) as u8, (color.b * 255.0) as u8, dot_alpha);
                draw_circle(cx, cy, (xhair.dot_size * sh) / 100.0, dot_color);
            }
        }
        CrosshairType::Circle => {
            let gap = hip_gap + ads_rest_gap;
            let thick = (xhair.line_thickness * sh) / 100.0;
            if xhair.outline_enabled {
                let ot = thick + (xhair.outline_thickness * sh) / 100.0 * 2.0;
                draw_circle_lines(cx, cy, gap, ot, Color::from_rgba(0, 0, 0, 180));
            }
            draw_circle_lines(cx, cy, gap, thick, color);
            if xhair.dot_enabled {
                draw_circle(cx, cy, (xhair.dot_size * sh) / 100.0, color);
            }
        }
        CrosshairType::Square => {
            let size = (hip_gap + ads_rest_gap) * 2.0;
            let thick = (xhair.line_thickness * sh) / 100.0;
            if xhair.outline_enabled {
                let ot = thick + (xhair.outline_thickness * sh) / 100.0 * 2.0;
                draw_rectangle_lines(cx - size / 2.0, cy - size / 2.0, size, size, ot, Color::from_rgba(0, 0, 0, 180));
            }
            draw_rectangle_lines(cx - size / 2.0, cy - size / 2.0, size, size, thick, color);
            if xhair.dot_enabled {
                draw_circle(cx, cy, (xhair.dot_size * sh) / 100.0, color);
            }
        }
        CrosshairType::Triangle => {
            let size = (hip_gap + ads_rest_gap) * 2.0;
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
            let gap = hip_gap + ads_rest_gap;
            let arm = ((xhair.line_length) * sh) / 100.0;
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
                let dot_alpha = ((1.0 - ads_progress) * 255.0) as u8;
                let dot_color = Color::from_rgba((color.r * 255.0) as u8, (color.g * 255.0) as u8, (color.b * 255.0) as u8, dot_alpha);
                draw_circle(cx, cy, (xhair.dot_size * sh) / 100.0, dot_color);
            }
        }
    }

}
