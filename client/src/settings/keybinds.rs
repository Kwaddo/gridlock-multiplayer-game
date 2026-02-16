use crate::core::{GameConfig, InputBinding};
use crate::ui::fonts::{measure_game_text, text_params};
use macroquad::prelude::*;

// all rebindable actions in display order
const BIND_ACTIONS: &[(&str, &str)] = &[
    ("Forward", "forward"),
    ("Backward", "backward"),
    ("Left", "left"),
    ("Right", "right"),
    ("Sprint", "sprint"),
    ("Crouch", "crouch"),
    ("Jump", "jump"),
    ("Reload", "reload"),
    ("Fire", "fire"),
    ("Aim", "aim"),
    ("Settings", "settings"),
    ("Weapon 1", "weapon1"),
    ("Weapon 2", "weapon2"),
    ("Weapon 3", "weapon3"),
    ("Weapon 4", "weapon4"),
    ("Weapon 5", "weapon5"),
    ("Fullscreen", "fullscreen"),
];

fn get_binding(config: &GameConfig, action: &str) -> InputBinding {
    match action {
        "forward" => config.bind_forward,
        "backward" => config.bind_backward,
        "left" => config.bind_left,
        "right" => config.bind_right,
        "sprint" => config.bind_sprint,
        "crouch" => config.bind_crouch,
        "jump" => config.bind_jump,
        "reload" => config.bind_reload,
        "fire" => config.bind_fire,
        "aim" => config.bind_aim,
        "settings" => config.bind_settings,
        "weapon1" => config.bind_weapon1,
        "weapon2" => config.bind_weapon2,
        "weapon3" => config.bind_weapon3,
        "weapon4" => config.bind_weapon4,
        "weapon5" => config.bind_weapon5,
        "fullscreen" => config.bind_fullscreen,
        _ => InputBinding::Key(KeyCode::Unknown),
    }
}

fn set_binding(config: &mut GameConfig, action: &str, binding: InputBinding) {
    match action {
        "forward" => config.bind_forward = binding,
        "backward" => config.bind_backward = binding,
        "left" => config.bind_left = binding,
        "right" => config.bind_right = binding,
        "sprint" => config.bind_sprint = binding,
        "crouch" => config.bind_crouch = binding,
        "jump" => config.bind_jump = binding,
        "reload" => config.bind_reload = binding,
        "fire" => config.bind_fire = binding,
        "aim" => config.bind_aim = binding,
        "settings" => config.bind_settings = binding,
        "weapon1" => config.bind_weapon1 = binding,
        "weapon2" => config.bind_weapon2 = binding,
        "weapon3" => config.bind_weapon3 = binding,
        "weapon4" => config.bind_weapon4 = binding,
        "weapon5" => config.bind_weapon5 = binding,
        "fullscreen" => config.bind_fullscreen = binding,
        _ => {}
    }
}

/// Calculate row dimensions that fit within the panel, preventing overflow
fn calc_row_dims(panel_h: f32, s: f32) -> (f32, f32) {
    let total_rows = BIND_ACTIONS.len() as f32 + 1.5; // +1 for toggle, +0.5 for gap
    let title_overhead = 50.0 * s;
    let available = panel_h - title_overhead;
    let base_row_h = 30.0 * s;
    let base_row_gap = 3.0 * s;
    let base_pitch = base_row_h + base_row_gap;

    // If content fits, use scaled sizes; otherwise shrink to fit
    if base_pitch * total_rows <= available {
        (base_row_h, base_row_gap)
    } else {
        let pitch = available / total_rows;
        let gap = (3.0 * s).min(pitch * 0.1);
        (pitch - gap, gap)
    }
}

pub fn update_keybinds(
    config: &mut GameConfig,
    rebinding: &mut Option<String>,
    right_x: f32,
    panel_top: f32,
    panel_w: f32,
    mouse: (f32, f32),
    clicked: bool,
    s: f32,
) {
    let panel_h = screen_height() - panel_top - 70.0 * s;
    let (row_h, row_gap) = calc_row_dims(panel_h, s);
    let bx = right_x + 20.0 * s;
    let bw = panel_w - 40.0 * s;

    // if currently rebinding, listen for key or mouse input
    if let Some(action) = rebinding.clone() {
        if is_mouse_button_pressed(MouseButton::Left) {
            set_binding(config, &action, InputBinding::Mouse(MouseButton::Left));
            *rebinding = None;
            return;
        }
        if is_mouse_button_pressed(MouseButton::Right) {
            set_binding(config, &action, InputBinding::Mouse(MouseButton::Right));
            *rebinding = None;
            return;
        }
        if is_mouse_button_pressed(MouseButton::Middle) {
            set_binding(config, &action, InputBinding::Mouse(MouseButton::Middle));
            *rebinding = None;
            return;
        }
        if let Some(key) = get_last_key_pressed() {
            if key == KeyCode::Escape {
                *rebinding = None;
            } else {
                set_binding(config, &action, InputBinding::Key(key));
                *rebinding = None;
            }
            return;
        }
        return;
    }

    if !clicked {
        return;
    }

    // check clicks on bind rows
    for (i, &(_, action_id)) in BIND_ACTIONS.iter().enumerate() {
        let by = panel_top + 50.0 * s + (i as f32) * (row_h + row_gap);
        if mouse.0 >= bx && mouse.0 <= bx + bw && mouse.1 >= by && mouse.1 <= by + row_h {
            *rebinding = Some(action_id.to_string());
            return;
        }
    }

    // crouch toggle button
    let toggle_y =
        panel_top + 50.0 * s + (BIND_ACTIONS.len() as f32) * (row_h + row_gap) + 10.0 * s;
    if mouse.0 >= bx && mouse.0 <= bx + bw && mouse.1 >= toggle_y && mouse.1 <= toggle_y + row_h {
        config.crouch_toggle = !config.crouch_toggle;
    }
}

pub fn draw_keybinds(
    config: &GameConfig,
    rebinding: &Option<String>,
    right_x: f32,
    panel_w: f32,
    panel_top: f32,
    panel_h: f32,
    s: f32,
) {
    draw_rectangle(
        right_x,
        panel_top,
        panel_w,
        panel_h,
        Color::from_rgba(30, 30, 30, 240),
    );
    draw_rectangle_lines(right_x, panel_top, panel_w, panel_h, 1.0, GRAY);

    let title_fs = (22.0 * s) as u16;
    let kb_title = "Key Bindings";
    let kb_w = measure_game_text(kb_title, title_fs);
    draw_text_ex(
        kb_title,
        right_x + (panel_w - kb_w) / 2.0,
        panel_top + 30.0 * s,
        text_params(title_fs, YELLOW),
    );

    let (row_h, row_gap) = calc_row_dims(panel_h, s);
    let bx = right_x + 20.0 * s;
    let bw = panel_w - 40.0 * s;
    let row_fs = (18.0 * s).min(row_h * 0.6) as u16;

    let all_bindings = config.all_bindings();

    for (i, &(label, action_id)) in BIND_ACTIONS.iter().enumerate() {
        let by = panel_top + 50.0 * s + (i as f32) * (row_h + row_gap);
        let binding = get_binding(config, action_id);

        let duplicate = all_bindings.iter().filter(|(_, b)| *b == binding).count() > 1;

        let is_rebinding = rebinding.as_deref() == Some(action_id);
        let bg_color = if is_rebinding {
            Color::from_rgba(80, 80, 20, 255)
        } else {
            Color::from_rgba(45, 45, 45, 255)
        };

        draw_rectangle(bx, by, bw, row_h, bg_color);
        draw_rectangle_lines(bx, by, bw, row_h, 1.0, GRAY);
        draw_text_ex(
            label,
            bx + 10.0 * s,
            by + row_h * 0.67,
            text_params(row_fs, WHITE),
        );

        let key_text = if is_rebinding {
            "Press key/mouse...".to_string()
        } else {
            binding.display_name()
        };
        let key_tw = measure_game_text(&key_text, row_fs);
        draw_text_ex(
            &key_text,
            bx + bw - key_tw - 10.0 * s,
            by + row_h * 0.67,
            text_params(row_fs, if duplicate { RED } else { YELLOW }),
        );
    }

    // Crouch mode toggle button
    let toggle_y =
        panel_top + 50.0 * s + (BIND_ACTIONS.len() as f32) * (row_h + row_gap) + 10.0 * s;

    let mouse = mouse_position();
    let hover =
        mouse.0 >= bx && mouse.0 <= bx + bw && mouse.1 >= toggle_y && mouse.1 <= toggle_y + row_h;

    let bg = if hover {
        Color::from_rgba(65, 65, 85, 255)
    } else {
        Color::from_rgba(55, 55, 75, 255)
    };
    draw_rectangle(bx, toggle_y, bw, row_h, bg);
    draw_rectangle_lines(bx, toggle_y, bw, row_h, 1.0, GRAY);
    draw_text_ex(
        "Crouch Mode",
        bx + 10.0 * s,
        toggle_y + row_h * 0.67,
        text_params(row_fs, WHITE),
    );

    let mode_text = if config.crouch_toggle {
        "Toggle"
    } else {
        "Hold"
    };
    let mode_tw = measure_game_text(mode_text, row_fs);
    let mode_btn_w = 80.0 * s;
    let mode_btn_x = bx + bw - mode_btn_w - 5.0 * s;
    let mode_btn_y = toggle_y + 3.0 * s;
    let mode_btn_h = row_h - 6.0 * s;
    let mode_btn_color = if config.crouch_toggle {
        Color::from_rgba(30, 100, 30, 255)
    } else {
        Color::from_rgba(100, 60, 20, 255)
    };
    draw_rectangle(
        mode_btn_x,
        mode_btn_y,
        mode_btn_w,
        mode_btn_h,
        mode_btn_color,
    );
    draw_rectangle_lines(mode_btn_x, mode_btn_y, mode_btn_w, mode_btn_h, 1.0, WHITE);
    draw_text_ex(
        mode_text,
        mode_btn_x + (mode_btn_w - mode_tw) / 2.0,
        toggle_y + row_h * 0.63,
        text_params(row_fs, WHITE),
    );
}
