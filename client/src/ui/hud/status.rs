use macroquad::prelude::*;

use crate::physics::movement::MovementState;
use crate::ui::fonts::text_params;

pub(super) fn draw_status_panels(
    movement_state: &MovementState,
    hp: f32,
    max_hp: f32,
    shield: f32,
    max_shield: f32,
    match_time_left: f32,
    config: &crate::core::GameConfig,
) {
    // Match timer (top-center) — only display server-provided time when present.
    if match_time_left > 0.0 {
        let secs = match_time_left.ceil() as u32;
        let mins = secs / 60;
        let s = secs % 60;
        let time_text = format!("{:02}:{:02}", mins, s);
        let tw = crate::ui::fonts::measure_game_text(&time_text, 20);
        draw_text_ex(
            &time_text,
            (screen_width() - tw) / 2.0,
            32.0,
            text_params(20, Color::from_rgba(220, 220, 220, 220)),
        );
    }

    let hp_bar_x = 20.0;
    let hp_bar_y = screen_height() - 120.0;
    let hp_bar_w = 200.0;
    let hp_bar_h = 20.0;
    let hp_frac = (hp / max_hp).clamp(0.0, 1.0);

    draw_rectangle(
        hp_bar_x,
        hp_bar_y,
        hp_bar_w,
        hp_bar_h,
        Color::from_rgba(40, 40, 40, 200),
    );

    let hp_color = if hp_frac > 0.5 {
        Color::from_rgba(80, 200, 80, 255)
    } else if hp_frac > 0.25 {
        Color::from_rgba(220, 180, 40, 255)
    } else {
        RED
    };
    draw_rectangle(hp_bar_x, hp_bar_y, hp_bar_w * hp_frac, hp_bar_h, hp_color);

    draw_rectangle_lines(hp_bar_x, hp_bar_y, hp_bar_w, hp_bar_h, 2.0, WHITE);

    let hp_text = format!("{:.0} HP", hp);
    draw_text_ex(
        &hp_text,
        hp_bar_x + 5.0,
        hp_bar_y + 15.0,
        text_params(16, WHITE),
    );

    if max_shield > 0.0 {
        let shield_bar_y = hp_bar_y - 25.0;
        let shield_frac = (shield / max_shield).clamp(0.0, 1.0);

        draw_rectangle(
            hp_bar_x,
            shield_bar_y,
            hp_bar_w,
            hp_bar_h,
            Color::from_rgba(40, 40, 40, 200),
        );

        draw_rectangle(
            hp_bar_x,
            shield_bar_y,
            hp_bar_w * shield_frac,
            hp_bar_h,
            Color::from_rgba(80, 180, 255, 255),
        );

        draw_rectangle_lines(hp_bar_x, shield_bar_y, hp_bar_w, hp_bar_h, 2.0, WHITE);

        let shield_text = format!("{:.0} SHIELD", shield);
        draw_text_ex(
            &shield_text,
            hp_bar_x + 5.0,
            shield_bar_y + 15.0,
            text_params(16, WHITE),
        );
    }

    let bar_width = 200.0;
    let bar_height = 8.0;
    let bar_x = screen_width() - bar_width - 20.0;
    let bar_y = screen_height() - 40.0;
    draw_rectangle(
        bar_x,
        bar_y,
        bar_width,
        bar_height,
        Color::from_rgba(50, 50, 50, 255),
    );
    let fill = movement_state.stamina / 100.0;
    let bar_color = if movement_state.stamina < 20.0 {
        RED
    } else {
        Color::from_rgba(80, 180, 80, 255)
    };
    draw_rectangle(bar_x, bar_y, bar_width * fill, bar_height, bar_color);
    draw_rectangle_lines(bar_x, bar_y, bar_width, bar_height, 1.0, GRAY);

    if movement_state.is_crouching {
        draw_text_ex(
            "CROUCHING",
            20.0,
            screen_height() - 20.0,
            text_params(16, Color::from_rgba(150, 150, 150, 255)),
        );
    }
}
