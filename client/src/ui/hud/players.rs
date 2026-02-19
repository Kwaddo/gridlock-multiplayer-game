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
    connected_players: &[(u32, String, u32)],
    self_id: u32,
    self_kills: u32,
    in_game: bool,
) {
    use crate::ui::fonts::{measure_game_text, text_params};

    // build combined list (include local player if not present in connected_players)
    let mut rows: Vec<(String, u32, bool)> = Vec::new();
    let mut seen_self = false;
    for (id, name, kills) in connected_players.iter() {
        let is_self = *id == self_id;
        if is_self { seen_self = true; }
        rows.push((name.clone(), *kills, is_self));
    }
    if !seen_self {
        rows.push(("You".to_string(), self_kills, true));
    }

    if in_game {
        // sort by kills desc
        rows.sort_by(|a, b| b.1.cmp(&a.1));
    } else {
        rows.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    }

    // layout
    let row_h = 28.0;
    let w = 420.0;
    let h = (rows.len() as f32 + 1.0) * row_h + 28.0;
    let x = (screen_width() - w) / 2.0;
    let y = 120.0;

    draw_rectangle(x - 8.0, y - 12.0, w + 16.0, h + 24.0, Color::from_rgba(8, 10, 12, 220));
    draw_rectangle_lines(x - 8.0, y - 12.0, w + 16.0, h + 24.0, 2.0, Color::from_rgba(200, 200, 200, 60));

    // header
    let title = if in_game { "Scoreboard" } else { "Players online" };
    draw_text_ex(title, x + 12.0, y + 20.0, text_params(22, YELLOW));

    // rows
    let mut py = y + 50.0;
    for (i, (name, score, is_self)) in rows.iter().enumerate() {
        let score_text = format!("{}", score);
        let score_w = measure_game_text(&score_text, 18);

        let bg_alpha = if *is_self { 0.08 } else { 0.0 };
        if bg_alpha > 0.0 {
            draw_rectangle(x + 8.0, py - 18.0, w - 16.0, row_h - 4.0, Color::from_rgba(255, 255, 255, (bg_alpha * 255.0) as u8));
        }

        draw_text_ex(
            &format!("{}. {}", i + 1, name),
            x + 14.0,
            py,
            text_params(18, if *is_self { Color::from_rgba(180, 220, 255, 255) } else { WHITE }),
        );

        draw_text_ex(&score_text, x + w - 28.0 - score_w, py, text_params(18, WHITE));

        py += row_h;
    }
}
