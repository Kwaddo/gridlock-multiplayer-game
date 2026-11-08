use crate::net::protocols::MODE_GUN_GAME;
use macroquad::prelude::*;

use super::PlayingState;

use super::constants::{
    ACCENT, BLUE_BORDER, BLUE_PANEL, BLUE_TEAM, CONTROL_BG, PANEL_BORDER, RED_BORDER, RED_PANEL,
    RED_TEAM, ROSTER_H, TEXT_DIM, TEXT_LABEL,
};
use super::helpers::{
    bot_split, difficulty_summary, draw_wrapped, duration_text, mode_description, mode_name,
    mode_tips,
};

impl PlayingState {
    /// Humans in the lobby on (Red, Blue). The server keeps every lobby
    /// player on a team; anyone it has not placed yet counts for the
    /// smaller side, as it will place them.
    pub(super) fn lobby_humans(&self) -> (usize, usize) {
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
    pub(super) fn draw_rosters(
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
    pub(super) fn draw_guest_info(&mut self, x: f32, panel_w: f32, sep_y: f32) {
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
