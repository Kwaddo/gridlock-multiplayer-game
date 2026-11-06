//! Team Deathmatch score bar, top centre: RED score | match clock | score BLUE,
//! each with a bar filling toward the score that wins.

use macroquad::prelude::*;

use crate::net::TeamScore;
use crate::ui::fonts::{measure_game_text, text_params};

const TOP: f32 = 10.0;
const HEIGHT: f32 = 42.0;
const CLOCK_W: f32 = 96.0;
const GAP: f32 = 4.0;
const PROGRESS_H: f32 = 4.0;

const RED: Color = Color::new(0.90, 0.30, 0.28, 1.0);
const RED_BG: Color = Color::new(0.30, 0.06, 0.06, 0.78);
const BLUE: Color = Color::new(0.30, 0.55, 0.95, 1.0);
const BLUE_BG: Color = Color::new(0.05, 0.12, 0.32, 0.78);
const CLOCK_BG: Color = Color::new(0.03, 0.04, 0.06, 0.78);
const TRACK: Color = Color::new(1.0, 1.0, 1.0, 0.12);

/// `local_team`: 1 = red, 2 = blue (anything else: watching).
/// `time_left`: seconds, `None` to show no clock (hidden by the player, or not sent yet).
pub fn draw_team_score_bar(score: TeamScore, local_team: u8, time_left: Option<f32>) {
    let sw = screen_width();
    let panel_w = (sw * 0.12).clamp(110.0, 160.0);
    let clock_x = (sw - CLOCK_W) / 2.0;
    let red_x = clock_x - GAP - panel_w;
    let blue_x = clock_x + CLOCK_W + GAP;
    let to_win = score.to_win.max(1);

    // clock
    draw_rectangle(clock_x, TOP, CLOCK_W, HEIGHT, CLOCK_BG);
    let clock = match time_left {
        Some(t) if t.ceil() as u32 >= u16::MAX as u32 => "∞".to_string(), // infinite match
        Some(t) => {
            let secs = t.max(0.0).ceil() as u32;
            format!("{}:{:02}", secs / 60, secs % 60)
        }
        None => "TDM".to_string(),
    };
    let urgent = matches!(time_left, Some(t) if t <= 30.0);
    let clock_color = if urgent { Color::new(1.0, 0.75, 0.35, 1.0) } else { WHITE };
    let cw = measure_game_text(&clock, 22);
    draw_text_ex(&clock, clock_x + (CLOCK_W - cw) / 2.0, TOP + 25.0, text_params(22, clock_color));
    let goal = format!("FIRST TO {}", to_win);
    let gw = measure_game_text(&goal, 10);
    draw_text_ex(
        &goal,
        clock_x + (CLOCK_W - gw) / 2.0,
        TOP + 38.0,
        text_params(10, Color::new(0.75, 0.77, 0.82, 0.9)),
    );

    let leader = if score.red > score.blue { 1 } else if score.blue > score.red { 2 } else { 0 };
    draw_team_panel(red_x, panel_w, "RED", score.red, to_win, RED, RED_BG, true, local_team == 1, leader == 1);
    draw_team_panel(blue_x, panel_w, "BLUE", score.blue, to_win, BLUE, BLUE_BG, false, local_team == 2, leader == 2);
}

/// One team's half. `inner_right`: the panel sits left of the clock, so its
/// score and progress grow toward the centre.
#[allow(clippy::too_many_arguments)]
fn draw_team_panel(
    x: f32,
    w: f32,
    label: &str,
    score: u16,
    to_win: u16,
    accent: Color,
    bg: Color,
    inner_right: bool,
    ours: bool,
    leading: bool,
) {
    draw_rectangle(x, TOP, w, HEIGHT, bg);
    // a strip of team colour on the outer edge
    let strip_x = if inner_right { x } else { x + w - 4.0 };
    draw_rectangle(strip_x, TOP, 4.0, HEIGHT, accent);
    if ours {
        draw_rectangle_lines(x, TOP, w, HEIGHT, 2.0, Color::new(1.0, 1.0, 1.0, 0.85));
    }

    // team name (and YOU under it) on the outer side, score on the inner side
    // lightened so it reads on the team's own colour
    let name_color = Color::new(
        accent.r + (1.0 - accent.r) * 0.45,
        accent.g + (1.0 - accent.g) * 0.45,
        accent.b + (1.0 - accent.b) * 0.45,
        1.0,
    );
    let outer = |text_w: f32| if inner_right { x + 12.0 } else { x + w - 12.0 - text_w };
    let name_y = if ours { TOP + 20.0 } else { TOP + 26.0 };
    draw_text_ex(label, outer(measure_game_text(label, 13)), name_y, text_params(13, name_color));
    if ours {
        let you = "YOU";
        draw_text_ex(you, outer(measure_game_text(you, 10)), TOP + 34.0, text_params(10, WHITE));
    }

    let score_text = score.to_string();
    let score_color = if leading { WHITE } else { Color::new(0.85, 0.85, 0.88, 0.85) };
    let sw = measure_game_text(&score_text, 30);
    let score_x = if inner_right { x + w - 12.0 - sw } else { x + 12.0 };
    draw_text_ex(&score_text, score_x, TOP + 31.0, text_params(30, score_color));

    // progress toward the winning score, filling from the clock outward
    let bar_y = TOP + HEIGHT + 3.0;
    draw_rectangle(x, bar_y, w, PROGRESS_H, TRACK);
    let fill = (score as f32 / to_win as f32).clamp(0.0, 1.0) * w;
    let fill_x = if inner_right { x + w - fill } else { x };
    draw_rectangle(fill_x, bar_y, fill, PROGRESS_H, accent);
}
