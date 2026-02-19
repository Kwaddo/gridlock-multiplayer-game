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

        let panel_w = 380.0;
        let panel_h = 220.0;
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
        let tw = measure_text(title, None, 26, 1.0);
        draw_text(
            &title,
            (screen_width() - tw.width) / 2.0,
            panel_y + 38.0,
            26.0,
            WHITE,
        );

        let info = "Start a match for players currently in the lobby.";
        let info_w = measure_text(info, None, 16, 1.0);
        draw_text(
            &info,
            (screen_width() - info_w.width) / 2.0,
            panel_y + 74.0,
            16.0,
            Color::from_rgba(200, 200, 200, 220),
        );

        // Show transient console feedback (e.g. "match running / full")
        if let Some(msg) = &self.lobby_console_message {
            let mw = measure_text(msg, None, 14, 1.0);
            draw_text(
                msg,
                panel_x + (panel_w - mw.width) / 2.0,
                panel_y + 100.0,
                14.0,
                Color::from_rgba(255, 120, 120, 255),
            );
        }

        let btn_w = 220.0;
        let btn_h = 44.0;
        let btn_x = panel_x + (panel_w - btn_w) / 2.0;

        if is_button_clicked(
            btn_x,
            panel_y + 110.0,
            btn_w,
            btn_h,
            "Start Match",
            Color::from_rgba(40, 80, 40, 255),
            DARKGRAY,
        ) {
            self.pending_start_request = true;
            self.lobby_console_open = false;
            // return cursor to FPS mode
            set_cursor_grab(true);
            show_mouse(false);
            self.last_mouse_pos = mouse_position();
        }

        if is_button_clicked(
            btn_x,
            panel_y + 170.0,
            btn_w,
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
            panel_y + panel_h - 12.0,
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
