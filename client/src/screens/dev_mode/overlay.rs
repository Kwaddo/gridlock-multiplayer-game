use crate::core::GameConfig;
use crate::physics::movement::MovementConfig;
use crate::ui::fonts::{measure_game_text, text_params};
use crate::weapons::state::WeaponInventory;
use macroquad::prelude::*;

use super::state::{DevModeState, TAB_NAMES};
use super::tabs::{
    draw_crosshair_tab, draw_debug_tab, draw_movement_tab, draw_viewmodel_tab, draw_weapon_tab,
};

pub fn draw_dev_overlay<F>(
    dev: &mut DevModeState,
    inventory: &mut WeaponInventory,
    move_config: &mut MovementConfig,
    _game_config: &mut GameConfig,
    mut draw_bot_tab: F,
) where
    F: FnMut(f32, f32, f32, (f32, f32), bool, bool),
{
    let sh = screen_height();
    let mouse = mouse_position();
    let clicked = is_mouse_button_pressed(MouseButton::Left);
    let held = is_mouse_button_down(MouseButton::Left);

    let panel_w = 460.0;
    let panel_x = 10.0;
    let panel_y = 10.0;
    let panel_h = sh - 20.0;

    draw_rectangle(
        panel_x,
        panel_y,
        panel_w,
        panel_h,
        Color::from_rgba(20, 20, 20, 230),
    );
    draw_rectangle_lines(
        panel_x,
        panel_y,
        panel_w,
        panel_h,
        2.0,
        Color::from_rgba(80, 150, 220, 255),
    );

    draw_text_ex(
        "DEV MODE",
        panel_x + 10.0,
        panel_y + 24.0,
        text_params(20, Color::from_rgba(80, 150, 220, 255)),
    );
    draw_text_ex(
        "F3 to close",
        panel_x + panel_w - 100.0,
        panel_y + 24.0,
        text_params(12, GRAY),
    );

    let tab_y = panel_y + 36.0;
    let tab_w = (panel_w - 20.0) / TAB_NAMES.len() as f32;
    for (i, name) in TAB_NAMES.iter().enumerate() {
        let tx = panel_x + 10.0 + i as f32 * tab_w;
        let active = i == dev.active_tab;
        let color = if active {
            Color::from_rgba(80, 150, 220, 255)
        } else if mouse.0 >= tx
            && mouse.0 <= tx + tab_w
            && mouse.1 >= tab_y
            && mouse.1 <= tab_y + 26.0
        {
            Color::from_rgba(60, 60, 60, 255)
        } else {
            Color::from_rgba(40, 40, 40, 255)
        };
        draw_rectangle(tx, tab_y, tab_w - 2.0, 26.0, color);
        let tw = measure_game_text(name, 14);
        draw_text_ex(
            name,
            tx + (tab_w - 2.0 - tw) / 2.0,
            tab_y + 18.0,
            text_params(14, WHITE),
        );

        if clicked
            && mouse.0 >= tx
            && mouse.0 <= tx + tab_w
            && mouse.1 >= tab_y
            && mouse.1 <= tab_y + 26.0
        {
            dev.active_tab = i;
            dev.scroll_offset = 0.0;
        }
    }

    let (_, scroll_y) = mouse_wheel();
    if mouse.0 >= panel_x
        && mouse.0 <= panel_x + panel_w
        && mouse.1 >= panel_y
        && mouse.1 <= panel_y + panel_h
    {
        dev.scroll_offset += scroll_y * 30.0;
        dev.scroll_offset = dev.scroll_offset.min(0.0);
        // clamp scroll so you can't scroll past the bottom of content
        let content_start_y = tab_y + 36.0; // where content begins (below tabs)
        let available_h = panel_y + panel_h - content_start_y;
        let max_scroll_down = -(dev.content_height - available_h).max(0.0);
        dev.scroll_offset = dev.scroll_offset.max(max_scroll_down);
    }

    let content_x = panel_x + 16.0;
    let content_y = tab_y + 36.0 + dev.scroll_offset;
    let content_w = panel_w - 32.0;

    match dev.active_tab {
        0 => draw_weapon_tab(inventory, content_x, content_y, content_w, mouse, clicked, held),
        1 => draw_crosshair_tab(inventory, content_x, content_y, content_w, mouse, clicked, held),
        2 => draw_movement_tab(
            move_config,
            content_x,
            content_y,
            content_w,
            mouse,
            clicked,
            held,
        ),
        3 => {
            let bottom_y = draw_viewmodel_tab(inventory, dev, content_x, content_y, content_w, mouse, clicked, held);
            dev.content_height = bottom_y - content_y;
        }
        4 => draw_debug_tab(dev, inventory, content_x, content_y, mouse, clicked),
        5 => {
            draw_bot_tab(content_x, content_y, content_w, mouse, clicked, held);
        }
        _ => {}
    }

    // ── Viewport Drag Mode (Viewmodel tab only) ───────────────────────────
    if dev.active_tab == 3 && dev.viewport_drag_active {
        let right_held = is_mouse_button_down(MouseButton::Right);
        let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        let mouse_over_panel = mouse.0 >= panel_x && mouse.0 <= panel_x + panel_w;

        let pos_sens = if dev.precision_mode { 0.0002 } else { 0.002 };
        let rot_sens = if dev.precision_mode { 0.0008 } else { 0.008 };

        // ── Mouse drag (axis-locked: dominant axis wins) ──
        if (held || right_held) && !mouse_over_panel {
            let raw_dx = mouse.0 - dev.viewport_drag_prev_mouse.0;
            let raw_dy = mouse.1 - dev.viewport_drag_prev_mouse.1;

            // Lock to dominant axis so you don't accidentally drift sideways
            let (dx, dy) = if raw_dx.abs() > raw_dy.abs() {
                (raw_dx, 0.0)
            } else {
                (0.0, raw_dy)
            };

            let weapon = inventory.active_mut();

            if right_held {
                weapon.stats_data.viewmodel_rotation.1 =
                    (weapon.stats_data.viewmodel_rotation.1 + dx * rot_sens).clamp(-3.15, 3.15);
                weapon.stats_data.viewmodel_rotation.0 =
                    (weapon.stats_data.viewmodel_rotation.0 - dy * rot_sens).clamp(-3.15, 3.15);
            } else if ctrl {
                weapon.stats_data.viewmodel_rotation.2 =
                    (weapon.stats_data.viewmodel_rotation.2 + dx * rot_sens).clamp(-3.15, 3.15);
            } else if shift {
                weapon.stats_data.viewmodel_position.2 =
                    (weapon.stats_data.viewmodel_position.2 - dy * pos_sens).clamp(-0.5, 0.5);
            } else {
                weapon.stats_data.viewmodel_position.0 =
                    (weapon.stats_data.viewmodel_position.0 + dx * pos_sens).clamp(-0.5, 0.5);
                weapon.stats_data.viewmodel_position.1 =
                    (weapon.stats_data.viewmodel_position.1 - dy * pos_sens).clamp(-0.5, 0.5);
            }
        }

        dev.viewport_drag_prev_mouse = mouse;

        // ── Arrow keys: precise single-axis nudging ──
        let arrow_step_pos = if dev.precision_mode { 0.001 } else { 0.005 };
        let arrow_step_rot = if dev.precision_mode { 0.005 } else { 0.02 };
        {
            let weapon = inventory.active_mut();

            if shift {
                // Shift + arrows → yaw / pitch
                if is_key_down(KeyCode::Left) {
                    weapon.stats_data.viewmodel_rotation.1 =
                        (weapon.stats_data.viewmodel_rotation.1 - arrow_step_rot).clamp(-3.15, 3.15);
                }
                if is_key_down(KeyCode::Right) {
                    weapon.stats_data.viewmodel_rotation.1 =
                        (weapon.stats_data.viewmodel_rotation.1 + arrow_step_rot).clamp(-3.15, 3.15);
                }
                if is_key_down(KeyCode::Up) {
                    weapon.stats_data.viewmodel_rotation.0 =
                        (weapon.stats_data.viewmodel_rotation.0 + arrow_step_rot).clamp(-3.15, 3.15);
                }
                if is_key_down(KeyCode::Down) {
                    weapon.stats_data.viewmodel_rotation.0 =
                        (weapon.stats_data.viewmodel_rotation.0 - arrow_step_rot).clamp(-3.15, 3.15);
                }
            } else if ctrl {
                // Ctrl + left/right → roll
                if is_key_down(KeyCode::Left) {
                    weapon.stats_data.viewmodel_rotation.2 =
                        (weapon.stats_data.viewmodel_rotation.2 - arrow_step_rot).clamp(-3.15, 3.15);
                }
                if is_key_down(KeyCode::Right) {
                    weapon.stats_data.viewmodel_rotation.2 =
                        (weapon.stats_data.viewmodel_rotation.2 + arrow_step_rot).clamp(-3.15, 3.15);
                }
                // Ctrl + up/down → pos Z (depth)
                if is_key_down(KeyCode::Up) {
                    weapon.stats_data.viewmodel_position.2 =
                        (weapon.stats_data.viewmodel_position.2 + arrow_step_pos).clamp(-0.5, 0.5);
                }
                if is_key_down(KeyCode::Down) {
                    weapon.stats_data.viewmodel_position.2 =
                        (weapon.stats_data.viewmodel_position.2 - arrow_step_pos).clamp(-0.5, 0.5);
                }
            } else {
                // Arrows → pos X / Y
                if is_key_down(KeyCode::Left) {
                    weapon.stats_data.viewmodel_position.0 =
                        (weapon.stats_data.viewmodel_position.0 - arrow_step_pos).clamp(-0.5, 0.5);
                }
                if is_key_down(KeyCode::Right) {
                    weapon.stats_data.viewmodel_position.0 =
                        (weapon.stats_data.viewmodel_position.0 + arrow_step_pos).clamp(-0.5, 0.5);
                }
                if is_key_down(KeyCode::Up) {
                    weapon.stats_data.viewmodel_position.1 =
                        (weapon.stats_data.viewmodel_position.1 + arrow_step_pos).clamp(-0.5, 0.5);
                }
                if is_key_down(KeyCode::Down) {
                    weapon.stats_data.viewmodel_position.1 =
                        (weapon.stats_data.viewmodel_position.1 - arrow_step_pos).clamp(-0.5, 0.5);
                }
            }
        }

        // ── Hint overlay (lower-right corner) ──
        let sw = screen_width();
        let sh = screen_height();
        let hints = [
            ("LMB drag", "Move X / Y (axis-locked)"),
            ("Shift+LMB", "Move Z (depth)"),
            ("RMB drag", "Rotate Yaw / Pitch"),
            ("Ctrl+LMB", "Roll"),
            ("", ""),
            ("Arrows", "Move X / Y"),
            ("Shift+Arrows", "Rotate Yaw / Pitch"),
            ("Ctrl+Arrows", "Roll / Depth"),
        ];
        let hint_w = 290.0_f32;
        let hint_h = 16.0 * hints.len() as f32 + 28.0;
        let hx = sw - hint_w - 10.0;
        let hy = sh - hint_h - 10.0;

        draw_rectangle(hx, hy, hint_w, hint_h, Color::from_rgba(10, 10, 10, 200));
        draw_rectangle_lines(hx, hy, hint_w, hint_h, 1.0, Color::from_rgba(80, 200, 80, 255));
        draw_text_ex(
            "DRAG MODE",
            hx + 8.0,
            hy + 16.0,
            text_params(13, Color::from_rgba(80, 200, 80, 255)),
        );
        for (i, (key, desc)) in hints.iter().enumerate() {
            if key.is_empty() { continue; }
            let row_y = hy + 28.0 + i as f32 * 16.0;
            draw_text_ex(key, hx + 8.0, row_y + 12.0, text_params(11, Color::from_rgba(200, 200, 200, 255)));
            draw_text_ex(desc, hx + 110.0, row_y + 12.0, text_params(11, Color::from_rgba(160, 160, 160, 255)));
        }
    }
}
