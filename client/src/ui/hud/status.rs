use macroquad::prelude::*;

use crate::physics::movement::MovementState;
use crate::ui::fonts::text_params;

pub(super) fn draw_status_panels(
    movement_state: &MovementState,
    hp: f32,
    max_hp: f32,
    match_time_left: f32,
    config: &crate::core::GameConfig,
) {
    // Match timer (top-center) — only display when not hidden by user and
    // when the server has sent a time.  infinite matches show as “∞”.
    if !config.hide_match_timer && match_time_left > 0.0 {
        let secs = match_time_left.ceil() as u32;
        let time_text = if secs == u16::MAX as u32 {
            // sentinel value used by server to indicate “infinite” duration
            "∞".to_string()
        } else {
            let mins = secs / 60;
            let s = secs % 60;
            format!("{:02}:{:02}", mins, s)
        };
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

    let bar_width = 200.0;
    let bar_height = 8.0;
    let bar_x = hp_bar_x;
    let bar_y = hp_bar_y + hp_bar_h + 8.0;
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

    let mut status_labels: Vec<&str> = Vec::new();
    if movement_state.is_sliding {
        status_labels.push("SLIDING");
    }
    if movement_state.is_slow_walking {
        status_labels.push("SLOW WALK");
    }
    if movement_state.is_prone {
        status_labels.push("PRONE");
    }
    if movement_state.is_crouching {
        status_labels.push("CROUCHING");
    }

    if !status_labels.is_empty() {
        let status_text = status_labels.join(" | ");
        draw_text_ex(
            &status_text,
            20.0,
            screen_height() - 20.0,
            text_params(16, Color::from_rgba(150, 150, 150, 255)),
        );
    }
}
