use macroquad::prelude::*;
use crate::core::GameConfig;
use crate::ui::fonts::{text_params, measure_game_text};

pub fn update_keybinds(
    config: &mut GameConfig,
    rebinding: &mut Option<String>,
    right_x: f32,
    panel_top: f32,
    panel_w: f32,
    mouse: (f32, f32),
    clicked: bool,
) {
    let bind_actions = ["forward", "backward", "left", "right", "sprint", "crouch", "jump"];

    if clicked && rebinding.is_none() {
        for (i, action) in bind_actions.iter().enumerate() {
            let by = panel_top + 50.0 + (i as f32) * 35.0;
            let bx = right_x + 20.0;
            let bw = panel_w - 40.0;
            let bh = 30.0;
            if mouse.0 >= bx && mouse.0 <= bx + bw && mouse.1 >= by && mouse.1 <= by + bh {
                *rebinding = Some(action.to_string());
            }
        }

        // Crouch toggle button
        let toggle_y = panel_top + 50.0 + 7.0 * 35.0 + 10.0;
        let bx = right_x + 20.0;
        let bw = panel_w - 40.0;
        let bh = 30.0;
        if mouse.0 >= bx && mouse.0 <= bx + bw
            && mouse.1 >= toggle_y && mouse.1 <= toggle_y + bh
        {
            config.crouch_toggle = !config.crouch_toggle;
        }
    }

    if let Some(action) = rebinding {
        if let Some(key_code) = get_last_key_pressed() {
            match action.as_str() {
                "forward" => config.key_forward = key_code,
                "backward" => config.key_backward = key_code,
                "left" => config.key_left = key_code,
                "right" => config.key_right = key_code,
                "sprint" => config.key_sprint = key_code,
                "crouch" => config.key_crouch = key_code,
                "jump" => config.key_jump = key_code,
                _ => {}
            }
            *rebinding = None;
        }
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
    draw_rectangle(right_x, panel_top, panel_w, panel_h, Color::from_rgba(30, 30, 30, 240));
    draw_rectangle_lines(right_x, panel_top, panel_w, panel_h, 1.0, GRAY);

    let kb_title = "Key Bindings";
    let kb_w = measure_game_text(kb_title, 22);
    draw_text_ex(
        kb_title,
        right_x + (panel_w - kb_w) / 2.0,
        panel_top + 30.0,
        text_params(22, YELLOW),
    );

    let binds: [(&str, KeyCode); 7] = [
        ("Forward", config.key_forward),
        ("Backward", config.key_backward),
        ("Left", config.key_left),
        ("Right", config.key_right),
        ("Sprint", config.key_sprint),
        ("Crouch", config.key_crouch),
        ("Jump", config.key_jump),
    ];

    let bind_actions = ["forward", "backward", "left", "right", "sprint", "crouch", "jump"];

    let all_keys = [
        config.key_forward, config.key_backward, config.key_left,
        config.key_right, config.key_sprint, config.key_crouch, config.key_jump,
    ];

    for (i, (label, key)) in binds.iter().enumerate() {
        let by = panel_top + 50.0 + (i as f32) * 35.0;
        let bx = right_x + 20.0;
        let bw = panel_w - 40.0;
        let bh = 30.0;

        let duplicate = all_keys.iter().filter(|&&k| k == *key).count() > 1;
        let is_rebinding = rebinding.as_deref() == Some(bind_actions[i]);
        let bg_color = if is_rebinding {
            Color::from_rgba(80, 80, 20, 255)
        } else {
            Color::from_rgba(45, 45, 45, 255)
        };

        draw_rectangle(bx, by, bw, bh, bg_color);
        draw_rectangle_lines(bx, by, bw, bh, 1.0, GRAY);
        draw_text_ex(label, bx + 10.0, by + 20.0, text_params(18, WHITE));

        let key_text = if is_rebinding {
            "Press a key...".to_string()
        } else {
            format!("{:?}", key)
        };
        let key_tw = measure_game_text(&key_text, 18);
        draw_text_ex(
            &key_text,
            bx + bw - key_tw - 10.0,
            by + 20.0,
            text_params(18, if duplicate { RED } else { YELLOW }),
        );
    }

    // Crouch mode toggle button
    let toggle_y = panel_top + 50.0 + 7.0 * 35.0 + 10.0;
    let bx = right_x + 20.0;
    let bw = panel_w - 40.0;
    let bh = 30.0;

    let mouse = mouse_position();
    let hover = mouse.0 >= bx && mouse.0 <= bx + bw
        && mouse.1 >= toggle_y && mouse.1 <= toggle_y + bh;

    let bg = if hover {
        Color::from_rgba(65, 65, 85, 255)
    } else {
        Color::from_rgba(55, 55, 75, 255)
    };
    draw_rectangle(bx, toggle_y, bw, bh, bg);
    draw_rectangle_lines(bx, toggle_y, bw, bh, 1.0, GRAY);
    draw_text_ex("Crouch Mode", bx + 10.0, toggle_y + 20.0, text_params(18, WHITE));

    // Mode button inset on the right side of the row
    let mode_text = if config.crouch_toggle { "Toggle" } else { "Hold" };
    let mode_tw = measure_game_text(mode_text, 18);
    let mode_btn_w = 80.0;
    let mode_btn_x = bx + bw - mode_btn_w - 5.0;
    let mode_btn_y = toggle_y + 3.0;
    let mode_btn_h = bh - 6.0;
    let mode_btn_color = if config.crouch_toggle {
        Color::from_rgba(30, 100, 30, 255)
    } else {
        Color::from_rgba(100, 60, 20, 255)
    };
    draw_rectangle(mode_btn_x, mode_btn_y, mode_btn_w, mode_btn_h, mode_btn_color);
    draw_rectangle_lines(mode_btn_x, mode_btn_y, mode_btn_w, mode_btn_h, 1.0, WHITE);
    draw_text_ex(
        mode_text,
        mode_btn_x + (mode_btn_w - mode_tw) / 2.0,
        toggle_y + 19.0,
        text_params(18, WHITE),
    );
}
