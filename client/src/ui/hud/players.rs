use macroquad::prelude::*;

use crate::ui::fonts::{measure_game_text, text_params};
use crate::ui::OtherPlayer;

pub(super) fn draw_players_overlay(
    position: Vec3,
    forward: Vec3,
    right: Vec3,
    pitch: f32,
    other_players: &[OtherPlayer],
) {
    for p in other_players {
        let head_pos = p.render_position() + vec3(0.0, 1.8, 0.0);
        let rel = head_pos - position;
        let dot = rel.dot(forward);
        if dot > 0.1 {
            let right_dot = rel.dot(right);
            let up_dot = rel.y - position.y + pitch.sin() * dot;
            let fov_scale = screen_height() / (2.0 * ((45.0_f32).to_radians() / 2.0).tan());
            let screen_x = screen_width() / 2.0 + (right_dot / dot) * fov_scale;
            let screen_y = screen_height() / 2.0 - (up_dot / dot) * fov_scale;
            let name_w = measure_game_text(&p.username, 16);
            draw_text_ex(
                &p.username,
                screen_x - name_w / 2.0,
                screen_y,
                text_params(16, p.color),
            );
        }
    }

}

/// Draws the in-world nametags (keeps existing behaviour) and the scoreboard/list is
/// rendered only when the caller explicitly invokes `draw_player_list`.

pub fn draw_player_list(
    connected_players: &[(u32, String, u32, u32, u8, u8, String)], // (id, name, kills, deaths, gun_level, team, weapon)
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
    for (id, name, kills, deaths, gun_level, team, weapon) in connected_players.iter() {
        let is_self = *id == self_id;
        if is_self { seen_self = true; }
        rows.push((name.clone(), *kills, *deaths, *gun_level, is_self, *team, weapon.clone()));
    }
    if !seen_self {
        rows.push(("You".to_string(), self_kills, self_deaths, self_gun_level, true, 0u8, "".to_string()));
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

    draw_rectangle(x - 8.0, y - 12.0, w + 16.0, h + 24.0, Color::from_rgba(8, 10, 12, 220));
    draw_rectangle_lines(x - 8.0, y - 12.0, w + 16.0, h + 24.0, 2.0, Color::from_rgba(200, 200, 200, 60));

    // title
    let title = if in_game { "Scoreboard" } else { "Players online" };
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
        draw_text_ex("WPN", col_wep_x - 4.0, header_y, text_params(13, header_color));
    }

    // rows
    let mut py = y + 62.0;
    for (i, (name, kills, deaths, gun_level, is_self, team, weapon)) in rows.iter().enumerate() {
        let bg_alpha = if *is_self { 0.08 } else { 0.0 };
        if bg_alpha > 0.0 {
            draw_rectangle(x + 8.0, py - 18.0, w - 16.0, row_h - 4.0, Color::from_rgba(255, 255, 255, (bg_alpha * 255.0) as u8));
        }

        let name_color = if *is_self { Color::from_rgba(180, 220, 255, 255) } else { WHITE };

        // team badge (only show for Team Deathmatch)
        let show_team = match match_mode {
            Some(m) => m == crate::net::protocols::MODE_TEAM_DEATHMATCH,
            None => false,
        };
        if show_team {
            let badge_x = col_name_x - 28.0;
            let badge_w = 18.0;
            let badge_h = 18.0;
            let badge_color = match *team {
                1 => Color::from_rgba(200, 60, 60, 255), // Red
                2 => Color::from_rgba(60, 120, 200, 255), // Blue
                _ => Color::from_rgba(120, 120, 120, 255),
            };
            draw_rectangle(badge_x, py - 16.0, badge_w, badge_h, badge_color);
            draw_rectangle_lines(badge_x, py - 16.0, badge_w, badge_h, 1.0, Color::from_rgba(0,0,0,100));
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
            let wtext = if !weapon.is_empty() { weapon.clone() } else { format!("W{}", gun_level) };
            draw_text_ex(&wtext, col_wep_x - 40.0, py, text_params(14, Color::from_rgba(255, 200, 100, 255)));
        }

        py += row_h;
    }
}
