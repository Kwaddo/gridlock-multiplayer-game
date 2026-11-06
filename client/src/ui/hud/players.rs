use macroquad::prelude::*;

/// Draws the in-world nametags (keeps existing behaviour) and the scoreboard/list is
/// rendered only when the caller explicitly invokes `draw_player_list`.

pub fn draw_player_list(
    connected_players: &[(u32, String, u32, u32, u8, u8, String, bool)], // (id, name, kills, deaths, gun_level, team, weapon, is_bot)
    self_id: u32,
    self_kills: u32,
    self_deaths: u32,
    self_gun_level: u8,
    in_game: bool,
    match_mode: Option<u8>,
) {
    use crate::ui::fonts::text_params;

    // (name, kills, deaths, gun_level, is_self, team, weapon)
    let mut rows: Vec<(String, u32, u32, u8, bool, u8, String)> = Vec::new();
    let mut seen_self = false;
    for (id, name, kills, deaths, gun_level, team, weapon, is_bot) in connected_players.iter() {
        let is_self = *id == self_id;
        if is_self {
            seen_self = true;
        }
        let display_name = if *is_bot {
            format!("[BOT] {}", name)
        } else {
            name.clone()
        };
        rows.push((
            display_name,
            *kills,
            *deaths,
            *gun_level,
            is_self,
            *team,
            weapon.clone(),
        ));
    }
    if !seen_self {
        rows.push((
            "You".to_string(),
            self_kills,
            self_deaths,
            self_gun_level,
            true,
            0u8,
            "".to_string(),
        ));
    }

    if in_game {
        rows.sort_by(|a, b| b.1.cmp(&a.1));
    } else {
        rows.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    }

    // layout — wider to fit K/D/A + weapon columns
    let row_h = 28.0;
    let w = 560.0;
    let h = (rows.len() as f32 + 1.0) * row_h + 28.0;
    let x = (screen_width() - w) / 2.0;
    let y = 120.0;

    let header_color = Color::from_rgba(160, 160, 170, 255);
    let dim_color = Color::from_rgba(120, 120, 130, 255);

    draw_rectangle(
        x - 8.0,
        y - 12.0,
        w + 16.0,
        h + 24.0,
        Color::from_rgba(8, 10, 12, 220),
    );
    draw_rectangle_lines(
        x - 8.0,
        y - 12.0,
        w + 16.0,
        h + 24.0,
        2.0,
        Color::from_rgba(200, 200, 200, 60),
    );

    // title
    let title = if in_game {
        "Scoreboard"
    } else {
        "Players online"
    };
    draw_text_ex(title, x + 12.0, y + 20.0, text_params(22, YELLOW));

    // column headers
    let col_name_x = x + 14.0;
    let col_k_x = x + w - 220.0;
    let col_d_x = x + w - 170.0;
    let col_a_x = x + w - 120.0;
    let col_wep_x = x + w - 60.0;

    let header_y = y + 42.0;
    draw_text_ex("K", col_k_x + 4.0, header_y, text_params(13, header_color));
    draw_text_ex("D", col_d_x + 4.0, header_y, text_params(13, header_color));
    draw_text_ex("A", col_a_x + 4.0, header_y, text_params(13, header_color));
    if in_game {
        draw_text_ex(
            "WPN",
            col_wep_x - 4.0,
            header_y,
            text_params(13, header_color),
        );
    }

    // rows
    let mut py = y + 62.0;
    for (i, (name, kills, deaths, gun_level, is_self, team, weapon)) in rows.iter().enumerate() {
        let bg_alpha = if *is_self { 0.08 } else { 0.0 };
        if bg_alpha > 0.0 {
            draw_rectangle(
                x + 8.0,
                py - 18.0,
                w - 16.0,
                row_h - 4.0,
                Color::from_rgba(255, 255, 255, (bg_alpha * 255.0) as u8),
            );
        }

        // Color player names by team when available (shades of red/blue).
        let is_team_mode = matches!(
            match_mode,
            Some(m) if m == crate::net::protocols::MODE_TEAM_DEATHMATCH
                    || m == crate::net::protocols::MODE_SEARCH_AND_DESTROY
        );
        let name_color = if is_team_mode {
            match *team {
                1 => {
                    if *is_self {
                        Color::from_rgba(255, 150, 150, 255) // brighter red for self
                    } else {
                        Color::from_rgba(230, 120, 120, 255)
                    }
                }
                2 => {
                    if *is_self {
                        Color::from_rgba(150, 200, 255, 255) // brighter blue for self
                    } else {
                        Color::from_rgba(120, 170, 230, 255)
                    }
                }
                _ => {
                    if *is_self {
                        Color::from_rgba(180, 220, 255, 255)
                    } else {
                        WHITE
                    }
                }
            }
        } else if *is_self {
            Color::from_rgba(180, 220, 255, 255)
        } else {
            WHITE
        };

        // team badge (show for team-based modes)
        let show_team = is_team_mode;
        if show_team {
            let badge_x = col_name_x - 28.0;
            let badge_w = 18.0;
            let badge_h = 18.0;
            let badge_color = match *team {
                1 => Color::from_rgba(200, 60, 60, 255),  // Red
                2 => Color::from_rgba(60, 120, 200, 255), // Blue
                _ => Color::from_rgba(120, 120, 120, 255),
            };
            draw_rectangle(badge_x, py - 16.0, badge_w, badge_h, badge_color);
            draw_rectangle_lines(
                badge_x,
                py - 16.0,
                badge_w,
                badge_h,
                1.0,
                Color::from_rgba(0, 0, 0, 100),
            );
        }

        // rank + name
        draw_text_ex(
            &format!("{}. {}", i + 1, name),
            col_name_x,
            py,
            text_params(18, name_color),
        );

        // K
        let k_text = format!("{}", kills);
        draw_text_ex(&k_text, col_k_x, py, text_params(18, WHITE));

        // D
        let d_text = format!("{}", deaths);
        draw_text_ex(&d_text, col_d_x, py, text_params(18, dim_color));

        // A (assists not tracked yet)
        draw_text_ex("0", col_a_x, py, text_params(18, dim_color));

        // weapon column: show weapon name when available (prefer weapon string), fall back to gun_level number
        if in_game {
            let wtext = if !weapon.is_empty() {
                weapon.clone()
            } else {
                format!("W{}", gun_level)
            };
            draw_text_ex(
                &wtext,
                col_wep_x - 40.0,
                py,
                text_params(14, Color::from_rgba(255, 200, 100, 255)),
            );
        }

        py += row_h;
    }
}

/// Team Deathmatch scoreboard: Red and Blue side by side, each headed by the
/// team's score and listing its players by kills.
/// `in_match`: ids of players in the match (lobby players are left out; empty = no filter).
pub fn draw_team_scoreboard(
    connected_players: &[(u32, String, u32, u32, u8, u8, String, bool)], // (id, name, kills, deaths, gun_level, team, weapon, is_bot)
    self_id: u32,
    in_match: &std::collections::HashSet<u32>,
    score: Option<crate::net::TeamScore>,
) {
    use crate::ui::fonts::{measure_game_text, text_params};

    let col_w = 300.0_f32.min((screen_width() - 48.0) / 2.0);
    let gap = 16.0;
    let total_w = col_w * 2.0 + gap;
    let x0 = (screen_width() - total_w) / 2.0;
    let y0 = 110.0;
    let header_h = 48.0;
    let row_h = 26.0;

    let mut teams: [Vec<&(u32, String, u32, u32, u8, u8, String, bool)>; 2] = [Vec::new(), Vec::new()];
    for p in connected_players {
        if !in_match.is_empty() && !in_match.contains(&p.0) {
            continue;
        }
        match p.5 {
            1 => teams[0].push(p),
            2 => teams[1].push(p),
            _ => {}
        }
    }
    for team in &mut teams {
        // most kills first, then fewest deaths
        team.sort_by(|a, b| b.2.cmp(&a.2).then(a.3.cmp(&b.3)).then(a.1.cmp(&b.1)));
    }
    let rows = teams[0].len().max(teams[1].len()).max(1);
    let body_h = 22.0 + rows as f32 * row_h + 8.0;

    let team_scores = [score.map(|s| s.red as u32), score.map(|s| s.blue as u32)];
    let looks = [
        ("RED", Color::from_rgba(230, 80, 75, 255), Color::from_rgba(70, 16, 16, 230)),
        ("BLUE", Color::from_rgba(80, 140, 240, 255), Color::from_rgba(14, 30, 78, 230)),
    ];
    // team names, lightened so they read on the team colour
    let label_colors = [
        Color::from_rgba(255, 160, 150, 255),
        Color::from_rgba(165, 200, 255, 255),
    ];
    let header_color = Color::from_rgba(150, 152, 165, 255);
    let dim = Color::from_rgba(150, 150, 160, 255);

    for (t, team) in teams.iter().enumerate() {
        let (label, accent, head_bg) = looks[t];
        let x = x0 + t as f32 * (col_w + gap);

        // header: name and team score (falls back to summed kills before the score arrives)
        draw_rectangle(x, y0, col_w, header_h, head_bg);
        draw_rectangle(x, y0 + header_h - 3.0, col_w, 3.0, accent);
        draw_text_ex(label, x + 14.0, y0 + 30.0, text_params(20, label_colors[t]));
        let team_score = team_scores[t].unwrap_or_else(|| team.iter().map(|p| p.2).sum());
        let st = team_score.to_string();
        let stw = measure_game_text(&st, 30);
        draw_text_ex(&st, x + col_w - 14.0 - stw, y0 + 34.0, text_params(30, WHITE));

        // body
        let by = y0 + header_h;
        draw_rectangle(x, by, col_w, body_h, Color::from_rgba(8, 10, 14, 225));
        let k_x = x + col_w - 74.0;
        let d_x = x + col_w - 38.0;
        draw_text_ex("PLAYER", x + 14.0, by + 17.0, text_params(11, header_color));
        draw_text_ex("K", k_x, by + 17.0, text_params(11, header_color));
        draw_text_ex("D", d_x, by + 17.0, text_params(11, header_color));

        let mut ry = by + 22.0;
        for (id, name, kills, deaths, _gl, _team, _weapon, is_bot) in team.iter().map(|p| &**p) {
            let is_self = *id == self_id;
            if is_self {
                draw_rectangle(x + 4.0, ry + 2.0, col_w - 8.0, row_h - 2.0, Color::from_rgba(255, 255, 255, 26));
                draw_rectangle(x + 4.0, ry + 2.0, 3.0, row_h - 2.0, accent);
            }
            let name_color = if is_self { WHITE } else { Color::from_rgba(215, 215, 222, 255) };
            let mut nx = x + 14.0;
            if *is_bot {
                draw_text_ex("BOT", nx, ry + 18.0, text_params(10, dim));
                nx += measure_game_text("BOT", 10) + 6.0;
            }
            // keep long names clear of the K/D columns
            let mut shown = name.clone();
            while measure_game_text(&shown, 16) > k_x - nx - 10.0 && shown.chars().count() > 3 {
                shown.pop();
            }
            if shown.len() < name.len() {
                shown.push_str("..");
            }
            draw_text_ex(&shown, nx, ry + 18.0, text_params(16, name_color));
            draw_text_ex(&kills.to_string(), k_x, ry + 18.0, text_params(16, WHITE));
            draw_text_ex(&deaths.to_string(), d_x, ry + 18.0, text_params(16, dim));
            ry += row_h;
        }
        if team.is_empty() {
            draw_text_ex("No players", x + 14.0, ry + 18.0, text_params(14, dim));
        }
    }
}
