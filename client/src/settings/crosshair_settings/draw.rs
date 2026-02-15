use std::collections::HashMap;

use crate::ui::draw_arc;
use crate::ui::fonts::{measure_game_text, text_params};
use crate::weapons::crosshair::CrosshairType;
use crate::weapons::crosshair_presets::CrosshairPresetManager;
use crate::weapons::state::{WeaponInventory, WeaponState};
use crate::weapons::stats::WeaponKind;
use macroquad::prelude::*;

use super::color::hsv_to_rgb;
use super::state::CrosshairSettingsPreview;
use super::types::crosshair_type_name;
use super::widgets::{
    draw_dropdown_display, draw_section_header, draw_slider_display, draw_toggle_display,
};

pub fn draw_crosshair_settings(
    state: &CrosshairSettingsPreview,
    weapons: &WeaponInventory,
    presets: &CrosshairPresetManager,
    x: f32,
    y: f32,
    w: f32,
    _h: f32,
    assets: &HashMap<String, Texture2D>,
    mouse: (f32, f32),
) {
    draw_section_header("Crosshair Settings", x + 20.0, y + 20.0);

    // Weapon selector buttons
    let btn_y = y + 60.0;
    let btn_w = 100.0;
    let btn_h = 35.0;
    let btn_gap = 10.0;

    for (i, kind) in WeaponKind::ALL.iter().enumerate() {
        let btn_x = x + (btn_w + btn_gap) * (i as f32);
        let btn_color = if kind == &state.preview_weapon {
            Color::from_rgba(80, 150, 220, 255)
        } else {
            Color::from_rgba(40, 40, 40, 255)
        };
        draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_color);
        draw_rectangle_lines(btn_x, btn_y, btn_w, btn_h, 1.0, GRAY);
        let name = kind.stats().name;
        let text_w = measure_game_text(name, 14);
        draw_text_ex(
            name,
            btn_x + (btn_w - text_w) / 2.0,
            btn_y + 22.0,
            text_params(14, WHITE),
        );
    }

    if state.preview_weapon == WeaponKind::Sniper {
        let sniper_y = btn_y + btn_h + 60.0;
        draw_text_ex(
            "Sniper scope customization coming soon",
            x + 20.0,
            sniper_y,
            text_params(16, YELLOW),
        );
        return;
    }

    let left_w = (w * 0.55) - 20.0;

    // Preset row: dropdown + delete
    let preset_y = btn_y + btn_h + 15.0;
    let preset_dropdown_w = left_w - 100.0;
    let preset_btn_w = 75.0;
    let preset_btn_gap = 5.0;

    // Preset dropdown
    let preset_name = if state.selected_preset < presets.presets.len() {
        &presets.presets[state.selected_preset].name
    } else {
        "None"
    };
    let is_custom = state.selected_preset < presets.presets.len()
        && !presets.presets[state.selected_preset].is_default;

    draw_rectangle(
        x,
        preset_y,
        preset_dropdown_w,
        30.0,
        Color::from_rgba(35, 35, 35, 255),
    );
    draw_rectangle_lines(
        x,
        preset_y,
        preset_dropdown_w,
        30.0,
        1.0,
        Color::from_rgba(80, 80, 80, 255),
    );
    let preset_tw = measure_game_text(preset_name, 14);
    draw_text_ex(
        preset_name,
        x + (preset_dropdown_w - preset_tw) / 2.0,
        preset_y + 20.0,
        text_params(14, WHITE),
    );
    draw_text_ex("<", x + 6.0, preset_y + 20.0, text_params(14, GRAY));
    let arrow_r = measure_game_text(">", 14);
    draw_text_ex(
        ">",
        x + preset_dropdown_w - arrow_r - 6.0,
        preset_y + 20.0,
        text_params(14, GRAY),
    );

    // "Delete" button
    let delete_x = x + preset_dropdown_w + preset_btn_gap;
    let del_color = if is_custom {
        Color::from_rgba(80, 30, 30, 255)
    } else {
        Color::from_rgba(30, 30, 30, 255)
    };
    draw_rectangle(delete_x, preset_y, preset_btn_w, 30.0, del_color);
    draw_rectangle_lines(delete_x, preset_y, preset_btn_w, 30.0, 1.0, GRAY);
    let del_tw = measure_game_text("Delete", 12);
    let del_text_color = if is_custom {
        WHITE
    } else {
        Color::from_rgba(80, 80, 80, 255)
    };
    draw_text_ex(
        "Delete",
        delete_x + (preset_btn_w - del_tw) / 2.0,
        preset_y + 19.0,
        text_params(12, del_text_color),
    );

    // Sliders
    let slider_x = x;
    let mut slider_y = preset_y + 45.0;
    let slider_w = left_w;

    let weapon = &weapons.weapons[state.preview_weapon.slot() - 1];
    let ch = &weapon.crosshair;

    draw_toggle_display("Enable dot", ch.dot_enabled, slider_x, slider_y);
    slider_y += 28.0;
    draw_slider_display(
        "Gap",
        ch.center_gap,
        0.0,
        10.0,
        slider_x,
        slider_y,
        slider_w,
        None,
    );
    slider_y += 28.0;
    draw_slider_display(
        "Length",
        ch.line_length,
        0.0,
        10.0,
        slider_x,
        slider_y,
        slider_w,
        None,
    );
    slider_y += 28.0;
    draw_slider_display(
        "Thickness",
        ch.line_thickness,
        0.0,
        2.0,
        slider_x,
        slider_y,
        slider_w,
        None,
    );
    slider_y += 28.0;
    draw_slider_display(
        "Dot size",
        ch.dot_size,
        0.0,
        5.0,
        slider_x,
        slider_y,
        slider_w,
        None,
    );
    slider_y += 33.0;
    draw_toggle_display("Outline", ch.outline_enabled, slider_x, slider_y);
    slider_y += 28.0;
    draw_slider_display(
        "Thickness",
        ch.outline_thickness,
        0.0,
        2.0,
        slider_x,
        slider_y,
        slider_w,
        None,
    );
    slider_y += 33.0;

    // Crosshair type display
    let type_name = crosshair_type_name(ch.crosshair_type);
    draw_dropdown_display("Crosshair Type", type_name, slider_x, slider_y, slider_w);
    slider_y += 40.0;

    // Color picker (display only; input handled in update)
    draw_text_ex(
        "COLOR",
        slider_x,
        slider_y + 14.0,
        text_params(14, Color::from_rgba(100, 180, 255, 255)),
    );
    slider_y += 20.0;

    let picker_size = 140.0;
    let hue_bar_w = 20.0;
    let sv_x = slider_x;
    let sv_y = slider_y;

    let steps = 20u32;
    let cell_w = picker_size / steps as f32;
    let cell_h = picker_size / steps as f32;
    for sy in 0..steps {
        for sx in 0..steps {
            let s = (sx as f32 + 0.5) / steps as f32;
            let v = 1.0 - (sy as f32 + 0.5) / steps as f32;
            let (r, g, b) = hsv_to_rgb(state.hue, s, v);
            draw_rectangle(
                sv_x + sx as f32 * cell_w,
                sv_y + sy as f32 * cell_h,
                cell_w + 1.0,
                cell_h + 1.0,
                Color::new(r, g, b, 1.0),
            );
        }
    }
    draw_rectangle_lines(sv_x, sv_y, picker_size, picker_size, 1.0, GRAY);

    let cursor_x = sv_x + state.sat * picker_size;
    let cursor_y = sv_y + (1.0 - state.val) * picker_size;
    draw_circle_lines(cursor_x, cursor_y, 5.0, 2.0, WHITE);
    draw_circle_lines(cursor_x, cursor_y, 4.0, 1.0, BLACK);

    let hue_x = sv_x + picker_size + 10.0;
    let hue_steps = 36u32;
    let hue_cell_h = picker_size / hue_steps as f32;
    for i in 0..hue_steps {
        let h = (i as f32 / hue_steps as f32) * 360.0;
        let (r, g, b) = hsv_to_rgb(h, 1.0, 1.0);
        draw_rectangle(
            hue_x,
            sv_y + i as f32 * hue_cell_h,
            hue_bar_w,
            hue_cell_h + 1.0,
            Color::new(r, g, b, 1.0),
        );
    }
    draw_rectangle_lines(hue_x, sv_y, hue_bar_w, picker_size, 1.0, GRAY);

    let hue_cursor_y = sv_y + (state.hue / 360.0) * picker_size;
    draw_rectangle(hue_x - 2.0, hue_cursor_y - 2.0, hue_bar_w + 4.0, 4.0, WHITE);
    draw_rectangle_lines(
        hue_x - 2.0,
        hue_cursor_y - 2.0,
        hue_bar_w + 4.0,
        4.0,
        1.0,
        BLACK,
    );

    let swatch_x = hue_x + hue_bar_w + 15.0;
    let swatch_size = 40.0;
    let check = 10.0;
    for cy_i in 0..4 {
        for cx_i in 0..4 {
            let col = if (cx_i + cy_i) % 2 == 0 {
                Color::from_rgba(180, 180, 180, 255)
            } else {
                Color::from_rgba(120, 120, 120, 255)
            };
            draw_rectangle(
                swatch_x + cx_i as f32 * check,
                sv_y + cy_i as f32 * check,
                check,
                check,
                col,
            );
        }
    }
    let (pr, pg, pb) = hsv_to_rgb(state.hue, state.sat, state.val);
    draw_rectangle(
        swatch_x,
        sv_y,
        swatch_size,
        swatch_size,
        Color::new(pr, pg, pb, state.alpha_val / 255.0),
    );
    draw_rectangle_lines(swatch_x, sv_y, swatch_size, swatch_size, 1.0, WHITE);

    let alpha_y = sv_y + picker_size + 10.0;
    draw_slider_display(
        "Alpha",
        state.alpha_val,
        0.0,
        255.0,
        slider_x,
        alpha_y,
        picker_size + hue_bar_w + 10.0,
        Some(Color::from_rgba(200, 200, 200, 255)),
    );

    // Preview box
    let preview_x = x + w * 0.55 + 20.0;
    let preview_y = btn_y + btn_h + 15.0;
    let preview_w = w * 0.45 - 40.0;
    let preview_h = 300.0;

    draw_crosshair_preview(
        weapon, state, preview_x, preview_y, preview_w, preview_h, assets,
    );

    // Preview buttons
    let btn_row_y = preview_y + preview_h + 10.0;
    let small_btn_w = 90.0;
    let small_btn_h = 30.0;
    let small_gap = 8.0;

    let ads_color = if state.preview_ads {
        Color::from_rgba(80, 150, 220, 255)
    } else {
        Color::from_rgba(40, 40, 40, 255)
    };
    draw_rectangle(preview_x, btn_row_y, small_btn_w, small_btn_h, ads_color);
    draw_rectangle_lines(preview_x, btn_row_y, small_btn_w, small_btn_h, 1.0, GRAY);
    let ads_text = if state.preview_ads {
        "ADS: ON"
    } else {
        "ADS: OFF"
    };
    let ads_tw = measure_game_text(ads_text, 12);
    draw_text_ex(
        ads_text,
        preview_x + (small_btn_w - ads_tw) / 2.0,
        btn_row_y + 20.0,
        text_params(12, WHITE),
    );

    let bg_btn_x = preview_x + small_btn_w + small_gap;
    let bg_color = if state.preview_light_bg {
        Color::from_rgba(180, 180, 180, 255)
    } else {
        Color::from_rgba(40, 40, 40, 255)
    };
    draw_rectangle(bg_btn_x, btn_row_y, small_btn_w, small_btn_h, bg_color);
    draw_rectangle_lines(bg_btn_x, btn_row_y, small_btn_w, small_btn_h, 1.0, GRAY);
    let bg_text = if state.preview_light_bg {
        "Light"
    } else {
        "Dark"
    };
    let bg_tw = measure_game_text(bg_text, 12);
    let bg_text_color = if state.preview_light_bg { BLACK } else { WHITE };
    draw_text_ex(
        bg_text,
        bg_btn_x + (small_btn_w - bg_tw) / 2.0,
        btn_row_y + 20.0,
        text_params(12, bg_text_color),
    );

    let reset_btn_x = bg_btn_x + small_btn_w + small_gap;
    draw_rectangle(
        reset_btn_x,
        btn_row_y,
        small_btn_w,
        small_btn_h,
        Color::from_rgba(60, 40, 40, 255),
    );
    draw_rectangle_lines(reset_btn_x, btn_row_y, small_btn_w, small_btn_h, 1.0, GRAY);
    let reset_tw = measure_game_text("Reset", 12);
    draw_text_ex(
        "Reset",
        reset_btn_x + (small_btn_w - reset_tw) / 2.0,
        btn_row_y + 20.0,
        text_params(12, WHITE),
    );

    if state.preview_weapon == WeaponKind::Shotgun && state.shotgun_type_warning {
        draw_shotgun_type_warning(preview_x, btn_row_y, 0.0, 0.0, mouse, false);
    }
}

fn draw_crosshair_preview(
    weapon: &WeaponState,
    state: &CrosshairSettingsPreview,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    assets: &HashMap<String, Texture2D>,
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
    draw_rectangle_lines(x, y, w, h, 2.0, GRAY);

    let cx = x + w / 2.0;
    let cy = y + h / 2.0;

    if weapon.kind == WeaponKind::Sniper {
        draw_text_ex(
            "Scope preview coming soon",
            cx - 80.0,
            cy,
            text_params(14, YELLOW),
        );
        return;
    }

    let xhair = &weapon.crosshair;
    let sh = screen_height();
    let ads_progress = if state.preview_ads { 1.0 } else { 0.0 };
    let color = xhair.color;

    let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - ads_progress);
    let ads_rest_gap = ((xhair.center_gap * sh) / 100.0) * 0.4 * ads_progress;

    match xhair.crosshair_type {
        CrosshairType::FourCurves => {
            let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - ads_progress);
            let ads_rest_gap = ((xhair.center_gap * sh) / 100.0) * 0.4 * ads_progress;
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
            for q in 0..4 {
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
                let dot_alpha = ((1.0 - ads_progress) * 255.0) as u8;
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
            let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - ads_progress);
            let ads_rest_gap = ((xhair.center_gap * sh) / 100.0) * 0.4 * ads_progress;
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
                let dot_alpha = ((1.0 - ads_progress) * 255.0) as u8;
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
            let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - ads_progress);
            let ads_rest_gap = ((xhair.center_gap * sh) / 100.0) * 0.4 * ads_progress;
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
                let dot_color = Color::from_rgba(
                    (color.r * 255.0) as u8,
                    (color.g * 255.0) as u8,
                    (color.b * 255.0) as u8,
                    dot_alpha,
                );
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

    let name_color = if state.preview_light_bg { BLACK } else { WHITE };
    draw_text_ex(
        weapon.kind.stats().name,
        x + 10.0,
        y + 20.0,
        text_params(16, name_color),
    );
}

pub fn draw_shotgun_type_warning(
    x: f32,
    y: f32,
    _btn_w: f32,
    _btn_h: f32,
    mouse: (f32, f32),
    clicked: bool,
) -> bool {
    let box_w = 340.0;
    let box_h = 80.0;
    let box_x = x;
    let box_y = y - box_h - 10.0;

    draw_rectangle(
        box_x,
        box_y,
        box_w,
        box_h,
        Color::from_rgba(60, 30, 10, 240),
    );
    draw_rectangle_lines(
        box_x,
        box_y,
        box_w,
        box_h,
        2.0,
        Color::from_rgba(255, 160, 0, 255),
    );
    draw_text_ex(
        "Switching to a non-standard crosshair",
        box_x + 10.0,
        box_y + 22.0,
        text_params(14, Color::from_rgba(255, 200, 80, 255)),
    );
    draw_text_ex(
        "for shotgun may affect gameplay.",
        box_x + 10.0,
        box_y + 42.0,
        text_params(14, Color::from_rgba(255, 200, 80, 255)),
    );

    let ok_w = 50.0;
    let ok_h = 25.0;
    let ok_x = box_x + box_w - ok_w - 10.0;
    let ok_y = box_y + box_h - ok_h - 8.0;

    draw_rectangle(ok_x, ok_y, ok_w, ok_h, Color::from_rgba(80, 150, 220, 255));
    draw_rectangle_lines(ok_x, ok_y, ok_w, ok_h, 1.0, GRAY);
    let ok_tw = measure_game_text("OK", 14);
    draw_text_ex(
        "OK",
        ok_x + (ok_w - ok_tw) / 2.0,
        ok_y + 17.0,
        text_params(14, WHITE),
    );

    clicked
        && mouse.0 >= ok_x
        && mouse.0 <= ok_x + ok_w
        && mouse.1 >= ok_y
        && mouse.1 <= ok_y + ok_h
}
