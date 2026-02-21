use crate::net::protocols::{MODE_GUN_GAME, MODE_SEARCH_AND_DESTROY, MODE_TEAM_DEATHMATCH};
use crate::types::world::WorldId;
use macroquad::prelude::*;

use super::PlayingState;

impl PlayingState {
    /// Modal lobby console UI (pause-menu style). Blocks gameplay input while open.
    pub(super) fn draw_lobby_console(&mut self) {
        use crate::ui::is_button_clicked;

        if self.current_world != WorldId::Lobby || !self.lobby_console_open {
            return;
        }

        // Semi-transparent overlay
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::from_rgba(0, 0, 0, 150),
        );

        // make the console panel larger so longer messages fit comfortably
        let panel_w = 640.0;
        let panel_h = 380.0;
        let panel_x = (screen_width() - panel_w) / 2.0;
        let panel_y = (screen_height() - panel_h) / 2.0;

        draw_rectangle(
            panel_x,
            panel_y,
            panel_w,
            panel_h,
            Color::from_rgba(30, 30, 30, 240),
        );
        draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 2.0, WHITE);

        let title = "LOBBY CONSOLE";
        draw_text(&title, panel_x + 24.0, panel_y + 46.0, 32.0, WHITE);

        // small helper: word-wrap text to fit `max_w` using `font_size`
        let wrap_and_draw = |text: &str, max_w: f32, font_size: u16, y_start: f32, color: Color| {
            let mut lines: Vec<String> = Vec::new();
            let mut cur = String::new();
            for word in text.split_whitespace() {
                if cur.is_empty() {
                    cur.push_str(word);
                    continue;
                }
                let test = format!("{} {}", cur, word);
                if measure_text(&test, None, font_size, 1.0).width > max_w {
                    lines.push(cur.clone());
                    cur.clear();
                    cur.push_str(word);
                } else {
                    cur = test;
                }
            }
            if !cur.is_empty() {
                lines.push(cur);
            }

            for (i, line) in lines.iter().enumerate() {
                let dy = y_start + i as f32 * (font_size as f32 + 8.0);
                let tw = measure_text(line, None, font_size, 1.0);
                draw_text(
                    line,
                    panel_x + (panel_w - tw.width) / 2.0,
                    dy,
                    font_size as f32,
                    color,
                );
            }
            lines.len() as f32 * (font_size as f32 + 8.0)
        };

        let info = "Select a game mode and start a match for players currently in the lobby.";
        // allow up to panel_w - 48 px for wrapping
        let _info_h = wrap_and_draw(
            info,
            panel_w - 48.0,
            16,
            panel_y + 86.0,
            Color::from_rgba(200, 200, 200, 220),
        );

        // Game mode selection buttons — larger buttons for readability
        let mode_w = 180.0;
        let mode_h = 44.0;
        let gap = 18.0;
        let total_w = mode_w * 3.0 + gap * 2.0;
        let start_x_modes = panel_x + (panel_w - total_w) / 2.0;
        let modes: [(&str, u8); 3] = [
            ("Gun Game", MODE_GUN_GAME),
            ("Team Deathmatch", MODE_TEAM_DEATHMATCH),
            ("Search & Destroy", MODE_SEARCH_AND_DESTROY),
        ];
        for (i, (label, val)) in modes.iter().enumerate() {
            let x = start_x_modes + i as f32 * (mode_w + gap);
            let selected = self.pending_start_gamemode == *val;
            let bg = if selected {
                Color::from_rgba(40, 120, 40, 255)
            } else {
                Color::from_rgba(60, 60, 60, 255)
            };
            let hover = if selected {
                Color::from_rgba(50, 140, 50, 255)
            } else {
                DARKGRAY
            };
            if is_button_clicked(x, panel_y + 140.0, mode_w, mode_h, label, bg, hover) {
                self.pending_start_gamemode = *val;
            }
            if selected {
                draw_rectangle_lines(
                    x - 3.0,
                    panel_y + 136.0,
                    mode_w + 6.0,
                    mode_h + 6.0,
                    2.0,
                    YELLOW,
                );
            }
        }

        // Show transient console feedback (multi-line if needed)
        if let Some(msg) = &self.lobby_console_message {
            let _msg_h = wrap_and_draw(
                msg,
                panel_w - 64.0,
                16,
                panel_y + 200.0,
                Color::from_rgba(255, 120, 120, 255),
            );
        }

        // Action buttons — grouped horizontally at the bottom
        let start_btn_w = 280.0;
        let close_btn_w = 140.0;
        let btn_h = 48.0;
        let total_action_w = start_btn_w + 16.0 + close_btn_w;
        let actions_x = panel_x + (panel_w - total_action_w) / 2.0;
        let actions_y = panel_y + panel_h - 110.0;

        // Start shows currently-selected mode
        let start_label = match self.pending_start_gamemode {
            MODE_GUN_GAME => "Start Match (Gun Game)",
            MODE_TEAM_DEATHMATCH => "Start Match (TDM)",
            MODE_SEARCH_AND_DESTROY => "Start Match (S&D)",
            _ => "Start Match",
        };

        if is_button_clicked(
            actions_x,
            actions_y,
            start_btn_w,
            btn_h,
            start_label,
            Color::from_rgba(40, 80, 40, 255),
            DARKGRAY,
        ) {
            self.pending_start_request = true;
            self.lobby_console_open = false;
            set_cursor_grab(true);
            show_mouse(false);
            self.last_mouse_pos = mouse_position();
        }

        if is_button_clicked(
            actions_x + start_btn_w + 16.0,
            actions_y,
            close_btn_w,
            btn_h,
            "Close",
            Color::from_rgba(50, 50, 50, 255),
            DARKGRAY,
        ) {
            self.lobby_console_open = false;
            set_cursor_grab(true);
            show_mouse(false);
            self.last_mouse_pos = mouse_position();
        }

        // hint
        let hint = "Press ESC to close";
        let hint_w = measure_text(hint, None, 12, 1.0);
        draw_text(
            &hint,
            panel_x + (panel_w - hint_w.width) / 2.0,
            panel_y + panel_h - 18.0,
            12.0,
            Color::from_rgba(180, 180, 200, 200),
        );
    }

    /// Draws the in-world prompt for the lobby console ("[E] USE CONSOLE").
    pub(super) fn draw_lobby_console_prompt(&self) {
        use crate::world::lobby::LOBBY_CONSOLE_POS;
        if self.current_world != WorldId::Lobby {
            return;
        }

        let pos = LOBBY_CONSOLE_POS;
        let dist = vec3(self.position.x - pos.x, 0.0, self.position.z - pos.z).length();
        if dist < 2.5 {
            let text = "[E] USE CONSOLE";
            let measure = measure_text(text, None, 20, 1.0);
            draw_text(
                text,
                screen_width() / 2.0 - measure.width / 2.0,
                screen_height() / 2.0 + 60.0,
                20.0,
                WHITE,
            );
        }
    }

    /// Handles press-to-open / press-to-close behavior for the in-world lobby console.
    pub(super) fn check_lobby_console(&mut self) {
        use crate::world::lobby::LOBBY_CONSOLE_POS;
        if self.current_world != WorldId::Lobby {
            return;
        }

        // Close with Escape when open
        if self.lobby_console_open && is_key_pressed(KeyCode::Escape) {
            self.lobby_console_open = false;
            // restore FPS cursor state
            set_cursor_grab(true);
            show_mouse(false);
            self.last_mouse_pos = mouse_position();
            return;
        }

        let pos = LOBBY_CONSOLE_POS;
        let dist = vec3(self.position.x - pos.x, 0.0, self.position.z - pos.z).length();

        if dist < 2.0 && is_key_pressed(KeyCode::E) {
            // toggle and update cursor visibility/grab so UI becomes interactive
            self.lobby_console_open = !self.lobby_console_open;
            if self.lobby_console_open {
                set_cursor_grab(false);
                show_mouse(true);
            } else {
                set_cursor_grab(true);
                show_mouse(false);
                self.last_mouse_pos = mouse_position();
            }
        }
    }
}
