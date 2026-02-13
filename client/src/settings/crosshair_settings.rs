/*
    This module contains the crosshair settings, this will allow players to 
    customize their own crosshairs, it will import the crosshair config and the draw function from draw_hud
    so that we may have live preview, it will also import weapon kind to allow per-weapon crosshair
    and allows you to preview ADS and spread patterns.
*/

use macroquad::prelude::*;
use crate::weapons::WeaponState;
use crate::weapons::{ WeaponInventory, crosshair::{ CrosshairConfig, CrosshairType } };
use crate::weapons::stats::WeaponKind;
use std::collections::HashMap;
use crate::ui::fonts::{text_params, measure_game_text};

const CROSSHAIR_TYPES: &[CrosshairType] = &[
    CrosshairType::Cross,
    CrosshairType::FourCurves,
    CrosshairType::Chevron,
];

fn crosshair_type_name(ct: CrosshairType) -> &'static str {
    match ct {
        CrosshairType::Cross => "Cross",
        CrosshairType::FourCurves => "4-Curves",
        CrosshairType::Chevron => "Chevron",
    }
}

pub struct CrosshairSettingsPreview {
    pub preview_weapon: WeaponKind,
    pub preview_ads: bool,
    pub preview_light_bg: bool,
    pub shotgun_type_warning: bool,
    pub shotgun_warning_shown_this_visit: bool,
}

impl CrosshairSettingsPreview {
    pub fn new() -> Self {
        Self {
            preview_weapon: WeaponKind::Pistol,
            preview_ads: false,
            preview_light_bg: false,
            shotgun_type_warning: false,
            shotgun_warning_shown_this_visit: false,
        }
    }

    pub fn reset_visit(&mut self) {
        self.shotgun_warning_shown_this_visit = false;
    }
}

pub fn update_crosshair_settings(
    state: &mut CrosshairSettingsPreview,
    weapons: &mut WeaponInventory,
    x: f32,
    y: f32,
    w: f32,
    _h: f32,
    mouse: (f32, f32),
    clicked: bool,
    held: bool,
) {
    // Weapon selector buttons
    let btn_y = y + 60.0;
    let btn_w = 100.0;
    let btn_h = 35.0;
    let btn_gap = 10.0;

    for (i, kind) in WeaponKind::ALL.iter().enumerate() {
        let btn_x = x + (btn_w + btn_gap) * (i as f32);
        if clicked && mouse.0 >= btn_x && mouse.0 <= btn_x + btn_w &&
           mouse.1 >= btn_y && mouse.1 <= btn_y + btn_h
        {
            state.preview_weapon = *kind;
        }
    }

    let slider_x = x + 20.0;
    let mut slider_y = btn_y + btn_h + 40.0;
    let slider_w = (w * 0.5) - 40.0;

    if state.preview_weapon == WeaponKind::Sniper {
        // TODO: Sniper scope customization
        draw_text_ex("TODO", slider_x, slider_y + 100.0, text_params(18, YELLOW));
        return;
    }

    // crosshair property sliders — modify the PREVIEW weapon, not the equipped one
    let weapon = &mut weapons.weapons[state.preview_weapon.slot() - 1];
    let ch = &mut weapon.crosshair;

    slider_y += draw_toggle("Enable dot", &mut ch.dot_enabled, slider_x, slider_y, mouse, clicked);
    slider_y += draw_slider("Gap", &mut ch.center_gap, 0.0, 10.0, 0.1, slider_x, slider_y, slider_w, mouse, clicked, held);
    slider_y += draw_slider("Length", &mut ch.line_length, 0.0, 10.0, 0.1, slider_x, slider_y, slider_w, mouse, clicked, held);
    slider_y += draw_slider("Thickness", &mut ch.line_thickness, 0.0, 2.0, 0.05, slider_x, slider_y, slider_w, mouse, clicked, held);
    slider_y += draw_slider("Dot size", &mut ch.dot_size, 0.0, 5.0, 0.1, slider_x, slider_y, slider_w, mouse, clicked, held);
    slider_y += 10.0;
    slider_y += draw_toggle("Outline", &mut ch.outline_enabled, slider_x, slider_y, mouse, clicked);
    slider_y += draw_slider("Thickness", &mut ch.outline_thickness, 0.0, 2.0, 0.05, slider_x, slider_y, slider_w, mouse, clicked, held);
    slider_y += 10.0;

    // Color RGBA sliders
    let mut r = ch.color.r * 255.0;
    let mut g = ch.color.g * 255.0;
    let mut b = ch.color.b * 255.0;
    let mut a = ch.color.a * 255.0;
    slider_y += draw_slider("Red", &mut r, 0.0, 255.0, 1.0, slider_x, slider_y, slider_w, mouse, clicked, held);
    slider_y += draw_slider("Green", &mut g, 0.0, 255.0, 1.0, slider_x, slider_y, slider_w, mouse, clicked, held);
    slider_y += draw_slider("Blue", &mut b, 0.0, 255.0, 1.0, slider_x, slider_y, slider_w, mouse, clicked, held);
    slider_y += draw_slider("Alpha", &mut a, 0.0, 255.0, 1.0, slider_x, slider_y, slider_w, mouse, clicked, held);
    ch.color = Color::from_rgba(r as u8, g as u8, b as u8, a as u8);
    slider_y += 10.0;

    // crosshair Type dropdown
    let type_names: Vec<&str> = CROSSHAIR_TYPES.iter().map(|t| crosshair_type_name(*t)).collect();
    let mut selected_idx = CROSSHAIR_TYPES.iter().position(|t| *t == ch.crosshair_type).unwrap_or(0);
    let prev_idx = selected_idx;

    draw_dropdown("Crosshair Type", &type_names, &mut selected_idx, slider_x, slider_y, slider_w, mouse, clicked);

    if selected_idx < CROSSHAIR_TYPES.len() {
        ch.crosshair_type = CROSSHAIR_TYPES[selected_idx];
    }

    // Shotgun warning when crosshair type changes (only once per visit)
    if state.preview_weapon == WeaponKind::Shotgun && selected_idx != prev_idx && !state.shotgun_warning_shown_this_visit {
        state.shotgun_type_warning = true;
    }

    // Preview area buttons
    let preview_x = x + w * 0.5 + 20.0;
    let btn_row_y = y + 530.0;
    let small_btn_w = 100.0;
    let small_btn_h = 35.0;
    let small_gap = 10.0;

    // ADS toggle
    if clicked && mouse.0 >= preview_x && mouse.0 <= preview_x + small_btn_w &&
       mouse.1 >= btn_row_y && mouse.1 <= btn_row_y + small_btn_h
    {
        state.preview_ads = !state.preview_ads;
    }

    // Light/Dark background toggle
    let bg_btn_x = preview_x + small_btn_w + small_gap;
    if clicked && mouse.0 >= bg_btn_x && mouse.0 <= bg_btn_x + small_btn_w &&
       mouse.1 >= btn_row_y && mouse.1 <= btn_row_y + small_btn_h
    {
        state.preview_light_bg = !state.preview_light_bg;
    }

    // Reset to defaults
    let weapon = &mut weapons.weapons[state.preview_weapon.slot() - 1];
    let reset_btn_x = bg_btn_x + small_btn_w + small_gap;
    if clicked && mouse.0 >= reset_btn_x && mouse.0 <= reset_btn_x + small_btn_w &&
       mouse.1 >= btn_row_y && mouse.1 <= btn_row_y + small_btn_h
    {
        weapon.crosshair = CrosshairConfig::for_weapon(weapon.kind);
    }

    // Shotgun warning dismiss
    if state.shotgun_type_warning {
        if draw_shotgun_type_warning(preview_x, btn_row_y, 0.0, 0.0, mouse, clicked) {
            state.shotgun_type_warning = false;
            state.shotgun_warning_shown_this_visit = true;
        }
    }
}

pub fn draw_crosshair_settings(
    state: &CrosshairSettingsPreview,
    weapons: &WeaponInventory,
    x: f32,
    y: f32,
    w: f32,
    _h: f32,
    _assets: &HashMap<String, Texture2D>,
    _mouse: (f32, f32)
) {
    // Section header
    draw_section_header("Crosshair Settings", x + 20.0, y + 20.0);

    // Weapon selector buttons
    let btn_y = y + 60.0;
    let btn_w = 100.0;
    let btn_h = 35.0;
    let btn_gap = 10.0;

    for (i, kind) in WeaponKind::ALL.iter().enumerate() {
        let btn_x = x + (btn_w + btn_gap) * (i as f32);

        // button background
        let btn_color = if kind == &state.preview_weapon {
            Color::from_rgba(80, 150, 220, 255)
        } else {
            Color::from_rgba(40, 40, 40, 255)
        };

        draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_color);
        draw_rectangle_lines(btn_x, btn_y, btn_w, btn_h, 1.0, GRAY);

        // button text
        let name = kind.stats().name;
        let text_w = measure_game_text(name, 14);
        draw_text_ex(
            name,
            btn_x + (btn_w - text_w) / 2.0,
            btn_y + 22.0,
            text_params(14, WHITE)
        );
    }



    // preview box
    let preview_x = x + w * 0.5 + 20.0;
    let preview_y = y + 120.0;
    let preview_w = 400.0;
    let preview_h = 400.0;

    // get weapon
    let weapon = &weapons.weapons[state.preview_weapon.slot() - 1];
    draw_crosshair_preview(weapon, state, preview_x, preview_y, preview_w, preview_h, _assets);

    // Button row below preview
    let btn_row_y = y + 530.0;
    let small_btn_w = 100.0;
    let small_btn_h = 35.0;
    let small_gap = 10.0;

    // ADS toggle
    let ads_color = if state.preview_ads { Color::from_rgba(80, 150, 220, 255) } else { Color::from_rgba(40, 40, 40, 255) };
    draw_rectangle(preview_x, btn_row_y, small_btn_w, small_btn_h, ads_color);
    draw_rectangle_lines(preview_x, btn_row_y, small_btn_w, small_btn_h, 1.0, GRAY);
    let ads_text = if state.preview_ads { "ADS: ON" } else { "ADS: OFF" };
    let ads_tw = measure_game_text(ads_text, 14);
    draw_text_ex(ads_text, preview_x + (small_btn_w - ads_tw) / 2.0, btn_row_y + 22.0, text_params(14, WHITE));

    // Light/Dark background toggle
    let bg_btn_x = preview_x + small_btn_w + small_gap;
    let bg_color = if state.preview_light_bg { Color::from_rgba(180, 180, 180, 255) } else { Color::from_rgba(40, 40, 40, 255) };
    draw_rectangle(bg_btn_x, btn_row_y, small_btn_w, small_btn_h, bg_color);
    draw_rectangle_lines(bg_btn_x, btn_row_y, small_btn_w, small_btn_h, 1.0, GRAY);
    let bg_text = if state.preview_light_bg { "Light mode" } else { "Dark mode" };
    let bg_tw = measure_game_text(bg_text, 14);
    let bg_text_color = if state.preview_light_bg { BLACK } else { WHITE };
    draw_text_ex(bg_text, bg_btn_x + (small_btn_w - bg_tw) / 2.0, btn_row_y + 22.0, text_params(14, bg_text_color));

    // Reset button
    let reset_btn_x = bg_btn_x + small_btn_w + small_gap;
    draw_rectangle(reset_btn_x, btn_row_y, small_btn_w, small_btn_h, Color::from_rgba(60, 40, 40, 255));
    draw_rectangle_lines(reset_btn_x, btn_row_y, small_btn_w, small_btn_h, 1.0, GRAY);
    let reset_tw = measure_game_text("Reset", 14);
    draw_text_ex("Reset", reset_btn_x + (small_btn_w - reset_tw) / 2.0, btn_row_y + 22.0, text_params(14, WHITE));

    // Shotgun crosshair type warning overlay
    if state.preview_weapon == WeaponKind::Shotgun && state.shotgun_type_warning {
        draw_shotgun_type_warning(preview_x, btn_row_y, 0.0, 0.0, _mouse, false);
    }
}

// Helper function to draw preview box

fn draw_crosshair_preview(
    weapon: &WeaponState,
    state: &CrosshairSettingsPreview,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    assets: &HashMap<String, Texture2D>
) {
    // Background: try to load custom texture, fallback to solid color
    const PREVIEW_BG_TEXTURE_KEY: &str = "crosshair_preview_bg";

    if let Some(bg_texture) = assets.get(PREVIEW_BG_TEXTURE_KEY) {
        // Draw texture background if available
        draw_texture_ex(
            bg_texture,
            x,
            y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(w, h)),
                ..Default::default()
            }
        );
    } else {
        // Fallback to solid color (toggleable light/dark)
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

    // Sniper: TODO
    if weapon.kind == WeaponKind::Sniper {
        draw_text_ex("TODO", cx - 20.0, cy, text_params(18, YELLOW));
        return;
    }

    let xhair = &weapon.crosshair;
    let sh = screen_height(); // Scale relative to actual screen, same as in-game HUD
    let ads_progress = if state.preview_ads { 1.0 } else { 0.0 };

    let color = xhair.color;

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
                    draw_arc(cx, cy, radius, base_angle + gap_angle, base_angle + quarter - gap_angle, ot, segs, oc);
                }
            }

            for q in 0..4 {
                let base_angle = q as f32 * quarter;
                draw_arc(cx, cy, radius, base_angle + gap_angle, base_angle + quarter - gap_angle, thick, segs, color);
            }

            if xhair.dot_enabled {
                let dot_alpha = ((1.0 - ads_progress) * 255.0) as u8;
                let dot_color = Color::from_rgba(
                    (color.r * 255.0) as u8, (color.g * 255.0) as u8, (color.b * 255.0) as u8, dot_alpha
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

            let corners: [(f32, f32); 4] = [
                (-1.0, -1.0), ( 1.0, -1.0),
                (-1.0,  1.0), ( 1.0,  1.0),
            ];

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
                    (u*u*p0x + 2.0*u*t*p1x + t*t*p2x,
                     u*u*p0y + 2.0*u*t*p1y + t*t*p2y)
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
                    (color.r * 255.0) as u8, (color.g * 255.0) as u8, (color.b * 255.0) as u8, dot_alpha
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
                    (color.r * 255.0) as u8, (color.g * 255.0) as u8, (color.b * 255.0) as u8, dot_alpha
                );
                draw_circle(cx, cy, (xhair.dot_size * sh) / 100.0, dot_color);
            }
        }
    }

    // Weapon name
    let name_color = if state.preview_light_bg { BLACK } else { WHITE };
    draw_text_ex(weapon.kind.stats().name, x + 10.0, y + 25.0, text_params(18, name_color));
}

fn draw_shotgun_type_warning(
    x: f32,
    y: f32,
    _btn_w: f32,
    _btn_h: f32,
    mouse: (f32, f32),
    clicked: bool,
) -> bool {
    // Warning box
    let box_w = 340.0;
    let box_h = 80.0;
    let box_x = x;
    let box_y = y - box_h - 10.0;

    draw_rectangle(box_x, box_y, box_w, box_h, Color::from_rgba(60, 30, 10, 240));
    draw_rectangle_lines(box_x, box_y, box_w, box_h, 2.0, Color::from_rgba(255, 160, 0, 255));

    draw_text_ex("Switching to a non-standard crosshair", box_x + 10.0, box_y + 22.0, text_params(14, Color::from_rgba(255, 200, 80, 255)));
    draw_text_ex("for shotgun may affect gameplay.", box_x + 10.0, box_y + 42.0, text_params(14, Color::from_rgba(255, 200, 80, 255)));

    // OK button
    let ok_w = 50.0;
    let ok_h = 25.0;
    let ok_x = box_x + box_w - ok_w - 10.0;
    let ok_y = box_y + box_h - ok_h - 8.0;

    draw_rectangle(ok_x, ok_y, ok_w, ok_h, Color::from_rgba(80, 150, 220, 255));
    draw_rectangle_lines(ok_x, ok_y, ok_w, ok_h, 1.0, GRAY);
    let ok_tw = measure_game_text("OK", 14);
    draw_text_ex("OK", ok_x + (ok_w - ok_tw) / 2.0, ok_y + 17.0, text_params(14, WHITE));

    // Return true if OK was clicked (dismiss)
    if clicked && mouse.0 >= ok_x && mouse.0 <= ok_x + ok_w &&
       mouse.1 >= ok_y && mouse.1 <= ok_y + ok_h
    {
        return true;
    }

    false
}

//  HELPER FUNCTIONS

fn draw_section_header(label: &str, x: f32, y: f32) -> f32 {
    use crate::ui::fonts::text_params;
    draw_text_ex(label, x, y + 18.0, text_params(18, Color::from_rgba(100, 180, 255, 255)));
    draw_line(x, y + 25.0, x + 300.0, y + 25.0, 1.0, GRAY);
    35.0 // returns height used for vertical stacking
}

fn draw_slider(
    label: &str,
    value: &mut f32,
    min: f32,
    max: f32,
    step: f32,
    x: f32,
    y: f32,
    width: f32,
    mouse: (f32, f32),
    _clicked: bool,
    held: bool
) -> f32 {
    use crate::ui::fonts::text_params;

    let label_w = 150.0;
    let slider_x = x + label_w;
    let slider_w = width - label_w - 60.0;
    let slider_h = 16.0;
    let row_h = 28.0;

    // label
    draw_text_ex(label, x, y + 16.0, text_params(16, WHITE));

    // slider track
    draw_rectangle(slider_x, y, slider_w, slider_h, Color::from_rgba(40, 40, 40, 255));
    let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
    draw_rectangle(slider_x, y, slider_w * t, slider_h, Color::from_rgba(80, 150, 220, 255));
    draw_rectangle_lines(slider_x, y, slider_w, slider_h, 1.0, GRAY);

    // drag to adjust value
    if
        held &&
        mouse.1 >= y &&
        mouse.1 <= y + row_h &&
        mouse.0 >= slider_x &&
        mouse.0 <= slider_x + slider_w
    {
        let raw = min + ((mouse.0 - slider_x) / slider_w) * (max - min);
        *value = (raw / step).round() * step;
        *value = value.clamp(min, max);
    }

    // value text on right
    let precision = if step >= 1.0 { 0 } else if step >= 0.1 { 1 } else { 2 };

    let txt = format!("{:.*}", precision, *value);
    draw_text_ex(&txt, slider_x + slider_w + 8.0, y + 14.0, text_params(14, WHITE));

    row_h
}

fn draw_toggle(
    label: &str,
    value: &mut bool,
    x: f32,
    y: f32,
    mouse: (f32, f32),
    clicked: bool
) -> f32 {
    use crate::ui::fonts::text_params;

    let box_size = 20.0;
    let row_h = 28.0;

    // checkbox
    let checkbox_color = if *value {
        Color::from_rgba(80, 150, 220, 255)
    } else {
        Color::from_rgba(40, 40, 40, 255)
    };

    draw_rectangle(x, y, box_size, box_size, checkbox_color);
    draw_rectangle_lines(x, y, box_size, box_size, 1.0, GRAY);

    // click detection
    if
        clicked &&
        mouse.0 >= x &&
        mouse.0 <= x + box_size &&
        mouse.1 >= y &&
        mouse.1 <= y + box_size
    {
        *value = !*value;
    }

    // label
    draw_text_ex(label, x + box_size + 10.0, y + 16.0, text_params(16, WHITE));

    row_h
}

fn draw_dropdown(
    label: &str,
    options: &[&str],
    selected: &mut usize,
    x: f32,
    y: f32,
    width: f32,
    mouse: (f32, f32),
    clicked: bool,
) -> f32 {
    
    let label_w = 150.0;
    let dropdown_x = x + label_w;
    let dropdown_w = width - label_w - 10.0;
    let dropdown_h = 30.0;
    
    // label
    draw_text_ex(label, x, y + 20.0, text_params(16, WHITE));
    
    // dropdown box
    draw_rectangle(dropdown_x, y, dropdown_w, dropdown_h, Color::from_rgba(40, 40, 40, 255));
    draw_rectangle_lines(dropdown_x, y, dropdown_w, dropdown_h, 1.0, GRAY);
    
    // current selection text
    let current_text = options[*selected];
    let text_w = measure_game_text(current_text, 16);
    draw_text_ex(
        current_text,
        dropdown_x + (dropdown_w - text_w) / 2.0,
        y + 20.0,
        text_params(16, WHITE),
    );
    
    // click to cycle through options
    if clicked && mouse.0 >= dropdown_x && mouse.0 <= dropdown_x + dropdown_w &&
       mouse.1 >= y && mouse.1 <= y + dropdown_h
    {
        *selected = (*selected + 1) % options.len();
    }
    
    35.0 // row height
}


fn draw_arc(
    cx: f32,
    cy: f32,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    thickness: f32,
    segments: u32,
    color: Color
) {
    let angle_range = end_angle - start_angle;
    let angle_step = angle_range / (segments as f32);

    for i in 0..segments {
        let a1 = start_angle + (i as f32) * angle_step;
        let a2 = start_angle + ((i + 1) as f32) * angle_step;

        let x1_inner = cx + (radius - thickness / 2.0) * a1.cos();
        let y1_inner = cy - (radius - thickness / 2.0) * a1.sin();
        let x2_inner = cx + (radius - thickness / 2.0) * a2.cos();
        let y2_inner = cy - (radius - thickness / 2.0) * a2.sin();

        let x1_outer = cx + (radius + thickness / 2.0) * a1.cos();
        let y1_outer = cy - (radius + thickness / 2.0) * a1.sin();
        let x2_outer = cx + (radius + thickness / 2.0) * a2.cos();
        let y2_outer = cy - (radius + thickness / 2.0) * a2.sin();

        // draw quad segment as two triangles
        draw_triangle(
            vec2(x1_inner, y1_inner),
            vec2(x2_inner, y2_inner),
            vec2(x1_outer, y1_outer),
            color
        );
        draw_triangle(
            vec2(x2_inner, y2_inner),
            vec2(x2_outer, y2_outer),
            vec2(x1_outer, y1_outer),
            color
        );
    }
}
