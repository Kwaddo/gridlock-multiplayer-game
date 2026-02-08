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
    pub saved_config: Option<GameConfig>,
    pub show_unsaved_prompt: bool,
}

impl SettingsScreen {
    pub fn new() -> Self {
        Self {
            rebinding: None,
            saved_config: None,
            show_unsaved_prompt: false,
        }
    }

    pub fn enter(&mut self, config: &GameConfig) {
        self.saved_config = Some(config.clone());
        self.show_unsaved_prompt = false;
    }

    fn has_duplicates(config: &GameConfig) -> bool {
        let keys = [
            config.key_forward, config.key_backward, config.key_left,
            config.key_right, config.key_sprint, config.key_crouch, config.key_jump,
        ];
        for i in 0..keys.len() {
            for j in (i + 1)..keys.len() {
                if keys[i] == keys[j] {
                    return true;
                }
            }
        }
        false
    }

    fn has_unsaved_changes(&self, config: &GameConfig) -> bool {
        match &self.saved_config {
            Some(saved) => saved != config,
            None => false,
        }
    }
    pub fn update(&mut self, config: &mut GameConfig) -> bool {
        let mouse = mouse_position();
        let clicked = is_mouse_button_pressed(MouseButton::Left);
        let held = is_mouse_button_down(MouseButton::Left);

        // Handle unsaved changes prompt
        if self.show_unsaved_prompt {
            let pw = 350.0;
            let ph = 150.0;
            let px = (screen_width() - pw) / 2.0;
            let py = (screen_height() - ph) / 2.0;

            // Draw prompt overlay
            draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::from_rgba(0, 0, 0, 150));
            draw_rectangle(px, py, pw, ph, Color::from_rgba(40, 40, 40, 255));
            draw_rectangle_lines(px, py, pw, ph, 2.0, WHITE);

            let msg = "You have unsaved changes!";
            let msg_w = measure_text(msg, None, 20, 1.0).width;
            draw_text(msg, px + (pw - msg_w) / 2.0, py + 35.0, 20.0, YELLOW);

            // "Leave without saving" button
            let btn_w = 150.0;
            let btn_h = 35.0;
            let leave_x = px + 20.0;
            let btn_y = py + ph - btn_h - 20.0;
            draw_rectangle(leave_x, btn_y, btn_w, btn_h, Color::from_rgba(80, 30, 30, 255));
            draw_rectangle_lines(leave_x, btn_y, btn_w, btn_h, 1.0, WHITE);
            let lt = "Leave";
            let ltw = measure_text(lt, None, 18, 1.0).width;
            draw_text(lt, leave_x + (btn_w - ltw) / 2.0, btn_y + 24.0, 18.0, WHITE);

            // "Save & Leave" button
            let save_x = px + pw - btn_w - 20.0;
            let has_dupes = Self::has_duplicates(config);
            let save_color = if has_dupes { Color::from_rgba(50, 50, 50, 255) } else { Color::from_rgba(30, 80, 30, 255) };
            draw_rectangle(save_x, btn_y, btn_w, btn_h, save_color);
            draw_rectangle_lines(save_x, btn_y, btn_w, btn_h, 1.0, WHITE);
            let st = if has_dupes { "Fix duplicates" } else { "Save & Leave" };
            let stw = measure_text(st, None, 18, 1.0).width;
            draw_text(st, save_x + (btn_w - stw) / 2.0, btn_y + 24.0, 18.0, if has_dupes { GRAY } else { WHITE });

            if clicked {
                // Leave without saving
                if mouse.0 >= leave_x && mouse.0 <= leave_x + btn_w
                    && mouse.1 >= btn_y && mouse.1 <= btn_y + btn_h
                {
                    // Restore saved config
                    if let Some(saved) = &self.saved_config {
                        *config = saved.clone();
                    }
                    self.show_unsaved_prompt = false;
                    return true;
                }
                // Save & Leave (only if no duplicates)
                if !has_dupes && mouse.0 >= save_x && mouse.0 <= save_x + btn_w
                    && mouse.1 >= btn_y && mouse.1 <= btn_y + btn_h
                {
                    self.saved_config = Some(config.clone());
                    self.show_unsaved_prompt = false;
                    return true;
                }
            }

            if is_key_pressed(KeyCode::Escape) {
                self.show_unsaved_prompt = false;
            }

            return false;
        }

        // ESC tries to go back
        if is_key_pressed(KeyCode::Escape) {
            if self.has_unsaved_changes(config) {
                self.show_unsaved_prompt = true;
                return false;
            }
            return true;
        }

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

        // crouch toggle
        // text: "Crouch Mode: [Hold] / [Toggle]"

        let crouch_x = left_x + 20.0;
        let crouch_y = row_y2 + 60.0;
        let crouch_w = 200.0;
        let crouch_h = 30.0;
        if
            clicked &&
            mouse.0 >= crouch_x &&
            mouse.0 <= crouch_x + crouch_w &&
            mouse.1 >= crouch_y &&
            mouse.1 <= crouch_y + crouch_h
        {
            config.crouch_toggle = !config.crouch_toggle;
        }
        let crouch_text = if config.crouch_toggle { "[Toggle]" } else { "[Hold]" };
        draw_text(
            &format!("Crouch Mode: {}", crouch_text),
            crouch_x,
            crouch_y,
            20.0,
            WHITE,
        );

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
            if self.has_unsaved_changes(config) {
                self.show_unsaved_prompt = true;
            } else {
                return true;
            }
        }

        // Save button
        let save_w = 150.0;
        let save_x = back_x + back_w + 20.0;
        let has_dupes = Self::has_duplicates(config);
        if
            clicked &&
            !has_dupes &&
            mouse.0 >= save_x &&
            mouse.0 <= save_x + save_w &&
            mouse.1 >= back_y &&
            mouse.1 <= back_y + 40.0
        {
            self.saved_config = Some(config.clone());
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

        // Save button
        let save_x = back_x + back_w + 20.0;
        let has_dupes = Self::has_duplicates(config);
        let has_changes = self.has_unsaved_changes(config);
        let save_color = if has_dupes {
            Color::from_rgba(50, 50, 50, 255)
        } else if has_changes {
            Color::from_rgba(30, 100, 30, 255)
        } else {
            Color::from_rgba(40, 60, 40, 255)
        };
        draw_rectangle(save_x, back_y, back_w, 40.0, save_color);
        draw_rectangle_lines(save_x, back_y, back_w, 40.0, 1.0, if has_changes && !has_dupes { GREEN } else { WHITE });
        let save_label = if has_dupes { "Fix dupes" } else { "Save" };
        let save_tw = measure_text(save_label, None, 20, 1.0).width;
        draw_text(save_label, save_x + (back_w - save_tw) / 2.0, back_y + 26.0, 20.0, if has_dupes { GRAY } else { WHITE });

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
            
            // check if key appears more than once in binds (duplicate keys)
            let all_keys = [
                config.key_forward,
                config.key_backward,
                config.key_left,
                config.key_right,
                config.key_sprint,
                config.key_crouch,
                config.key_jump,
            ];
            let duplicate = all_keys.iter().filter(|&&k| k == *key).count() > 1;

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
            draw_text(&key_text, bx + bw - key_tw - 10.0, by + 20.0, 18.0, if duplicate { RED } else { YELLOW });
        }
    }
}
