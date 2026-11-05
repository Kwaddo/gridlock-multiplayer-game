use crate::net::browser::Origin;
use crate::ui::fonts::{measure_game_text, text_params};

use macroquad::prelude::*;

use super::widgets::button;
use super::{in_rect, LoginScreen, ACCENT, INPUT_BG, INPUT_BORDER, ROW_BG, ROW_HOVER, TEXT_DIM, TEXT_LABEL};

const LAN_BADGE: Color = Color::new(0.30, 0.90, 0.45, 1.0);
const ONLINE_BADGE: Color = Color::new(0.35, 0.65, 1.0, 1.0);

impl LoginScreen {
    pub(super) fn draw_browse_tab(&mut self, px: f32, cy: f32, pw: f32, ch: f32, pad: f32) {
        let mouse = mouse_position();
        let clicked = is_mouse_button_pressed(MouseButton::Left);
        let lx = px + pad;
        let lw = pw - pad * 2.0;
        let col_pad = 14.0;

        // column headers
        let hdr_y = cy + 12.0;
        let hdr_h = 28.0;
        draw_rectangle(lx, hdr_y, lw, hdr_h, Color::new(0.08, 0.09, 0.16, 1.0));
        draw_rectangle(lx, hdr_y + hdr_h - 1.0, lw, 1.0, Color::new(ACCENT.r, ACCENT.g, ACCENT.b, 0.25));
        draw_text_ex("SERVER", lx + col_pad, hdr_y + 19.0, text_params(11, TEXT_DIM));
        let plw = measure_game_text("PLAYERS", 11);
        draw_text_ex("PLAYERS", lx + lw - plw - col_pad, hdr_y + 19.0, text_params(11, TEXT_DIM));

        let ly = hdr_y + hdr_h;
        let lh = ch - (ly - cy) - 44.0;
        draw_rectangle(lx, ly, lw, lh, INPUT_BG);
        draw_rectangle_lines(lx, hdr_y, lw, hdr_h + lh, 1.0, INPUT_BORDER);

        let searching = self.browser.is_searching();
        let mut join_addr: Option<String> = None;

        if self.browser.entries().is_empty() {
            let empty = if searching { "Searching for games..." } else { "No games found" };
            let ew = measure_game_text(empty, 14);
            draw_text_ex(empty, px + (pw - ew) / 2.0, ly + lh / 2.0 + 4.0, text_params(14, TEXT_DIM));
        }

        let row_h = 50.0;
        for (i, server) in self.browser.entries().iter().enumerate() {
            let ry = ly + row_h * i as f32;
            if ry + row_h > ly + lh {
                break;
            }
            let hover = in_rect(mouse, lx, ry, lw, row_h);
            let bg = if hover {
                ROW_HOVER
            } else if i % 2 == 0 {
                ROW_BG
            } else {
                Color::new(0.05, 0.06, 0.12, 1.0)
            };
            draw_rectangle(lx, ry, lw, row_h, bg);
            if hover {
                draw_rectangle(lx, ry, 3.0, row_h, ACCENT);
            }

            let (badge, badge_color) = match server.origin {
                Origin::Lan => ("LAN", LAN_BADGE),
                Origin::Online => ("ONLINE", ONLINE_BADGE),
            };
            let tx = lx + col_pad;
            let bw = measure_game_text(badge, 10) + 10.0;
            draw_rectangle_lines(tx, ry + 9.0, bw, 16.0, 1.0, badge_color);
            draw_text_ex(badge, tx + 5.0, ry + 21.0, text_params(10, badge_color));

            let name_color = if hover { WHITE } else { Color::new(0.88, 0.88, 0.95, 1.0) };
            draw_text_ex(&server.name, tx + bw + 8.0, ry + 22.0, text_params(16, name_color));
            draw_text_ex(&server.detail, tx, ry + 41.0, text_params(12, TEXT_DIM));

            let full = server.players >= server.max_players;
            let pc = format!("{}/{}", server.players, server.max_players);
            let pcw = measure_game_text(&pc, 15);
            let pc_color = if full { Color::new(1.0, 0.35, 0.35, 1.0) } else { TEXT_LABEL };
            draw_text_ex(&pc, lx + lw - pcw - col_pad, ry + 30.0, text_params(15, pc_color));

            if hover && clicked && !full {
                join_addr = Some(server.target.clone());
            }
        }

        // status line + refresh
        let bar_y = ly + lh + 10.0;
        let count = self.browser.entries().len();
        let mut status = if searching {
            format!("Searching{}", ".".repeat((self.anim_time * 3.0) as usize % 4))
        } else {
            format!("{count} game(s) found")
        };
        if let Some(note) = self.browser.online_note() {
            status = format!("{status}   ({note})");
        }
        draw_text_ex(&status, lx, bar_y + 17.0, text_params(13, TEXT_DIM));
        if button("REFRESH", 13, lx + lw - 92.0, bar_y, 92.0, 26.0) {
            self.refresh_servers();
        }

        if let Some(addr) = join_addr {
            self.join(&addr);
        }
    }
}
