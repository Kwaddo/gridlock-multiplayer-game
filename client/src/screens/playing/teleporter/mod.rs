mod constants;
mod controls;
mod helpers;
mod host_setup;
mod roster;

use crate::net::protocols::{
    MATCH_STATUS_ENDING, MATCH_STATUS_RUNNING, MATCH_STATUS_STARTING, MODE_GUN_GAME,
    MODE_SEARCH_AND_DESTROY, MODE_TEAM_DEATHMATCH,
};
use crate::types::world::WorldId;
use macroquad::prelude::*;

use super::PlayingState;

use constants::{
    ACCENT, ACCENT_HOVER, BTN_CLOSE_BG, BTN_CLOSE_HOVER, MENU_BG_OVL, PANEL_BG, PANEL_BORDER, SEP,
    TEXT_DIM, TEXT_LABEL,
};

impl PlayingState {
    /// Modal lobby console UI. Blocks gameplay input while open.
    pub(super) fn draw_lobby_console(&mut self, config: &mut crate::core::GameConfig, player_id: u32) {
        use crate::ui::fonts::{measure_game_text, text_params};

        if self.current_world != WorldId::Lobby || !self.lobby_console_open {
            return;
        }

        let sw = screen_width();
        let sh = screen_height();
        let t = get_time() as f32;

        // ── backdrop ──────────────────────────────────────────────────────────
        draw_rectangle(0.0, 0.0, sw, sh, MENU_BG_OVL);

        // ── panel ─────────────────────────────────────────────────────────────
        let panel_w = 600.0_f32.min(sw * 0.88);
        let panel_h = 620.0_f32.min(sh * 0.92);
        let panel_x = (sw - panel_w) / 2.0;
        let panel_y = (sh - panel_h) / 2.0;

        draw_rectangle(panel_x, panel_y, panel_w, panel_h, PANEL_BG);
        draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 1.0, PANEL_BORDER);
        // top accent line
        draw_rectangle(panel_x, panel_y, panel_w, 2.0, ACCENT);

        // ── title ─────────────────────────────────────────────────────────────
        let title = "LOBBY CONSOLE";
        let ts: u16 = 26;
        let tw = measure_game_text(title, ts);
        let glow = ((t * 0.9).sin() * 0.10 + 0.90).clamp(0.80, 1.0);
        draw_text_ex(
            title,
            panel_x + (panel_w - tw) / 2.0 + 1.5,
            panel_y + 38.0,
            text_params(ts, Color::new(0.0, 0.0, 0.0, 0.55)),
        );
        draw_text_ex(
            title,
            panel_x + (panel_w - tw) / 2.0,
            panel_y + 38.0,
            text_params(ts, Color::new(glow, glow, glow, 1.0)),
        );
        // underline accent bar (matches login title style)
        let bar_w = tw * 0.50;
        draw_rectangle(
            panel_x + (panel_w - bar_w) / 2.0,
            panel_y + 44.0,
            bar_w,
            2.0,
            ACCENT,
        );

        // ── section separator ─────────────────────────────────────────────────
        let sep_y = panel_y + 56.0;
        draw_rectangle(panel_x + 24.0, sep_y, panel_w - 48.0, 1.0, SEP);

        // ── body: the host sets the next match up; everyone else sees it ─────
        let mouse = mouse_position();
        let clicked = is_mouse_button_pressed(MouseButton::Left);
        if self.is_host(player_id) {
            self.draw_host_setup(config, panel_x, panel_w, sep_y, player_id, mouse, clicked);
            // share what changed with the other players
            let settings = self.lobby_settings_now(config);
            if self.sent_lobby_settings != Some(settings) {
                self.sent_lobby_settings = Some(settings);
                self.pending_lobby_settings = Some(settings);
            }
        } else {
            // if this player becomes host, it sends its settings afresh
            self.sent_lobby_settings = None;
            self.draw_guest_info(panel_x, panel_w, sep_y);
        }

        // ── console feedback message ───────────────────────────────────────────
        if let Some(msg) = &self.lobby_console_message {
            let msg_y = panel_y + panel_h - 76.0;
            let mw = measure_game_text(msg, 14);
            let alpha = (self.lobby_console_message_timer / 3.0).clamp(0.0, 1.0);
            draw_text_ex(
                msg,
                panel_x + (panel_w - mw) / 2.0,
                msg_y,
                text_params(14, Color::new(1.0, 0.65, 0.42, alpha)),
            );
        }

        // ── action buttons ─────────────────────────────────────────────────────
        let btn_area_y = panel_y + panel_h - 56.0;
        let start_btn_w = 260.0;
        let close_btn_w = 120.0;
        let btn_h = 40.0;
        let total_btn_w = start_btn_w + 12.0 + close_btn_w;
        let btn_start_x = panel_x + (panel_w - total_btn_w) / 2.0;

        // What the main button does depends on the match phase: the host starts
        // a match; while one runs, anyone in the lobby can join it.
        let start_label = match self.pending_start_gamemode {
            MODE_GUN_GAME => "START  GUN GAME",
            MODE_TEAM_DEATHMATCH => "START  TEAM DEATHMATCH",
            MODE_SEARCH_AND_DESTROY => "START  SEARCH & DESTROY",
            _ => "START MATCH",
        };
        let (action_label, action_enabled) = match self.match_status {
            MATCH_STATUS_RUNNING => ("JOIN MATCH", true),
            MATCH_STATUS_STARTING => ("MATCH STARTING...", false),
            MATCH_STATUS_ENDING => ("MATCH ENDING...", false),
            _ if self.is_host(player_id) => (start_label, true),
            _ => ("WAITING FOR HOST...", false),
        };

        if action_enabled {
            // orange accent (matches login CONNECT button)
            let s_hov = mouse.0 >= btn_start_x
                && mouse.0 <= btn_start_x + start_btn_w
                && mouse.1 >= btn_area_y
                && mouse.1 <= btn_area_y + btn_h;
            draw_rectangle(
                btn_start_x,
                btn_area_y,
                start_btn_w,
                btn_h,
                if s_hov { ACCENT_HOVER } else { ACCENT },
            );
            let slw = measure_game_text(action_label, 17);
            draw_text_ex(
                action_label,
                btn_start_x + (start_btn_w - slw) / 2.0,
                btn_area_y + 27.0,
                text_params(17, Color::new(0.05, 0.05, 0.05, 1.0)),
            );
            if s_hov && clicked {
                // the same request starts a match or joins the running one
                self.pending_start_request = true;
                self.lobby_console_open = false;
                set_cursor_grab(true);
                show_mouse(false);
                self.last_mouse_pos = mouse_position();
            }
        } else {
            // disabled: waiting for the host, or for the match to start/finish
            draw_rectangle(
                btn_start_x,
                btn_area_y,
                start_btn_w,
                btn_h,
                Color::new(0.10, 0.11, 0.20, 1.0),
            );
            draw_rectangle_lines(btn_start_x, btn_area_y, start_btn_w, btn_h, 1.0, PANEL_BORDER);
            let wlw = measure_game_text(action_label, 17);
            draw_text_ex(
                action_label,
                btn_start_x + (start_btn_w - wlw) / 2.0,
                btn_area_y + 27.0,
                text_params(17, TEXT_DIM),
            );
        }

        // Close — muted dark button
        let close_x = btn_start_x + start_btn_w + 12.0;
        let c_hov = mouse.0 >= close_x
            && mouse.0 <= close_x + close_btn_w
            && mouse.1 >= btn_area_y
            && mouse.1 <= btn_area_y + btn_h;
        draw_rectangle(
            close_x,
            btn_area_y,
            close_btn_w,
            btn_h,
            if c_hov { BTN_CLOSE_HOVER } else { BTN_CLOSE_BG },
        );
        draw_rectangle_lines(close_x, btn_area_y, close_btn_w, btn_h, 1.0, PANEL_BORDER);
        let clw = measure_game_text("CLOSE", 17);
        draw_text_ex(
            "CLOSE",
            close_x + (close_btn_w - clw) / 2.0,
            btn_area_y + 27.0,
            text_params(17, TEXT_LABEL),
        );
        if c_hov && clicked {
            self.lobby_console_open = false;
            set_cursor_grab(true);
            show_mouse(false);
            self.last_mouse_pos = mouse_position();
        }

        // ── ESC hint ──────────────────────────────────────────────────────────
        let hw = measure_game_text("ESC to close", 12);
        draw_text_ex(
            "ESC to close",
            panel_x + (panel_w - hw) / 2.0,
            panel_y + panel_h - 8.0,
            text_params(12, TEXT_DIM),
        );

    }

    /// Draws the in-world prompt for the lobby console ("[E] USE CONSOLE").
    pub(super) fn draw_lobby_console_prompt(&self) {
        use crate::ui::fonts::{measure_game_text, text_params};
        use crate::world::lobby::LOBBY_CONSOLE_POS;
        if self.current_world != WorldId::Lobby {
            return;
        }

        let pos = LOBBY_CONSOLE_POS;
        let dist = vec3(self.position.x - pos.x, 0.0, self.position.z - pos.z).length();
        if dist < 2.5 {
            let text = "[E]  USE CONSOLE";
            let tw = measure_game_text(text, 18);
            let sx = screen_width() / 2.0 - tw / 2.0;
            let sy = screen_height() / 2.0 + 64.0;
            draw_text_ex(
                text,
                sx + 1.0,
                sy + 1.0,
                text_params(18, Color::new(0.0, 0.0, 0.0, 0.5)),
            );
            draw_text_ex(text, sx, sy, text_params(18, ACCENT));
        }
    }

    pub(super) fn draw_shooting_range_prompt(&self) {
        use crate::ui::fonts::{measure_game_text, text_params};
        use crate::world::shooting_range::RANGE_START_BUTTON_POS;
        if self.current_world != WorldId::Lobby || self.lobby_console_open {
            return;
        }

        let pos = RANGE_START_BUTTON_POS;
        let dist = vec3(self.position.x - pos.x, 0.0, self.position.z - pos.z).length();
        if dist < 2.4 {
            let text = "[E]  START RANGE";
            let tw = measure_game_text(text, 18);
            let sx = screen_width() / 2.0 - tw / 2.0;
            let sy = screen_height() / 2.0 + 92.0;
            draw_text_ex(
                text,
                sx + 1.0,
                sy + 1.0,
                text_params(18, Color::new(0.0, 0.0, 0.0, 0.5)),
            );
            draw_text_ex(text, sx, sy, text_params(18, ACCENT));
        }
    }

    /// Handles press-to-open / press-to-close behavior for the in-world lobby console.
    pub(super) fn check_lobby_console(&mut self) {
        use crate::world::lobby::LOBBY_CONSOLE_POS;
        if self.current_world != WorldId::Lobby {
            return;
        }

        if self.lobby_console_open && is_key_pressed(KeyCode::Escape) {
            self.lobby_console_open = false;
            set_cursor_grab(true);
            show_mouse(false);
            self.last_mouse_pos = mouse_position();
            return;
        }

        let pos = LOBBY_CONSOLE_POS;
        let dist = vec3(self.position.x - pos.x, 0.0, self.position.z - pos.z).length();

        if dist < 2.0 && is_key_pressed(KeyCode::E) {
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

    pub(super) fn check_shooting_range_start(&mut self) {
        use crate::world::shooting_range::RANGE_START_BUTTON_POS;
        if self.current_world != WorldId::Lobby {
            return;
        }
        let pos = RANGE_START_BUTTON_POS;
        let dist = vec3(self.position.x - pos.x, 0.0, self.position.z - pos.z).length();
        if dist < 2.0 && is_key_pressed(KeyCode::E) {
            self.pending_range_start = true;
        }
    }
}
