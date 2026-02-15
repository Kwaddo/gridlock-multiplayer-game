use super::{LoginScreen, MainMenuTab};
use crate::screens::login::colors::*;
use crate::screens::login::util::in_rect;
use crate::ui::fonts::{measure_game_text, text_params};
use macroquad::prelude::*;

impl LoginScreen {
    pub fn draw(&mut self) {
        self.draw_background();
        self.draw_title();

        let sw = screen_width();
        let sh = screen_height();

        let panel_w = 520.0_f32.min(sw * 0.85);
        let panel_h = 380.0;
        let panel_x = (sw - panel_w) / 2.0;
        let panel_y = sh * 0.28;
        draw_rectangle(panel_x, panel_y, panel_w, panel_h, PANEL_BG);
        draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 1.0, PANEL_BORDER);

        let tab_h = 38.0;
        let tab_w = panel_w / 2.0;
        let mouse = mouse_position();
        let clicked = is_mouse_button_pressed(MouseButton::Left);

        let is_play = matches!(self.active_tab, MainMenuTab::Play);
        let play_hover = in_rect(mouse, panel_x, panel_y, tab_w, tab_h);
        if play_hover && clicked {
            self.active_tab = MainMenuTab::Play;
        }
        draw_rectangle(
            panel_x,
            panel_y,
            tab_w,
            tab_h,
            if is_play {
                TAB_ACTIVE_BG
            } else {
                Color::new(0.0, 0.0, 0.0, 0.0)
            },
        );
        if is_play {
            draw_rectangle(panel_x, panel_y + tab_h - 2.0, tab_w, 2.0, ACCENT);
        }
        let ptw = measure_game_text("PLAY", 18);
        draw_text_ex(
            "PLAY",
            panel_x + (tab_w - ptw) / 2.0,
            panel_y + 26.0,
            text_params(18, if is_play { WHITE } else { TAB_INACTIVE_TEXT }),
        );

        let sx = panel_x + tab_w;
        let is_srv = matches!(self.active_tab, MainMenuTab::Servers);
        let srv_hover = in_rect(mouse, sx, panel_y, tab_w, tab_h);
        if srv_hover && clicked {
            self.active_tab = MainMenuTab::Servers;
            if self.scan_socket.is_none() {
                self.start_scan();
            }
        }
        draw_rectangle(
            sx,
            panel_y,
            tab_w,
            tab_h,
            if is_srv {
                TAB_ACTIVE_BG
            } else {
                Color::new(0.0, 0.0, 0.0, 0.0)
            },
        );
        if is_srv {
            draw_rectangle(sx, panel_y + tab_h - 2.0, tab_w, 2.0, ACCENT);
        }
        let stw = measure_game_text("SERVERS", 18);
        draw_text_ex(
            "SERVERS",
            sx + (tab_w - stw) / 2.0,
            panel_y + 26.0,
            text_params(18, if is_srv { WHITE } else { TAB_INACTIVE_TEXT }),
        );

        draw_rectangle(
            panel_x,
            panel_y + tab_h,
            panel_w,
            1.0,
            Color::new(0.20, 0.24, 0.40, 0.5),
        );

        let cy = panel_y + tab_h + 1.0;
        let ch = panel_h - tab_h - 1.0;
        let pad = 36.0;

        match self.active_tab {
            MainMenuTab::Play => self.draw_play_tab(panel_x, cy, panel_w, ch, pad),
            MainMenuTab::Servers => self.draw_servers_tab(panel_x, cy, panel_w, ch, pad),
        }

        let fy = panel_y + panel_h + 16.0;
        draw_text_ex(
            "F11 fullscreen   |   TAB switch fields   |   ENTER connect",
            panel_x,
            fy,
            text_params(13, TEXT_DIM),
        );

        if self.connecting {
            draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.55));
            let t = "Connecting...";
            let tw = measure_game_text(t, 28);
            draw_text_ex(t, (sw - tw) / 2.0, sh / 2.0, text_params(28, ACCENT));
        }
    }
}
