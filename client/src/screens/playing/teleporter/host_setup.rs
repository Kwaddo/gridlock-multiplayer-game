use crate::net::protocols::{
    MATCH_MAX_PLAYERS, MAX_BOTS_PER_TEAM, MAX_FFA_BOTS, MODE_GUN_GAME, MODE_SEARCH_AND_DESTROY,
    MODE_TEAM_DEATHMATCH,
};
use macroquad::prelude::*;

use super::PlayingState;

use super::constants::{
    ACCENT, ACCENT_DIM, BLUE_BORDER, BLUE_TEAM, PANEL_BORDER, RED_BORDER, RED_TEAM, ROSTER_H, SEP,
    TEXT_DIM, TEXT_LABEL,
};
use super::controls::{bot_stepper, small_button};
use super::helpers::{bot_split, draw_wrapped, mode_description};

impl PlayingState {
    /// The host's side of the lobby console: mode, teams, time limit, bots.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_host_setup(
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
}
