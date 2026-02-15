use super::{LoginScreen, MainMenuTab};
use crate::screens::login::colors::*;
use crate::screens::login::util::in_rect;
use crate::ui::fonts::{measure_game_text, text_params};
use macroquad::prelude::*;

impl LoginScreen {
    pub(super) fn draw_servers_tab(&mut self, px: f32, cy: f32, pw: f32, ch: f32, pad: f32) {
        let mouse = mouse_position();
        let clicked = is_mouse_button_pressed(MouseButton::Left);
        let scanning = self.scan_timer > 0.0;

        let hdr_y = cy + 10.0;
        let hdr_h = 30.0;
        let lx = px + pad;
        let lw = pw - pad * 2.0;
        draw_rectangle(lx, hdr_y, lw, hdr_h, Color::new(0.08, 0.09, 0.16, 1.0));
        draw_rectangle(
            lx,
            hdr_y + hdr_h - 1.0,
            lw,
            1.0,
            Color::new(ACCENT.r, ACCENT.g, ACCENT.b, 0.25),
        );

        let col_pad = 14.0;
        draw_text_ex(
            "SERVER",
            lx + col_pad,
            hdr_y + 20.0,
            text_params(11, TEXT_DIM),
        );
        let players_label = "PLAYERS";
        let plw = measure_game_text(players_label, 11);
        draw_text_ex(
            players_label,
            lx + lw - plw - col_pad - 90.0,
            hdr_y + 20.0,
            text_params(11, TEXT_DIM),
        );
        let addr_label = "ADDRESS";
        let alw = measure_game_text(addr_label, 11);
        draw_text_ex(
            addr_label,
            lx + lw - alw - col_pad,
            hdr_y + 20.0,
            text_params(11, TEXT_DIM),
        );

        let ly = hdr_y + hdr_h;
        let lh = ch - (ly - cy) - 44.0;
        draw_rectangle(lx, ly, lw, lh, INPUT_BG);
        draw_rectangle_lines(lx, hdr_y, lw, hdr_h + lh, 1.0, INPUT_BORDER);

        if self.discovered_servers.is_empty() {
            let empty = if scanning {
                "Searching local network..."
            } else {
                "No servers on local network"
            };
            let ew = measure_game_text(empty, 14);
            draw_text_ex(
                empty,
                px + (pw - ew) / 2.0,
                ly + lh / 2.0 + 4.0,
                text_params(14, TEXT_DIM),
            );
        } else {
            let row_h = 52.0;
            let row_gap = 1.0;
            let mut ry = ly;
            let mut click_idx: Option<usize> = None;

            for (i, server) in self.discovered_servers.iter().enumerate() {
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

                let text_x = lx + col_pad + (if hover { 4.0 } else { 0.0 });
                draw_text_ex(
                    &server.name,
                    text_x,
                    ry + 22.0,
                    text_params(
                        16,
                        if hover {
                            WHITE
                        } else {
                            Color::new(0.88, 0.88, 0.95, 1.0)
                        },
                    ),
                );

                draw_circle(
                    text_x + measure_game_text(&server.name, 16) + 8.0,
                    ry + 18.0,
                    3.0,
                    Color::new(0.3, 0.9, 0.4, 0.8),
                );

                draw_text_ex(&server.addr, text_x, ry + 40.0, text_params(12, TEXT_DIM));

                let pc = format!("{}/{}", server.players, server.max_players);
                let pcw = measure_game_text(&pc, 15);
                let pc_color = if server.players == 0 {
                    TEXT_DIM
                } else if server.players >= server.max_players {
                    Color::new(1.0, 0.35, 0.35, 1.0)
                } else {
                    Color::new(0.3, 0.9, 0.5, 1.0)
                };
                draw_text_ex(
                    &pc,
                    lx + lw - pcw - col_pad - 90.0,
                    ry + 30.0,
                    text_params(15, pc_color),
                );

                let addr_short = &server.addr;
                let aw = measure_game_text(addr_short, 12);
                draw_text_ex(
                    addr_short,
                    lx + lw - aw - col_pad,
                    ry + 30.0,
                    text_params(12, TEXT_LABEL),
                );

                draw_rectangle(
                    lx + 6.0,
                    ry + row_h - row_gap,
                    lw - 12.0,
                    row_gap,
                    Color::new(0.16, 0.18, 0.28, 0.5),
                );

                if hover && clicked {
                    click_idx = Some(i);
                }
                ry += row_h;
            }

            if let Some(idx) = click_idx {
                self.ip_input = self.discovered_servers[idx].addr.clone();
                self.active_tab = MainMenuTab::Play;
            }
        }

        let bar_y = ly + lh + 8.0;

        let dots = ".".repeat((self.anim_time * 3.0) as usize % 4);
        let status = if scanning {
            format!("Scanning{}", dots)
        } else if self.discovered_servers.is_empty() {
            "No servers found".to_string()
        } else {
            format!("{} server(s)", self.discovered_servers.len())
        };
        draw_text_ex(&status, lx, bar_y + 14.0, text_params(13, TEXT_DIM));

        let rbw = 92.0;
        let rbh = 26.0;
        let rbx = lx + lw - rbw;
        let rby = bar_y;
        let rh = in_rect(mouse, rbx, rby, rbw, rbh);
        draw_rectangle(rbx, rby, rbw, rbh, if rh { ACCENT_HOVER } else { ACCENT });
        let rt = "REFRESH";
        let rtw = measure_game_text(rt, 13);
        draw_text_ex(
            rt,
            rbx + (rbw - rtw) / 2.0,
            rby + 18.0,
            text_params(13, Color::new(0.05, 0.05, 0.05, 1.0)),
        );
        if rh && clicked {
            self.start_scan();
        }
    }
}
