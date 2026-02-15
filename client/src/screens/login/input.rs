use super::LoginScreen;
use crate::screens::login::colors::*;
use crate::ui::fonts::{measure_game_text, text_params};
use macroquad::prelude::*;

impl LoginScreen {
    pub(super) fn draw_input(
        &self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        text: &str,
        active: bool,
        placeholder: &str,
    ) {
        draw_rectangle(x, y, w, h, INPUT_BG);
        let bc = if active { ACCENT } else { INPUT_BORDER };
        draw_rectangle_lines(x, y, w, h, if active { 2.0 } else { 1.0 }, bc);

        if text.is_empty() && !active {
            draw_text_ex(
                placeholder,
                x + 10.0,
                y + 23.0,
                text_params(17, Color::new(0.30, 0.32, 0.42, 0.5)),
            );
        } else {
            draw_text_ex(text, x + 10.0, y + 23.0, text_params(17, WHITE));
            if active {
                let visible = (self.anim_time * 2.5) as i32 % 2 == 0;
                if visible {
                    let tw = measure_game_text(text, 17);
                    draw_rectangle(x + 10.0 + tw + 2.0, y + 6.0, 2.0, h - 12.0, ACCENT);
                }
            }
        }
    }
}
