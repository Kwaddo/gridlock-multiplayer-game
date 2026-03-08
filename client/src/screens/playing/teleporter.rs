use crate::net::protocols::{MODE_GUN_GAME, MODE_SEARCH_AND_DESTROY, MODE_TEAM_DEATHMATCH};
use crate::types::world::WorldId;
use macroquad::prelude::*;

use super::PlayingState;

// ── colour palette (matches login screen & settings) ──────────────────────────
const MENU_BG_OVL: Color = Color::new(0.0, 0.0, 0.0, 0.72);
const PANEL_BG: Color = Color::new(0.09, 0.11, 0.19, 0.97);
const PANEL_BORDER: Color = Color::new(0.20, 0.24, 0.40, 0.7);
const ACCENT: Color = Color::new(1.0, 0.42, 0.21, 1.0);
const ACCENT_HOVER: Color = Color::new(1.0, 0.55, 0.38, 1.0);
const ACCENT_DIM: Color = Color::new(1.0, 0.42, 0.21, 0.14);
const TEXT_DIM: Color = Color::new(0.50, 0.52, 0.62, 1.0);
const TEXT_LABEL: Color = Color::new(0.72, 0.74, 0.84, 1.0);
const SEP: Color = Color::new(0.20, 0.24, 0.40, 0.5);
const RED_TEAM: Color = Color::new(0.90, 0.40, 0.40, 1.0);
const BLUE_TEAM: Color = Color::new(0.42, 0.62, 0.95, 1.0);
const RED_PANEL: Color = Color::new(0.18, 0.07, 0.07, 0.95);
const BLUE_PANEL: Color = Color::new(0.07, 0.09, 0.22, 0.95);
const RED_BORDER: Color = Color::new(0.90, 0.40, 0.40, 0.35);
const BLUE_BORDER: Color = Color::new(0.42, 0.62, 0.95, 0.35);
const BTN_CLOSE_BG: Color = Color::new(0.12, 0.13, 0.22, 1.0);
const BTN_CLOSE_HOVER: Color = Color::new(0.18, 0.20, 0.32, 1.0);

impl PlayingState {
    /// Modal lobby console UI. Blocks gameplay input while open.
    pub(super) fn draw_lobby_console(&mut self, config: &mut crate::core::GameConfig) {
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

        // ── "SELECT MODE" label ───────────────────────────────────────────────
        let mode_label_y = sep_y + 18.0;
        draw_text_ex(
            "SELECT MODE",
            panel_x + 24.0,
            mode_label_y,
            text_params(12, TEXT_DIM),
        );

        // ── mode selector (tab-style, matching login tabs) ────────────────────
        let tab_y = mode_label_y + 6.0;
        let tab_h = 42.0;
        let modes: [(&str, u8); 3] = [
            ("GUN GAME", MODE_GUN_GAME),
            ("TEAM DEATHMATCH", MODE_TEAM_DEATHMATCH),
            ("SEARCH & DESTROY", MODE_SEARCH_AND_DESTROY),
        ];
        let tab_total_w = panel_w - 48.0;
        let tab_w = tab_total_w / modes.len() as f32;
        let tab_start_x = panel_x + 24.0;

        let mouse = mouse_position();
        let clicked = is_mouse_button_pressed(MouseButton::Left);

        for (i, (label, val)) in modes.iter().enumerate() {
            let tx = tab_start_x + i as f32 * tab_w;
            let selected = self.pending_start_gamemode == *val;
            let hovered = mouse.0 >= tx
                && mouse.0 <= tx + tab_w
                && mouse.1 >= tab_y
                && mouse.1 <= tab_y + tab_h;

            let bg = if selected {
                ACCENT_DIM
            } else if hovered {
                Color::new(0.12, 0.14, 0.24, 1.0)
            } else {
                Color::new(0.0, 0.0, 0.0, 0.0)
            };
            draw_rectangle(tx, tab_y, tab_w, tab_h, bg);

            // active bottom stripe
            if selected {
                draw_rectangle(tx, tab_y + tab_h - 2.0, tab_w, 2.0, ACCENT);
            }

            let lw = measure_game_text(label, 15);
            let lc = if selected {
                WHITE
            } else if hovered {
                TEXT_LABEL
            } else {
                Color::new(0.42, 0.44, 0.56, 1.0)
            };
            draw_text_ex(label, tx + (tab_w - lw) / 2.0, tab_y + 28.0, text_params(15, lc));

            if hovered && clicked {
                self.pending_start_gamemode = *val;
                if *val != MODE_GUN_GAME {
                    let mut red_count = 0usize;
                    let mut blue_count = 0usize;
                    for (_id, _name, _k, _d, _gl, team, _wep, _is_bot) in
                        self.connected_players.iter()
                    {
                        match *team {
                            1 => red_count += 1,
                            2 => blue_count += 1,
                            _ => {}
                        }
                    }
                    let chosen = if red_count <= blue_count { 1u8 } else { 2u8 };
                    self.pending_set_team = Some(chosen);
                    let msg = if chosen == 1 {
                        "Requested team: Red"
                    } else {
                        "Requested team: Blue"
                    };
                    self.lobby_console_message = Some(msg.to_string());
                    self.lobby_console_message_timer = 3.0;
                }
            }
        }

        draw_rectangle(tab_start_x, tab_y + tab_h, tab_total_w, 1.0, SEP);

        // ── mode description ──────────────────────────────────────────────────
        let desc_y = tab_y + tab_h + 14.0;
        let desc = match self.pending_start_gamemode {
            MODE_GUN_GAME => "Progress through every weapon — first to finish the arsenal wins.",
            MODE_TEAM_DEATHMATCH => {
                "Two teams compete. First to 100 kills or end of 15 minutes wins."
            }
            MODE_SEARCH_AND_DESTROY => {
                "Attackers plant the bomb, defenders defuse it. No respawns — rounds decide the match."
            }
            _ => "",
        };
        {
            let max_w = panel_w - 48.0;
            let font_size: u16 = 13;
            let line_h = font_size as f32 + 6.0;
            let mut lines: Vec<String> = Vec::new();
            let mut cur = String::new();
            for word in desc.split_whitespace() {
                if cur.is_empty() {
                    cur.push_str(word);
                    continue;
                }
                let test = format!("{} {}", cur, word);
                if measure_game_text(&test, font_size) > max_w {
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
                let lw = measure_game_text(line, font_size);
                draw_text_ex(
                    line,
                    panel_x + (panel_w - lw) / 2.0,
                    desc_y + i as f32 * line_h,
                    text_params(font_size, TEXT_DIM),
                );
            }
        }

        // ── team roster (TDM / S&D only) ──────────────────────────────────────
        let roster_top = desc_y + 44.0;

        if self.pending_start_gamemode != MODE_GUN_GAME {
            let mut preview_red: Vec<&(u32, String, u32, u32, u8, u8, String, bool)> = Vec::new();
            let mut preview_blue: Vec<&(u32, String, u32, u32, u8, u8, String, bool)> = Vec::new();
            for entry in self.connected_players.iter() {
                match entry.5 {
                    1 => preview_red.push(entry),
                    2 => preview_blue.push(entry),
                    _ => {
                        if preview_red.len() <= preview_blue.len() {
                            preview_red.push(entry);
                        } else {
                            preview_blue.push(entry);
                        }
                    }
                }
            }

            let gap = 12.0;
            let roster_w = (panel_w - 48.0 - gap) / 2.0;
            let roster_h = 118.0;
            let left_x = panel_x + 24.0;
            let right_x = left_x + roster_w + gap;

            let auto_label = "AUTO-ASSIGNED (BALANCED)";
            let alw = measure_game_text(auto_label, 11);
            draw_text_ex(
                auto_label,
                panel_x + (panel_w - alw) / 2.0,
                roster_top - 4.0,
                text_params(11, TEXT_DIM),
            );

            draw_rectangle(left_x, roster_top, roster_w, roster_h, RED_PANEL);
            draw_rectangle_lines(left_x, roster_top, roster_w, roster_h, 1.0, RED_BORDER);
            draw_rectangle(right_x, roster_top, roster_w, roster_h, BLUE_PANEL);
            draw_rectangle_lines(right_x, roster_top, roster_w, roster_h, 1.0, BLUE_BORDER);

            // team header stripes
            draw_rectangle(
                left_x,
                roster_top,
                roster_w,
                26.0,
                Color::new(0.90, 0.40, 0.40, 0.18),
            );
            draw_rectangle(
                right_x,
                roster_top,
                roster_w,
                26.0,
                Color::new(0.42, 0.62, 0.95, 0.18),
            );

            let red_title = format!("RED TEAM  ({})", preview_red.len());
            let blue_title = format!("BLUE TEAM  ({})", preview_blue.len());
            draw_text_ex(
                &red_title,
                left_x + 10.0,
                roster_top + 18.0,
                text_params(13, RED_TEAM),
            );
            draw_text_ex(
                &blue_title,
                right_x + 10.0,
                roster_top + 18.0,
                text_params(13, BLUE_TEAM),
            );

            draw_rectangle(left_x, roster_top + 26.0, roster_w, 1.0, RED_BORDER);
            draw_rectangle(right_x, roster_top + 26.0, roster_w, 1.0, BLUE_BORDER);

            let name_y0 = roster_top + 40.0;
            let line_h = 16.0;
            let max_rows = ((roster_h - 40.0) / line_h) as usize;
            for (i, (_id, name, kills, deaths, _gl, _team, _wep, _is_bot)) in
                preview_red.iter().enumerate().take(max_rows)
            {
                let display = format!("{}  {}/{}", name, kills, deaths);
                draw_text_ex(
                    &display,
                    left_x + 10.0,
                    name_y0 + i as f32 * line_h,
                    text_params(13, Color::new(0.90, 0.70, 0.70, 1.0)),
                );
            }
            for (i, (_id, name, kills, deaths, _gl, _team, _wep, _is_bot)) in
                preview_blue.iter().enumerate().take(max_rows)
            {
                let display = format!("{}  {}/{}", name, kills, deaths);
                draw_text_ex(
                    &display,
                    right_x + 10.0,
                    name_y0 + i as f32 * line_h,
                    text_params(13, Color::new(0.70, 0.82, 0.95, 1.0)),
                );
            }

            // join team buttons
            let jbtn_w = roster_w - 20.0;
            let jbtn_h = 28.0;
            let jbtn_y = roster_top + roster_h - jbtn_h - 8.0;

            let rhov = mouse.0 >= left_x + 10.0
                && mouse.0 <= left_x + 10.0 + jbtn_w
                && mouse.1 >= jbtn_y
                && mouse.1 <= jbtn_y + jbtn_h;
            draw_rectangle(
                left_x + 10.0,
                jbtn_y,
                jbtn_w,
                jbtn_h,
                if rhov {
                    Color::new(0.85, 0.32, 0.32, 1.0)
                } else {
                    Color::new(0.72, 0.22, 0.22, 1.0)
                },
            );
            let rjw = measure_game_text("JOIN RED", 14);
            draw_text_ex(
                "JOIN RED",
                left_x + 10.0 + (jbtn_w - rjw) / 2.0,
                jbtn_y + 20.0,
                text_params(14, WHITE),
            );
            if rhov && clicked {
                self.pending_set_team = Some(1u8);
                self.lobby_console_message = Some("Requested team: Red".to_string());
                self.lobby_console_message_timer = 3.0;
            }

            let bhov = mouse.0 >= right_x + 10.0
                && mouse.0 <= right_x + 10.0 + jbtn_w
                && mouse.1 >= jbtn_y
                && mouse.1 <= jbtn_y + jbtn_h;
            draw_rectangle(
                right_x + 10.0,
                jbtn_y,
                jbtn_w,
                jbtn_h,
                if bhov {
                    Color::new(0.32, 0.50, 0.88, 1.0)
                } else {
                    Color::new(0.22, 0.38, 0.72, 1.0)
                },
            );
            let bjw = measure_game_text("JOIN BLUE", 14);
            draw_text_ex(
                "JOIN BLUE",
                right_x + 10.0 + (jbtn_w - bjw) / 2.0,
                jbtn_y + 20.0,
                text_params(14, WHITE),
            );
            if bhov && clicked {
                self.pending_set_team = Some(2u8);
                self.lobby_console_message = Some("Requested team: Blue".to_string());
                self.lobby_console_message_timer = 3.0;
            }
        }

        // ── auto-fill bot settings ──────────────────────────────────────────────
        let autofill_top = if self.pending_start_gamemode != MODE_GUN_GAME {
            roster_top + 118.0 + 14.0
        } else {
            desc_y + 44.0
        };

        // Show auto-fill section for team-based modes only
        if self.pending_start_gamemode != MODE_GUN_GAME {
            let section_x = panel_x + 24.0;
            let section_w = panel_w - 48.0;

            // ── auto-fill checkbox ──
            let cb_y = autofill_top;
            let cb_size = 18.0;
            let cb_hov = mouse.0 >= section_x
                && mouse.0 <= section_x + cb_size
                && mouse.1 >= cb_y
                && mouse.1 <= cb_y + cb_size;

            // checkbox box
            draw_rectangle(section_x, cb_y, cb_size, cb_size,
                if cb_hov { Color::new(0.18, 0.20, 0.32, 1.0) } else { Color::new(0.12, 0.13, 0.22, 1.0) });
            draw_rectangle_lines(section_x, cb_y, cb_size, cb_size, 1.0,
                if self.autofill_enabled { ACCENT } else { PANEL_BORDER });

            // checkmark
            if self.autofill_enabled {
                let cx = section_x + 4.0;
                let cy = cb_y + 4.0;
                draw_rectangle(cx, cy, cb_size - 8.0, cb_size - 8.0, ACCENT);
            }

            if cb_hov && clicked {
                self.autofill_enabled = !self.autofill_enabled;
            }

            draw_text_ex("AUTO-FILL BOTS", section_x + cb_size + 8.0, cb_y + 14.0,
                text_params(14, if self.autofill_enabled { WHITE } else { TEXT_DIM }));

            // ── bots per team (only when enabled) ──
            if self.autofill_enabled {
                let bpt_y = cb_y + 28.0;

                draw_text_ex("BOTS PER TEAM", section_x, bpt_y + 14.0,
                    text_params(12, TEXT_LABEL));

                // minus button
                let minus_x = section_x + 120.0;
                let btn_sz = 22.0;
                let minus_hov = mouse.0 >= minus_x && mouse.0 <= minus_x + btn_sz
                    && mouse.1 >= bpt_y && mouse.1 <= bpt_y + btn_sz;
                draw_rectangle(minus_x, bpt_y, btn_sz, btn_sz,
                    if minus_hov { ACCENT_HOVER } else { ACCENT });
                let mw = measure_game_text("-", 16);
                draw_text_ex("-", minus_x + (btn_sz - mw) / 2.0, bpt_y + 17.0,
                    text_params(16, Color::new(0.05, 0.05, 0.05, 1.0)));
                if minus_hov && clicked && self.autofill_bots_per_team > 1 {
                    self.autofill_bots_per_team -= 1;
                }

                // number display
                let num_x = minus_x + btn_sz + 6.0;
                let num_w = 36.0;
                draw_rectangle(num_x, bpt_y, num_w, btn_sz,
                    Color::new(0.12, 0.13, 0.22, 1.0));
                draw_rectangle_lines(num_x, bpt_y, num_w, btn_sz, 1.0, PANEL_BORDER);
                let num_str = format!("{}", self.autofill_bots_per_team);
                let nw = measure_game_text(&num_str, 16);
                draw_text_ex(&num_str, num_x + (num_w - nw) / 2.0, bpt_y + 17.0,
                    text_params(16, WHITE));

                // plus button
                let plus_x = num_x + num_w + 6.0;
                let plus_hov = mouse.0 >= plus_x && mouse.0 <= plus_x + btn_sz
                    && mouse.1 >= bpt_y && mouse.1 <= bpt_y + btn_sz;
                draw_rectangle(plus_x, bpt_y, btn_sz, btn_sz,
                    if plus_hov { ACCENT_HOVER } else { ACCENT });
                let pw = measure_game_text("+", 16);
                draw_text_ex("+", plus_x + (btn_sz - pw) / 2.0, bpt_y + 17.0,
                    text_params(16, Color::new(0.05, 0.05, 0.05, 1.0)));
                if plus_hov && clicked && self.autofill_bots_per_team < 8 {
                    self.autofill_bots_per_team += 1;
                }

                // total bots label
                let total = self.autofill_bots_per_team * 2;
                let total_str = format!("({} total)", total);
                draw_text_ex(&total_str, plus_x + btn_sz + 10.0, bpt_y + 14.0,
                    text_params(12, TEXT_DIM));

                // ── difficulty filter dropdown ──
                let dd_y = bpt_y + 30.0;
                let arrow_label = "DIFFICULTY FILTER";
                draw_text_ex(arrow_label, section_x, dd_y + 14.0,
                    text_params(12, TEXT_LABEL));

                // dropdown arrow button
                let arrow_x = section_x + 140.0;
                let arrow_w = 24.0;
                let arrow_h = 20.0;
                let arrow_hov = mouse.0 >= arrow_x && mouse.0 <= arrow_x + arrow_w
                    && mouse.1 >= dd_y && mouse.1 <= dd_y + arrow_h;
                draw_rectangle(arrow_x, dd_y, arrow_w, arrow_h,
                    if arrow_hov { Color::new(0.18, 0.20, 0.32, 1.0) } else { Color::new(0.12, 0.13, 0.22, 1.0) });
                draw_rectangle_lines(arrow_x, dd_y, arrow_w, arrow_h, 1.0, PANEL_BORDER);
                let arrow_char = if self.autofill_dropdown_open { "^" } else { "v" };
                let aw = measure_game_text(arrow_char, 12);
                draw_text_ex(arrow_char, arrow_x + (arrow_w - aw) / 2.0, dd_y + 14.0,
                    text_params(12, TEXT_LABEL));

                if arrow_hov && clicked {
                    self.autofill_dropdown_open = !self.autofill_dropdown_open;
                }

                // active difficulties summary (inline)
                let mut active: Vec<&str> = Vec::new();
                if self.autofill_diff_recruit { active.push("Rec"); }
                if self.autofill_diff_easy { active.push("Easy"); }
                if self.autofill_diff_normal { active.push("Norm"); }
                if self.autofill_diff_hard { active.push("Hard"); }
                if self.autofill_diff_nightmare { active.push("NM"); }
                let summary = if active.len() == 5 { "All".to_string() } else if active.is_empty() { "None!".to_string() } else { active.join(", ") };
                draw_text_ex(&summary, arrow_x + arrow_w + 8.0, dd_y + 14.0,
                    text_params(12, if active.is_empty() { Color::new(1.0, 0.3, 0.3, 1.0) } else { TEXT_DIM }));

                // dropdown panel (drawn on top)
                if self.autofill_dropdown_open {
                    let dd_panel_x = arrow_x;
                    let dd_panel_y = dd_y + arrow_h + 2.0;
                    let dd_panel_w = section_w - (arrow_x - section_x);
                    let dd_row_h = 24.0;
                    let dd_panel_h = dd_row_h * 5.0 + 8.0;

                    draw_rectangle(dd_panel_x, dd_panel_y, dd_panel_w, dd_panel_h,
                        Color::new(0.08, 0.09, 0.16, 0.98));
                    draw_rectangle_lines(dd_panel_x, dd_panel_y, dd_panel_w, dd_panel_h, 1.0, PANEL_BORDER);

                    let difficulties: [(&str, &str); 5] = [
                        ("Recruit", "Slowest reaction, worst aim"),
                        ("Easy", "Below average skill"),
                        ("Normal", "Average players"),
                        ("Hard", "Skilled opponents"),
                        ("Nightmare", "Near-perfect aim & reactions"),
                    ];
                    let diff_flags: [bool; 5] = [
                        self.autofill_diff_recruit,
                        self.autofill_diff_easy,
                        self.autofill_diff_normal,
                        self.autofill_diff_hard,
                        self.autofill_diff_nightmare,
                    ];

                    for (di, ((diff_name, diff_desc), &enabled)) in
                        difficulties.iter().zip(diff_flags.iter()).enumerate()
                    {
                        let row_y = dd_panel_y + 4.0 + di as f32 * dd_row_h;
                        let dcb_x = dd_panel_x + 8.0;
                        let dcb_sz = 16.0;
                        let row_hov = mouse.0 >= dd_panel_x
                            && mouse.0 <= dd_panel_x + dd_panel_w
                            && mouse.1 >= row_y
                            && mouse.1 <= row_y + dd_row_h;

                        if row_hov {
                            draw_rectangle(dd_panel_x + 1.0, row_y, dd_panel_w - 2.0, dd_row_h,
                                Color::new(0.15, 0.17, 0.28, 0.8));
                        }

                        // checkbox
                        draw_rectangle(dcb_x, row_y + 4.0, dcb_sz, dcb_sz,
                            Color::new(0.12, 0.13, 0.22, 1.0));
                        draw_rectangle_lines(dcb_x, row_y + 4.0, dcb_sz, dcb_sz, 1.0,
                            if enabled { ACCENT } else { PANEL_BORDER });
                        if enabled {
                            draw_rectangle(dcb_x + 3.0, row_y + 7.0, dcb_sz - 6.0, dcb_sz - 6.0, ACCENT);
                        }

                        draw_text_ex(diff_name, dcb_x + dcb_sz + 8.0, row_y + 16.0,
                            text_params(13, if enabled { WHITE } else { TEXT_DIM }));
                        draw_text_ex(diff_desc, dcb_x + dcb_sz + 90.0, row_y + 16.0,
                            text_params(11, Color::new(0.45, 0.47, 0.58, 0.8)));

                        if row_hov && clicked {
                            match di {
                                0 => self.autofill_diff_recruit = !self.autofill_diff_recruit,
                                1 => self.autofill_diff_easy = !self.autofill_diff_easy,
                                2 => self.autofill_diff_normal = !self.autofill_diff_normal,
                                3 => self.autofill_diff_hard = !self.autofill_diff_hard,
                                4 => self.autofill_diff_nightmare = !self.autofill_diff_nightmare,
                                _ => {}
                            }
                        }
                    }
                }
            }
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

        // Start Match — orange accent (matches login CONNECT button)
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
        let start_label = match self.pending_start_gamemode {
            MODE_GUN_GAME => "START  GUN GAME",
            MODE_TEAM_DEATHMATCH => "START  TEAM DEATHMATCH",
            MODE_SEARCH_AND_DESTROY => "START  SEARCH & DESTROY",
            _ => "START MATCH",
        };
        let slw = measure_game_text(start_label, 17);
        draw_text_ex(
            start_label,
            btn_start_x + (start_btn_w - slw) / 2.0,
            btn_area_y + 27.0,
            text_params(17, Color::new(0.05, 0.05, 0.05, 1.0)),
        );
        if s_hov && clicked {
            self.pending_start_request = true;
            self.lobby_console_open = false;
            set_cursor_grab(true);
            show_mouse(false);
            self.last_mouse_pos = mouse_position();
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

        // suppress unused warning (time buttons removed — duration is server-side now)
        let _ = config;
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
