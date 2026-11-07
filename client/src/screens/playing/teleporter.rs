use crate::net::protocols::{
    MATCH_MAX_PLAYERS, MATCH_STATUS_ENDING, MATCH_STATUS_RUNNING, MATCH_STATUS_STARTING,
    MAX_BOTS_PER_TEAM, MAX_FFA_BOTS, MODE_GUN_GAME, MODE_SEARCH_AND_DESTROY, MODE_TEAM_DEATHMATCH,
    balanced_bot_order,
};
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
const CONTROL_BG: Color = Color::new(0.12, 0.13, 0.22, 1.0);
const CONTROL_HOVER: Color = Color::new(0.18, 0.20, 0.32, 1.0);

/// Height of each team's roster panel in the lobby console.
const ROSTER_H: f32 = 118.0;

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

impl PlayingState {
    /// The host's side of the lobby console: mode, teams, time limit, bots.
    #[allow(clippy::too_many_arguments)]
    fn draw_host_setup(
        &mut self,
        config: &mut crate::core::GameConfig,
        panel_x: f32,
        panel_w: f32,
        sep_y: f32,
        player_id: u32,
        mouse: (f32, f32),
        clicked: bool,
    ) {
        use crate::ui::fonts::{measure_game_text, text_params};

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
                if *val == MODE_GUN_GAME {
                    // Default to 15 min when entering Gun Game so a time is always selected
                    if config.match_time_override_secs.is_none() {
                        config.match_time_override_secs = Some(900);
                    }
                } else {
                    // Clear Gun Game time override so it doesn't leak into other modes
                    config.match_time_override_secs = None;
                }
            }
        }

        draw_rectangle(tab_start_x, tab_y + tab_h, tab_total_w, 1.0, SEP);

        // ── mode description ──────────────────────────────────────────────────
        let desc_y = tab_y + tab_h + 14.0;
        draw_wrapped(
            mode_description(self.pending_start_gamemode),
            panel_x + 24.0,
            panel_w - 48.0,
            desc_y,
            13,
            TEXT_DIM,
            true,
        );

        // ── team roster (TDM / S&D only) ──────────────────────────────────────
        let roster_top = desc_y + 44.0;
        let gun_game = self.pending_start_gamemode == MODE_GUN_GAME;
        let (red_humans, blue_humans) = self.lobby_humans();
        let humans = red_humans + blue_humans;
        // With company, bots fill the smaller team so teams stay even (the
        // server does the same); one count sets them all. Alone, the host
        // picks each side.
        let solo = humans < 2;
        if !solo {
            self.autofill_blue_bots = self.autofill_red_bots;
        }
        let (red_bots, blue_bots) =
            bot_split(self.autofill_red_bots, self.autofill_blue_bots, red_humans, blue_humans);
        if !gun_game {
            self.draw_rosters(
                panel_x + 24.0,
                panel_w - 48.0,
                roster_top,
                (red_bots, blue_bots),
                Some((player_id, mouse, clicked)),
            );
        }

        // ── time limit selector (Gun Game) ────────────────────────────────────
        let section_x = panel_x + 24.0;
        let section_w = panel_w - 48.0;
        if gun_game {
            let tl_top = desc_y + 44.0;

            draw_text_ex(
                "TIME LIMIT",
                section_x,
                tl_top,
                text_params(12, TEXT_DIM),
            );

            let btn_y = tl_top + 8.0;
            let btn_h = 38.0;
            let durations: [(&str, u32); 4] = [
                ("10 MIN",    600),
                ("15 MIN",    900),
                ("30 MIN",    1800),
                ("UNLIMITED", u16::MAX as u32),
            ];
            let btn_w = section_w / durations.len() as f32;

            for (i, (label, secs)) in durations.iter().enumerate() {
                let bx = section_x + i as f32 * btn_w;
                let selected = config.match_time_override_secs == Some(*secs);
                let hovered = mouse.0 >= bx
                    && mouse.0 <= bx + btn_w
                    && mouse.1 >= btn_y
                    && mouse.1 <= btn_y + btn_h;

                let bg = if selected {
                    ACCENT_DIM
                } else if hovered {
                    Color::new(0.12, 0.14, 0.24, 1.0)
                } else {
                    Color::new(0.0, 0.0, 0.0, 0.0)
                };
                draw_rectangle(bx, btn_y, btn_w, btn_h, bg);
                if selected {
                    draw_rectangle(bx, btn_y + btn_h - 2.0, btn_w, 2.0, ACCENT);
                }

                let lc = if selected {
                    WHITE
                } else if hovered {
                    TEXT_LABEL
                } else {
                    Color::new(0.42, 0.44, 0.56, 1.0)
                };
                let lw = measure_game_text(label, 15);
                draw_text_ex(
                    label,
                    bx + (btn_w - lw) / 2.0,
                    btn_y + 26.0,
                    text_params(15, lc),
                );

                if hovered && clicked {
                    config.match_time_override_secs = Some(*secs);
                    config.hide_match_timer = false;
                }
            }

            draw_rectangle(section_x, btn_y + btn_h, section_w, 1.0, SEP);
        }

        // ── bots ──────────────────────────────────────────────────────────────
        // Team modes: a bot count per team, under that team's roster. Gun Game:
        // one total, as it has no teams. Zero means no bots.
        let bots_top = if gun_game {
            desc_y + 44.0 + 8.0 + 38.0 + 24.0
        } else {
            roster_top + ROSTER_H + 24.0
        };
        let bots_label = if !gun_game && !solo { "BOTS PER TEAM" } else { "BOTS" };
        draw_text_ex(bots_label, section_x, bots_top, text_params(12, TEXT_DIM));

        let bots = if gun_game { self.autofill_ffa_bots as usize } else { red_bots + blue_bots };
        // what the match will look like, on the label line
        let summary = if gun_game {
            format!("{} PLAYERS  +  {} BOTS", humans, bots)
        } else {
            format!("MATCH-UP  {}  v  {}", red_humans + red_bots, blue_humans + blue_bots)
        };
        let sum_w = measure_game_text(&summary, 12);
        draw_text_ex(
            &summary,
            section_x + section_w - sum_w,
            bots_top,
            text_params(12, TEXT_LABEL),
        );

        let row_y = bots_top + 8.0;
        let row_h = 34.0;
        if gun_game {
            bot_stepper(
                &mut self.autofill_ffa_bots,
                MAX_FFA_BOTS,
                section_x,
                row_y,
                section_w,
                row_h,
                ACCENT,
                PANEL_BORDER,
                mouse,
                clicked,
            );
        } else if !solo {
            bot_stepper(
                &mut self.autofill_red_bots,
                MAX_BOTS_PER_TEAM,
                section_x,
                row_y,
                section_w,
                row_h,
                ACCENT,
                PANEL_BORDER,
                mouse,
                clicked,
            );
            self.autofill_blue_bots = self.autofill_red_bots;
        } else {
            let gap = 12.0;
            let col_w = (section_w - gap) / 2.0;
            bot_stepper(
                &mut self.autofill_red_bots,
                MAX_BOTS_PER_TEAM,
                section_x,
                row_y,
                col_w,
                row_h,
                RED_TEAM,
                RED_BORDER,
                mouse,
                clicked,
            );
            bot_stepper(
                &mut self.autofill_blue_bots,
                MAX_BOTS_PER_TEAM,
                section_x + col_w + gap,
                row_y,
                col_w,
                row_h,
                BLUE_TEAM,
                BLUE_BORDER,
                mouse,
                clicked,
            );
        }

        // second row: difficulty filter on the left; quick presets on the right
        let opts_y = row_y + row_h + 12.0;
        let preset_h = 22.0;
        let mut presets_x = section_x + section_w;
        if gun_game {
            presets_x -= 64.0;
            if small_button("NONE", presets_x, opts_y, 64.0, preset_h, mouse, clicked) {
                self.autofill_ffa_bots = 0;
            }
        } else if !solo {
            presets_x -= 64.0;
            if small_button("NONE", presets_x, opts_y, 64.0, preset_h, mouse, clicked) {
                self.autofill_red_bots = 0;
                self.autofill_blue_bots = 0;
            }
        } else {
            presets_x -= 64.0;
            if small_button("NONE", presets_x, opts_y, 64.0, preset_h, mouse, clicked) {
                self.autofill_red_bots = 0;
                self.autofill_blue_bots = 0;
            }
            presets_x -= 112.0 + 8.0;
            if small_button("EVEN TEAMS", presets_x, opts_y, 112.0, preset_h, mouse, clicked) {
                // top up the smaller side so both teams field the same number
                let red = red_humans + self.autofill_red_bots as usize;
                let blue = blue_humans + self.autofill_blue_bots as usize;
                let target = red.max(blue).max(1);
                let cap = MAX_BOTS_PER_TEAM as usize;
                self.autofill_red_bots = target.saturating_sub(red_humans).min(cap) as u8;
                self.autofill_blue_bots = target.saturating_sub(blue_humans).min(cap) as u8;
            }
        }

        // a full match turns some bots away; say so before the host starts
        if humans + bots > MATCH_MAX_PLAYERS && !self.autofill_dropdown_open {
            let fit = MATCH_MAX_PLAYERS.saturating_sub(humans);
            let warn = format!("A match holds {} players: only {} bots will join.", MATCH_MAX_PLAYERS, fit);
            draw_text_ex(
                &warn,
                section_x,
                opts_y + preset_h + 22.0,
                text_params(12, Color::new(1.0, 0.65, 0.42, 1.0)),
            );
        }

        // drawn last: its dropdown opens over everything above
        self.draw_bot_difficulty(section_x, opts_y, presets_x - section_x - 12.0, mouse, clicked);
    }

    /// "BOT DIFFICULTY" filter: a summary of the enabled difficulties and a
    /// dropdown of checkboxes, `w` wide from (`x`, `y`).
    fn draw_bot_difficulty(&mut self, x: f32, y: f32, w: f32, mouse: (f32, f32), clicked: bool) {
        use crate::ui::fonts::{measure_game_text, text_params};

        let h = 22.0;
        let hov = mouse.0 >= x && mouse.0 <= x + w && mouse.1 >= y && mouse.1 <= y + h;
        draw_rectangle(x, y, w, h, if hov || self.autofill_dropdown_open { CONTROL_HOVER } else { CONTROL_BG });
        draw_rectangle_lines(x, y, w, h, 1.0, if self.autofill_dropdown_open { ACCENT } else { PANEL_BORDER });
        if hov && clicked {
            self.autofill_dropdown_open = !self.autofill_dropdown_open;
        }

        draw_text_ex("DIFFICULTY", x + 8.0, y + 15.0, text_params(12, TEXT_LABEL));

        let mut active: Vec<&str> = Vec::new();
        if self.autofill_diff_recruit { active.push("Rec"); }
        if self.autofill_diff_easy { active.push("Easy"); }
        if self.autofill_diff_normal { active.push("Norm"); }
        if self.autofill_diff_hard { active.push("Hard"); }
        if self.autofill_diff_nightmare { active.push("NM"); }
        let summary = if active.len() == 5 {
            "All".to_string()
        } else if active.is_empty() {
            "None!".to_string()
        } else {
            active.join(", ")
        };
        let arrow = if self.autofill_dropdown_open { "^" } else { "v" };
        let aw = measure_game_text(arrow, 12);
        draw_text_ex(arrow, x + w - aw - 8.0, y + 15.0, text_params(12, TEXT_LABEL));
        draw_text_ex(
            &summary,
            x + 92.0,
            y + 15.0,
            text_params(12, if active.is_empty() { Color::new(1.0, 0.3, 0.3, 1.0) } else { TEXT_DIM }),
        );

        if !self.autofill_dropdown_open {
            return;
        }

        // dropdown panel, drawn over whatever is below
        let panel_x = x;
        let panel_y = y + h + 2.0;
        let panel_w = w.max(300.0);
        let row_h = 24.0;
        let panel_h = row_h * 5.0 + 8.0;
        draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::new(0.08, 0.09, 0.16, 0.98));
        draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 1.0, PANEL_BORDER);

        let difficulties: [(&str, &str); 5] = [
            ("Recruit", "Slowest reaction, worst aim"),
            ("Easy", "Below average skill"),
            ("Normal", "Average players"),
            ("Hard", "Skilled opponents"),
            ("Nightmare", "Near-perfect aim & reactions"),
        ];
        let flags: [&mut bool; 5] = [
            &mut self.autofill_diff_recruit,
            &mut self.autofill_diff_easy,
            &mut self.autofill_diff_normal,
            &mut self.autofill_diff_hard,
            &mut self.autofill_diff_nightmare,
        ];
        for (i, ((name, desc), flag)) in difficulties.iter().zip(flags).enumerate() {
            let row_y = panel_y + 4.0 + i as f32 * row_h;
            let cb_x = panel_x + 8.0;
            let cb = 16.0;
            let row_hov = mouse.0 >= panel_x
                && mouse.0 <= panel_x + panel_w
                && mouse.1 >= row_y
                && mouse.1 <= row_y + row_h;
            if row_hov {
                draw_rectangle(panel_x + 1.0, row_y, panel_w - 2.0, row_h, Color::new(0.15, 0.17, 0.28, 0.8));
            }
            draw_rectangle(cb_x, row_y + 4.0, cb, cb, CONTROL_BG);
            draw_rectangle_lines(cb_x, row_y + 4.0, cb, cb, 1.0, if *flag { ACCENT } else { PANEL_BORDER });
            if *flag {
                draw_rectangle(cb_x + 3.0, row_y + 7.0, cb - 6.0, cb - 6.0, ACCENT);
            }
            draw_text_ex(name, cb_x + cb + 8.0, row_y + 16.0, text_params(13, if *flag { WHITE } else { TEXT_DIM }));
            draw_text_ex(desc, cb_x + cb + 90.0, row_y + 16.0, text_params(11, Color::new(0.45, 0.47, 0.58, 0.8)));
            if row_hov && clicked {
                *flag = !*flag;
            }
        }
    }
}

/// A `- N BOTS +` stepper filling `w` x `h`, clamped to `0..=max`. `accent`
/// colours the buttons and count; `border` frames it.
#[allow(clippy::too_many_arguments)]
fn bot_stepper(
    value: &mut u8,
    max: u8,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    accent: Color,
    border: Color,
    mouse: (f32, f32),
    clicked: bool,
) {
    use crate::ui::fonts::{measure_game_text, text_params};

    draw_rectangle(x, y, w, h, CONTROL_BG);
    draw_rectangle_lines(x, y, w, h, 1.0, border);

    let inside = |bx: f32| mouse.0 >= bx && mouse.0 <= bx + h && mouse.1 >= y && mouse.1 <= y + h;
    let button = |bx: f32, label: &str, enabled: bool| -> bool {
        let hov = enabled && inside(bx);
        let bg = if !enabled {
            Color::new(accent.r, accent.g, accent.b, 0.10)
        } else if hov {
            Color::new(accent.r, accent.g, accent.b, 0.45)
        } else {
            Color::new(accent.r, accent.g, accent.b, 0.22)
        };
        draw_rectangle(bx + 3.0, y + 3.0, h - 6.0, h - 6.0, bg);
        let lw = measure_game_text(label, 18);
        let lc = if enabled { WHITE } else { TEXT_DIM };
        draw_text_ex(label, bx + (h - lw) / 2.0, y + h / 2.0 + 6.0, text_params(18, lc));
        hov && clicked
    };
    if button(x, "-", *value > 0) {
        *value -= 1;
    }
    if button(x + w - h, "+", *value < max) {
        *value += 1;
    }

    let label = match *value {
        0 => "NO BOTS".to_string(),
        1 => "1 BOT".to_string(),
        n => format!("{} BOTS", n),
    };
    let lw = measure_game_text(&label, 15);
    let lc = if *value == 0 { TEXT_DIM } else { accent };
    draw_text_ex(&label, x + (w - lw) / 2.0, y + h / 2.0 + 5.0, text_params(15, lc));
}

/// A small outlined button; returns true when clicked.
fn small_button(label: &str, x: f32, y: f32, w: f32, h: f32, mouse: (f32, f32), clicked: bool) -> bool {
    use crate::ui::fonts::{measure_game_text, text_params};

    let hov = mouse.0 >= x && mouse.0 <= x + w && mouse.1 >= y && mouse.1 <= y + h;
    draw_rectangle(x, y, w, h, if hov { CONTROL_HOVER } else { CONTROL_BG });
    draw_rectangle_lines(x, y, w, h, 1.0, if hov { ACCENT } else { PANEL_BORDER });
    let lw = measure_game_text(label, 12);
    draw_text_ex(label, x + (w - lw) / 2.0, y + h / 2.0 + 4.0, text_params(12, if hov { WHITE } else { TEXT_LABEL }));
    hov && clicked
}

impl PlayingState {
    /// Humans in the lobby on (Red, Blue). The server keeps every lobby
    /// player on a team; anyone it has not placed yet counts for the
    /// smaller side, as it will place them.
    fn lobby_humans(&self) -> (usize, usize) {
        let (mut red, mut blue) = (0usize, 0usize);
        for (_id, _name, _k, _d, _gl, team, _wep, is_bot) in self.connected_players.iter() {
            if *is_bot {
                continue;
            }
            match *team {
                1 => red += 1,
                2 => blue += 1,
                _ if red <= blue => red += 1,
                _ => blue += 1,
            }
        }
        (red, blue)
    }

    /// Red and Blue rosters side by side, `w` wide from (`x`, `top`), with
    /// each side's bot count in its header. `switch` (the host only) adds a
    /// button to change sides, offered only when the teams stay even.
    fn draw_rosters(
        &mut self,
        x: f32,
        w: f32,
        top: f32,
        (red_bots, blue_bots): (usize, usize),
        switch: Option<(u32, (f32, f32), bool)>,
    ) {
        use crate::ui::fonts::{measure_game_text, text_params};

        let mut red: Vec<(u32, String)> = Vec::new();
        let mut blue: Vec<(u32, String)> = Vec::new();
        for (id, name, _k, _d, _gl, team, _wep, is_bot) in self.connected_players.iter() {
            if *is_bot {
                continue;
            }
            let side = match *team {
                1 => &mut red,
                2 => &mut blue,
                _ if red.len() <= blue.len() => &mut red,
                _ => &mut blue,
            };
            side.push((*id, name.clone()));
        }

        let label = "AUTO-ASSIGNED (BALANCED)";
        let lw = measure_game_text(label, 11);
        draw_text_ex(label, x + (w - lw) / 2.0, top - 4.0, text_params(11, TEXT_DIM));

        let gap = 12.0;
        let col_w = (w - gap) / 2.0;
        let leader = self.lobby_leader_id;
        let sides = [
            (x, "RED", &red, red_bots, RED_TEAM, RED_PANEL, RED_BORDER, Color::new(0.90, 0.70, 0.70, 1.0), 1u8),
            (x + col_w + gap, "BLUE", &blue, blue_bots, BLUE_TEAM, BLUE_PANEL, BLUE_BORDER, Color::new(0.70, 0.82, 0.95, 1.0), 2u8),
        ];
        let mut requested: Option<u8> = None;
        for (sx, name, members, bots, title_c, panel_c, border_c, name_c, team_id) in sides {
            draw_rectangle(sx, top, col_w, ROSTER_H, panel_c);
            draw_rectangle_lines(sx, top, col_w, ROSTER_H, 1.0, border_c);
            draw_rectangle(sx, top, col_w, 26.0, Color::new(title_c.r, title_c.g, title_c.b, 0.18));
            draw_rectangle(sx, top + 26.0, col_w, 1.0, border_c);
            let title = match bots {
                0 => format!("{} TEAM  ({})", name, members.len()),
                1 => format!("{} TEAM  ({} + 1 BOT)", name, members.len()),
                n => format!("{} TEAM  ({} + {} BOTS)", name, members.len(), n),
            };
            draw_text_ex(&title, sx + 10.0, top + 18.0, text_params(13, title_c));

            let rows_h = if switch.is_some() { ROSTER_H - 40.0 - 36.0 } else { ROSTER_H - 40.0 };
            let max_rows = (rows_h / 16.0).floor().max(1.0) as usize + 1;
            for (i, (id, player)) in members.iter().enumerate().take(max_rows) {
                let y = top + 42.0 + i as f32 * 16.0;
                draw_text_ex(player, sx + 10.0, y, text_params(13, name_c));
                if leader == Some(*id) {
                    let pw = measure_game_text(player, 13);
                    draw_text_ex("HOST", sx + 16.0 + pw, y, text_params(10, ACCENT));
                }
            }
            if members.len() > max_rows {
                let more = format!("+{} more", members.len() - max_rows);
                draw_text_ex(&more, sx + col_w - measure_game_text(&more, 11) - 10.0, top + 18.0, text_params(11, TEXT_DIM));
            }

            let Some((player_id, mouse, clicked)) = switch else { continue };
            let mine = members.iter().any(|(id, _)| *id == player_id);
            // moving across must leave the teams within one of each other
            let (r, b) = (red.len(), blue.len());
            let allowed = !mine
                && if team_id == 1 {
                    (r + 1).abs_diff(b.saturating_sub(1)) <= 1
                } else {
                    (b + 1).abs_diff(r.saturating_sub(1)) <= 1
                };
            let (bx, by, bw, bh) = (sx + 10.0, top + ROSTER_H - 36.0, col_w - 20.0, 28.0);
            let hov = allowed && mouse.0 >= bx && mouse.0 <= bx + bw && mouse.1 >= by && mouse.1 <= by + bh;
            let (label, size) = if mine {
                ("YOUR TEAM".to_string(), 13)
            } else if allowed {
                (format!("SWITCH TO {}", name), 14)
            } else {
                ("TEAMS WOULD BE UNEVEN".to_string(), 11)
            };
            let bg = if allowed {
                Color::new(title_c.r * 0.8, title_c.g * 0.8, title_c.b * 0.8, if hov { 1.0 } else { 0.85 })
            } else {
                Color::new(0.0, 0.0, 0.0, 0.25)
            };
            draw_rectangle(bx, by, bw, bh, bg);
            if !allowed {
                draw_rectangle_lines(bx, by, bw, bh, 1.0, border_c);
            }
            let lw = measure_game_text(&label, size);
            draw_text_ex(&label, bx + (bw - lw) / 2.0, by + 19.0, text_params(size, if allowed { WHITE } else { TEXT_DIM }));
            if hov && clicked {
                requested = Some(team_id);
            }
        }
        if let Some(team) = requested {
            self.pending_set_team = Some(team);
        }
    }

    /// What a player who is not the host sees: the match the host is setting
    /// up (mode, time limit, bots, teams) and how the mode plays. Nothing here
    /// can be changed; the host's choices arrive as `LobbySettings`.
    fn draw_guest_info(&mut self, x: f32, panel_w: f32, sep_y: f32) {
        use crate::ui::fonts::{measure_game_text, text_params};

        let x = x + 24.0;
        let w = panel_w - 48.0;
        let mut y = sep_y + 22.0;

        // who runs this lobby
        let host_name = self
            .lobby_leader_id
            .and_then(|id| self.connected_players.iter().find(|p| p.0 == id))
            .map(|p| p.1.clone())
            .unwrap_or_else(|| "the host".to_string());
        draw_text_ex("HOSTED BY", x, y, text_params(12, TEXT_DIM));
        draw_text_ex(&host_name, x + measure_game_text("HOSTED BY", 12) + 8.0, y, text_params(13, WHITE));
        let note = "Only the host can change the match";
        draw_text_ex(note, x + w - measure_game_text(note, 11), y, text_params(11, TEXT_DIM));

        // the mode
        y += 14.0;
        let card_h = 86.0;
        draw_rectangle(x, y, w, card_h, CONTROL_BG);
        draw_rectangle_lines(x, y, w, card_h, 1.0, PANEL_BORDER);
        draw_rectangle(x, y, 3.0, card_h, ACCENT);
        let settings = self.host_settings;
        let (title, desc) = match settings {
            Some(s) => (mode_name(s.mode), mode_description(s.mode)),
            None => ("WAITING FOR THE HOST", "The host is choosing what to play next."),
        };
        draw_text_ex(title, x + 18.0, y + 32.0, text_params(22, WHITE));
        draw_wrapped(desc, x + 18.0, w - 36.0, y + 56.0, 13, TEXT_DIM, false);

        // the details
        y += card_h + 14.0;
        let (red_humans, blue_humans) = self.lobby_humans();
        let (red_bots, blue_bots) = match settings {
            Some(s) if s.mode != MODE_GUN_GAME => {
                bot_split(s.red_bots, s.blue_bots, red_humans, blue_humans)
            }
            _ => (0, 0),
        };
        let bots = match settings {
            None => "-".to_string(),
            Some(s) if s.mode == MODE_GUN_GAME && s.red_bots == 0 => "None".to_string(),
            Some(s) if s.mode == MODE_GUN_GAME => s.red_bots.to_string(),
            Some(_) if red_bots + blue_bots == 0 => "None".to_string(),
            Some(_) => format!("{} RED / {} BLUE", red_bots, blue_bots),
        };
        let any_bots = bots != "None" && bots != "-";
        let details = [
            ("TIME LIMIT", settings.map_or("-".to_string(), |s| duration_text(s.mode, s.duration))),
            ("BOTS", bots),
            (
                "BOT DIFFICULTY",
                match settings {
                    Some(s) if any_bots => difficulty_summary(s.diff_mask),
                    _ => "-".to_string(),
                },
            ),
        ];
        let gap = 10.0;
        let box_w = (w - gap * 2.0) / 3.0;
        let box_h = 46.0;
        for (i, (label, value)) in details.iter().enumerate() {
            let bx = x + i as f32 * (box_w + gap);
            draw_rectangle(bx, y, box_w, box_h, CONTROL_BG);
            draw_rectangle_lines(bx, y, box_w, box_h, 1.0, PANEL_BORDER);
            draw_text_ex(label, bx + 10.0, y + 17.0, text_params(11, TEXT_DIM));
            draw_text_ex(value, bx + 10.0, y + 36.0, text_params(14, WHITE));
        }

        // who is playing
        y += box_h + 22.0;
        let gun_game = settings.map_or(false, |s| s.mode == MODE_GUN_GAME);
        if gun_game {
            draw_text_ex(
                &format!("PLAYERS  ({})", red_humans + blue_humans),
                x,
                y - 4.0,
                text_params(11, TEXT_DIM),
            );
            draw_rectangle(x, y, w, ROSTER_H, CONTROL_BG);
            draw_rectangle_lines(x, y, w, ROSTER_H, 1.0, PANEL_BORDER);
            let names: Vec<&(u32, String, u32, u32, u8, u8, String, bool)> =
                self.connected_players.iter().filter(|p| !p.7).collect();
            let per_col = ((ROSTER_H - 16.0) / 18.0) as usize;
            for (i, p) in names.iter().enumerate().take(per_col * 2) {
                let col = (i / per_col) as f32;
                let row = (i % per_col) as f32;
                draw_text_ex(&p.1, x + 12.0 + col * (w / 2.0), y + 22.0 + row * 18.0, text_params(13, TEXT_LABEL));
            }
        } else {
            self.draw_rosters(x, w, y, (red_bots, blue_bots), None);
        }

        // how the mode plays
        y += ROSTER_H + 24.0;
        if let Some(s) = settings {
            draw_text_ex("HOW IT PLAYS", x, y, text_params(12, TEXT_DIM));
            for (i, tip) in mode_tips(s.mode).iter().enumerate() {
                let ty = y + 20.0 + i as f32 * 18.0;
                draw_rectangle(x + 2.0, ty - 6.0, 4.0, 4.0, ACCENT);
                draw_text_ex(tip, x + 14.0, ty, text_params(12, TEXT_LABEL));
            }
        }
    }
}

/// Bots per side once the match starts: with two or more humans, the
/// `red + blue` bots fill the smaller team first, as the server places them
/// (`balanced_bot_order`); a player alone gets what was asked for.
fn bot_split(red: u8, blue: u8, red_humans: usize, blue_humans: usize) -> (usize, usize) {
    if red_humans + blue_humans < 2 {
        return (red as usize, blue as usize);
    }
    let order = balanced_bot_order(red_humans, blue_humans, (red + blue) as usize);
    let to_red = order.iter().filter(|r| **r).count();
    (to_red, order.len() - to_red)
}

fn mode_name(mode: u8) -> &'static str {
    match mode {
        MODE_GUN_GAME => "GUN GAME",
        MODE_TEAM_DEATHMATCH => "TEAM DEATHMATCH",
        MODE_SEARCH_AND_DESTROY => "SEARCH & DESTROY",
        _ => "UNKNOWN MODE",
    }
}

fn mode_description(mode: u8) -> &'static str {
    match mode {
        MODE_GUN_GAME => "Progress through every weapon — first to finish the arsenal wins.",
        MODE_TEAM_DEATHMATCH => "Two teams compete. First to 100 kills or end of 15 minutes wins.",
        MODE_SEARCH_AND_DESTROY => {
            "Attackers plant the bomb, defenders defuse it. No respawns — rounds decide the match."
        }
        _ => "",
    }
}

fn mode_tips(mode: u8) -> &'static [&'static str] {
    match mode {
        MODE_GUN_GAME => &[
            "Every kill moves you on to the next weapon.",
            "After the last gun, win it with a punch kill.",
            "Get punched and you drop back to the first weapon.",
        ],
        MODE_TEAM_DEATHMATCH => &[
            "Every kill scores for your team; first to 100 wins.",
            "You respawn a few seconds after dying.",
            "Teams are balanced for you before the match starts.",
        ],
        MODE_SEARCH_AND_DESTROY => &[
            "Attackers plant the bomb at the site; defenders stop or defuse it.",
            "One life per round: the dead wait for the next round.",
            "Win the most rounds to win the match.",
        ],
        _ => &[],
    }
}

/// A `StartGame` duration code as players read it.
fn duration_text(mode: u8, code: u16) -> String {
    match code {
        u16::MAX => "Unlimited".to_string(),
        0 if mode == MODE_SEARCH_AND_DESTROY => "Rounds".to_string(),
        0 => "15 MIN".to_string(),
        secs if secs % 60 == 0 => format!("{} MIN", secs / 60),
        secs => format!("{}:{:02}", secs / 60, secs % 60),
    }
}

/// The enabled bot difficulties (bit0 = Recruit .. bit4 = Nightmare).
fn difficulty_summary(mask: u8) -> String {
    let names = ["Rec", "Easy", "Norm", "Hard", "NM"];
    let on: Vec<&str> = names.iter().enumerate().filter(|(i, _)| mask >> i & 1 == 1).map(|(_, n)| *n).collect();
    match on.len() {
        5 => "All".to_string(),
        0 => "None".to_string(),
        _ => on.join(", "),
    }
}

/// Word-wrapped text `w` wide from `x`, first baseline at `y`; centred in
/// that width or left-aligned.
#[allow(clippy::too_many_arguments)]
fn draw_wrapped(text: &str, x: f32, w: f32, y: f32, size: u16, color: Color, centred: bool) {
    use crate::ui::fonts::{measure_game_text, text_params};

    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let test = if cur.is_empty() { word.to_string() } else { format!("{} {}", cur, word) };
        if !cur.is_empty() && measure_game_text(&test, size) > w {
            lines.push(std::mem::replace(&mut cur, word.to_string()));
        } else {
            cur = test;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    let line_h = size as f32 + 6.0;
    for (i, line) in lines.iter().enumerate() {
        let lx = if centred { x + (w - measure_game_text(line, size)) / 2.0 } else { x };
        draw_text_ex(line, lx, y + i as f32 * line_h, text_params(size, color));
    }
}
