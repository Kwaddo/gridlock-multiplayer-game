use macroquad::prelude::*;
use crate::config::GameConfig;

// UI display ranges (what the user sees)
const SENS_MIN: f32 = 10.0;
const SENS_MAX: f32 = 100.0;
const SENS_STEP: f32 = 1.0;

const FOV_MIN: f32 = 40.0;
const FOV_MAX: f32 = 120.0;
const FOV_STEP: f32 = 5.0;


pub struct SettingsScreen {
    pub rebinding: Option<String>,
}

impl SettingsScreen {
    pub fn new() -> Self {
        Self {
            rebinding: None,
        }
    }
    pub fn update(&mut self, config: &mut GameConfig) -> bool {
        if is_key_pressed(KeyCode::Escape) {
            return true;
        }

        let mouse = mouse_position();
        let clicked = is_mouse_button_pressed(MouseButton::Left);
        let held = is_mouse_button_down(MouseButton::Left);

        // Layout
        let gap = 20.0;
        let panel_w = (screen_width() - gap * 3.0) / 2.0;
        let left_x = gap;
        let panel_top = 80.0;

        // Sensitivity slider
        let row_y = panel_top + 50.0;
        let slider_x = left_x + 160.0; // position of slider bar
        let slider_w = panel_w - 220.0; // width of slider bar
        let slider_h = 20.0; // height of slider bar
        let btn_size = 25.0;

        // < button
        let lt_x = slider_x - btn_size - 5.0;
        if
            clicked &&
            mouse.0 >= lt_x &&
            mouse.0 <= lt_x + btn_size &&
            mouse.1 >= row_y &&
            mouse.1 <= row_y + btn_size
        {
            config.sensitivity = (config.sensitivity - SENS_STEP).max(SENS_MIN);
        }
        // > button
        let gt_x = slider_x + slider_w + 5.0;
        if
            clicked &&
            mouse.0 >= gt_x &&
            mouse.0 <= gt_x + btn_size &&
            mouse.1 >= row_y &&
            mouse.1 <= row_y + btn_size
        {
            config.sensitivity = (config.sensitivity + SENS_STEP).min(SENS_MAX);
        }

        // Slider bar
        if
            held &&
            mouse.1 >= row_y &&
            mouse.1 <= row_y + slider_h &&
            mouse.0 >= slider_x &&
            mouse.0 <= slider_x + slider_w
        {
            let t = (mouse.0 - slider_x) / slider_w;
            config.sensitivity = SENS_MIN + t * (SENS_MAX - SENS_MIN);
        }

        // FOV Slider
        let row_y2 = row_y + 60.0;
        let lt_x2 = slider_x - btn_size - 5.0;
        if
            clicked &&
            mouse.0 >= lt_x2 &&
            mouse.0 <= lt_x2 + btn_size &&
            mouse.1 >= row_y2 &&
            mouse.1 <= row_y2 + btn_size
        {
            config.fov = (config.fov - FOV_STEP).max(FOV_MIN);
        }
        let gt_x2 = slider_x + slider_w + 5.0;
        if
            clicked &&
            mouse.0 >= gt_x2 &&
            mouse.0 <= gt_x2 + btn_size &&
            mouse.1 >= row_y2 &&
            mouse.1 <= row_y2 + btn_size
        {
            config.fov = (config.fov + FOV_STEP).min(FOV_MAX);
        }
        if
            held &&
            mouse.1 >= row_y2 &&
            mouse.1 <= row_y2 + slider_h &&
            mouse.0 >= slider_x &&
            mouse.0 <= slider_x + slider_w
        {
            let t = (mouse.0 - slider_x) / slider_w;
            config.fov = FOV_MIN + t * (FOV_MAX - FOV_MIN);
        }

        // Back button
        let back_y = screen_height() - 60.0;
        let back_w = 150.0;
        let back_x = (screen_width() - back_w) / 2.0;
        if
            clicked &&
            mouse.0 >= back_x &&
            mouse.0 <= back_x + back_w &&
            mouse.1 >= back_y &&
            mouse.1 <= back_y + 40.0
        {
            return true;
        }

        // Reset button
        let reset_w = 150.0;
        let reset_x = back_x - reset_w - 20.0;
        if
            clicked &&
            mouse.0 >= reset_x &&
            mouse.0 <= reset_x + reset_w &&
            mouse.1 >= back_y &&
            mouse.1 <= back_y + 40.0
        {
            config.reset();
        }

        // Keybind rows click detection
        let right_x = gap * 2.0 + panel_w;
        let bind_actions = ["forward", "backward", "left", "right", "sprint", "crouch", "jump"];
        if clicked && self.rebinding.is_none() {
            for (i, action) in bind_actions.iter().enumerate() {
                let by = panel_top + 50.0 + (i as f32) * 35.0;
                let bx = right_x + 20.0;
                let bw = panel_w - 40.0;
                let bh = 30.0;
                if mouse.0 >= bx && mouse.0 <= bx + bw && mouse.1 >= by && mouse.1 <= by + bh {
                    self.rebinding = Some(action.to_string());
                }
            }
        }

        if let Some(action) = &self.rebinding {
            if let Some(key_code) = get_last_key_pressed() {
                match action.as_str() {
                    "forward" => {
                        config.key_forward = key_code;
                    }
                    "backward" => {
                        config.key_backward = key_code;
                    }
                    "left" => {
                        config.key_left = key_code;
                    }
                    "right" => {
                        config.key_right = key_code;
                    }
                    "sprint" => {
                        config.key_sprint = key_code;
                    }
                    "crouch" => {
                        config.key_crouch = key_code;
                    }
                    "jump" => {
                        config.key_jump = key_code;
                    }
                    _ => {}
                }
                self.rebinding = None;
            }
        }
        false
    }
    pub fn draw(&self, config: &GameConfig) {
        // Title
        let title = "SETTINGS";
        let title_w = measure_text(title, None, 30, 1.0).width;
        draw_text(title, (screen_width() - title_w) / 2.0, 50.0, 30.0, WHITE);

        // Layout
        let gap = 20.0;
        let panel_w = (screen_width() - gap * 3.0) / 2.0;
        let left_x = gap;
        let right_x = gap * 2.0 + panel_w;
        let panel_top = 80.0;
        let panel_h = screen_height() - 150.0; // leave some space at bottom for back button

        // Left Panel - General settings
        draw_rectangle(left_x, panel_top, panel_w, panel_h, Color::from_rgba(30, 30, 30, 240));
        draw_rectangle_lines(left_x, panel_top, panel_w, panel_h, 1.0, GRAY);

        let section_title = "General";
        let section_w = measure_text(section_title, None, 22, 1.0).width;
        draw_text(
            section_title,
            left_x + (panel_w - section_w) / 2.0,
            panel_top + 30.0,
            22.0,
            YELLOW
        );

        // Mouse sensitivity
        let row_y = panel_top + 60.0;
        let label_x = left_x + 20.0;
        draw_text("Sensitivity", label_x, row_y, 18.0, WHITE);

        // Sensitivity slider
        let slider_x = left_x + 160.0;
        let slider_w = panel_w - 220.0;
        let slider_h = 20.0;
        let btn_size = 25.0;
        let row_y = panel_top + 50.0;

        // < button
        draw_rectangle(
            slider_x - btn_size - 5.0,
            row_y,
            btn_size,
            btn_size,
            Color::from_rgba(60, 60, 60, 255)
        );
        draw_text("<", slider_x - btn_size + 3.0, row_y + 18.0, 20.0, WHITE);
        // Slider bar
        draw_rectangle(slider_x, row_y, slider_w, slider_h, Color::from_rgba(40, 40, 40, 255));
        let sens_t = (config.sensitivity - SENS_MIN) / (SENS_MAX - SENS_MIN); // normalize 0-1
        draw_rectangle(
            slider_x,
            row_y,
            slider_w * sens_t,
            slider_h,
            Color::from_rgba(80, 150, 220, 255)
        );
        draw_rectangle_lines(slider_x, row_y, slider_w, slider_h, 1.0, GRAY);
        // > button
        draw_rectangle(
            slider_x + slider_w + 5.0,
            row_y,
            btn_size,
            btn_size,
            Color::from_rgba(60, 60, 60, 255)
        );
        draw_text(">", slider_x + slider_w + 13.0, row_y + 18.0, 20.0, WHITE);

        // Value text
        draw_text(
            &format!("{:.0}", config.sensitivity),
            slider_x + slider_w + 40.0,
            row_y + 16.0,
            16.0,
            WHITE
        );

        // FOV slider
        let row_y2 = row_y + 60.0;
        draw_text("FOV", label_x, row_y2 + 16.0, 18.0, WHITE);
        draw_rectangle(
            slider_x - btn_size - 5.0,
            row_y2,
            btn_size,
            btn_size,
            Color::from_rgba(60, 60, 60, 255)
        );
        draw_text("<", slider_x - btn_size + 3.0, row_y2 + 18.0, 20.0, WHITE);
        draw_rectangle(slider_x, row_y2, slider_w, slider_h, Color::from_rgba(40, 40, 40, 255));
        let fov_t = ((config.fov - FOV_MIN) / (FOV_MAX - FOV_MIN)).clamp(0.0, 1.0);
        draw_rectangle(
            slider_x,
            row_y2,
            slider_w * fov_t,
            slider_h,
            Color::from_rgba(80, 150, 220, 255)
        );
        draw_rectangle_lines(slider_x, row_y2, slider_w, slider_h, 1.0, GRAY);
        draw_rectangle(
            slider_x + slider_w + 5.0,
            row_y2,
            btn_size,
            btn_size,
            Color::from_rgba(60, 60, 60, 255)
        );
        draw_text(">", slider_x + slider_w + 13.0, row_y2 + 18.0, 20.0, WHITE);
        draw_text(
            &format!("{:.0}", config.fov),
            slider_x + slider_w + 40.0,
            row_y2 + 16.0,
            16.0,
            WHITE
        );

        // Back button
        let back_y = screen_height() - 60.0;
        let back_w = 150.0;
        let back_x = (screen_width() - back_w) / 2.0;
        draw_rectangle(back_x, back_y, back_w, 40.0, Color::from_rgba(50, 50, 50, 255));
        draw_rectangle_lines(back_x, back_y, back_w, 40.0, 1.0, WHITE);
        let back_tw = measure_text("Back", None, 20, 1.0).width;
        draw_text("Back", back_x + (back_w - back_tw) / 2.0, back_y + 26.0, 20.0, WHITE);

        // Reset button
        let reset_x = back_x - back_w - 20.0;
        draw_rectangle(reset_x, back_y, back_w, 40.0, Color::from_rgba(80, 60, 20, 255));
        draw_rectangle_lines(reset_x, back_y, back_w, 40.0, 1.0, WHITE);
        let reset_tw = measure_text("Reset", None, 20, 1.0).width;
        draw_text("Reset", reset_x + (back_w - reset_tw) / 2.0, back_y + 26.0, 20.0, WHITE);

        // Right Panel - Key Bindings
        draw_rectangle(right_x, panel_top, panel_w, panel_h, Color::from_rgba(30, 30, 30, 240));
        draw_rectangle_lines(right_x, panel_top, panel_w, panel_h, 1.0, GRAY);

        let kb_title = "Key Bindings";
        let kb_w = measure_text(kb_title, None, 22, 1.0).width;
        draw_text(
            kb_title,
            right_x + (panel_w - kb_w) / 2.0,
            panel_top + 30.0,
            22.0,
            YELLOW
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

        for (i, (label, key)) in binds.iter().enumerate() {
            let by = panel_top + 50.0 + (i as f32) * 35.0;
            let bx = right_x + 20.0;
            let bw = panel_w - 40.0;
            let bh = 30.0;

            let is_rebinding = self.rebinding.as_deref() == Some(bind_actions[i]);
            let bg_color = if is_rebinding {
                Color::from_rgba(80, 80, 20, 255)
            } else {
                Color::from_rgba(45, 45, 45, 255)
            };

            draw_rectangle(bx, by, bw, bh, bg_color);
            draw_rectangle_lines(bx, by, bw, bh, 1.0, GRAY);

            draw_text(label, bx + 10.0, by + 20.0, 18.0, WHITE);

            let key_text = if is_rebinding {
                "Press a key...".to_string()
            } else {
                format!("{:?}", key)
            };
            let key_tw = measure_text(&key_text, None, 18, 1.0).width;
            draw_text(&key_text, bx + bw - key_tw - 10.0, by + 20.0, 18.0, YELLOW);
        }
    }
}
