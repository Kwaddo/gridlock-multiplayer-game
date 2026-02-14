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

pub fn update_keybinds(
    config: &mut GameConfig,
    rebinding: &mut Option<String>,
    right_x: f32,
    panel_top: f32,
    panel_w: f32,
    mouse: (f32, f32),
    clicked: bool,
) {
    let row_h = 30.0;
    let row_gap = 3.0;
    let bx = right_x + 20.0;
    let bw = panel_w - 40.0;

    // if currently rebinding, listen for key or mouse input
    if let Some(action) = rebinding.clone() {
        // check mouse buttons first (Left, Right, Middle)
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
        // check keyboard
        if let Some(key) = get_last_key_pressed() {
            if key == KeyCode::Escape {
                // cancel rebinding
                *rebinding = None;
            } else {
                set_binding(config, &action, InputBinding::Key(key));
                *rebinding = None;
            }
            return;
        }
        return; // don't process clicks while rebinding
    }

    if !clicked {
        return;
    }

    // check clicks on bind rows
    for (i, &(_, action_id)) in BIND_ACTIONS.iter().enumerate() {
        let by = panel_top + 50.0 + (i as f32) * (row_h + row_gap);
        if mouse.0 >= bx && mouse.0 <= bx + bw && mouse.1 >= by && mouse.1 <= by + row_h {
            *rebinding = Some(action_id.to_string());
            return; // consume the click so it doesn't also fire as a mouse binding
        }
    }

    // crouch toggle button (after all bind rows)
    let toggle_y = panel_top + 50.0 + (BIND_ACTIONS.len() as f32) * (row_h + row_gap) + 10.0;
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
) {
    draw_rectangle(
        right_x,
        panel_top,
        panel_w,
        panel_h,
        Color::from_rgba(30, 30, 30, 240),
    );
    draw_rectangle_lines(right_x, panel_top, panel_w, panel_h, 1.0, GRAY);

    let kb_title = "Key Bindings";
    let kb_w = measure_game_text(kb_title, 22);
    draw_text_ex(
        kb_title,
        right_x + (panel_w - kb_w) / 2.0,
        panel_top + 30.0,
        text_params(22, YELLOW),
    );

    let row_h = 30.0;
    let row_gap = 3.0;
    let bx = right_x + 20.0;
    let bw = panel_w - 40.0;

    // collect all bindings for duplicate detection
    let all_bindings = config.all_bindings();

    for (i, &(label, action_id)) in BIND_ACTIONS.iter().enumerate() {
        let by = panel_top + 50.0 + (i as f32) * (row_h + row_gap);
        let binding = get_binding(config, action_id);

        // check if this binding is a duplicate
        let duplicate = all_bindings.iter().filter(|(_, b)| *b == binding).count() > 1;

        let is_rebinding = rebinding.as_deref() == Some(action_id);
        let bg_color = if is_rebinding {
            Color::from_rgba(80, 80, 20, 255)
        } else {
            Color::from_rgba(45, 45, 45, 255)
        };

        draw_rectangle(bx, by, bw, row_h, bg_color);
        draw_rectangle_lines(bx, by, bw, row_h, 1.0, GRAY);
        draw_text_ex(label, bx + 10.0, by + 20.0, text_params(18, WHITE));

        let key_text = if is_rebinding {
            "Press key/mouse...".to_string()
        } else {
            binding.display_name()
        };
        let key_tw = measure_game_text(&key_text, 18);
        draw_text_ex(
            &key_text,
            bx + bw - key_tw - 10.0,
            by + 20.0,
            text_params(18, if duplicate { RED } else { YELLOW }),
        );
    }

    // Crouch mode toggle button (after all bind rows)
    let toggle_y = panel_top + 50.0 + (BIND_ACTIONS.len() as f32) * (row_h + row_gap) + 10.0;

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
        bx + 10.0,
        toggle_y + 20.0,
        text_params(18, WHITE),
    );

    let mode_text = if config.crouch_toggle {
        "Toggle"
    } else {
        "Hold"
    };
    let mode_tw = measure_game_text(mode_text, 18);
    let mode_btn_w = 80.0;
    let mode_btn_x = bx + bw - mode_btn_w - 5.0;
    let mode_btn_y = toggle_y + 3.0;
    let mode_btn_h = row_h - 6.0;
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
        toggle_y + 19.0,
        text_params(18, WHITE),
    );
}
