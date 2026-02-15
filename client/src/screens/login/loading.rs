use super::LoginScreen;
use crate::screens::login::colors::*;
use crate::ui::fonts::{measure_game_text, text_params};
use macroquad::prelude::*;

impl LoginScreen {
    pub fn draw_loading(&self, total: usize, loaded: usize, current: &str, started: bool) {
        self.draw_background();
        self.draw_title();

        let sw = screen_width();
        let sh = screen_height();

        let pw = 480.0_f32.min(sw * 0.85);
        let ph = 150.0;
        let px = (sw - pw) / 2.0;
        let py = sh * 0.38;
        draw_rectangle(px, py, pw, ph, PANEL_BG);
        draw_rectangle_lines(px, py, pw, ph, 1.0, PANEL_BORDER);

        let label = if !started {
            "Receiving asset manifest..."
        } else if total == 0 {
            "Joining server..."
        } else {
            "Loading assets..."
        };
        let lw = measure_game_text(label, 20);
        draw_text_ex(label, (sw - lw) / 2.0, py + 34.0, text_params(20, WHITE));

        let pad = 30.0;
        let bx = px + pad;
        let bw = pw - pad * 2.0;
        let bh = 22.0;
        let by = py + 54.0;
        draw_rectangle(bx, by, bw, bh, INPUT_BG);
        draw_rectangle_lines(bx, by, bw, bh, 1.0, PANEL_BORDER);
        if total > 0 {
            let frac = loaded as f32 / total as f32;
            let fw = (bw - 4.0) * frac;
            draw_rectangle(bx + 2.0, by + 2.0, fw, bh - 4.0, ACCENT);
        } else if !started {
            let pulse_w = bw * 0.25;
            let offset = ((self.anim_time * 1.2).sin() * 0.5 + 0.5) * (bw - pulse_w);
            draw_rectangle(
                bx + 2.0 + offset,
                by + 2.0,
                pulse_w,
                bh - 4.0,
                Color::new(ACCENT.r, ACCENT.g, ACCENT.b, 0.45),
            );
        }

        if total > 0 {
            let count = format!("{} / {}", loaded, total);
            let cw = measure_game_text(&count, 15);
            draw_text_ex(
                &count,
                px + pw - pad - cw,
                by + bh + 18.0,
                text_params(15, TEXT_LABEL),
            );
        }

        if !current.is_empty() {
            draw_text_ex(current, bx, by + bh + 18.0, text_params(13, TEXT_DIM));
        }
    }
}
