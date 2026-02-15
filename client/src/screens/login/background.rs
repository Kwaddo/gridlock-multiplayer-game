use super::LoginScreen;
use crate::screens::login::colors::*;
use crate::ui::fonts::{measure_game_text, text_params};
use macroquad::prelude::*;

impl LoginScreen {
    pub fn update_background(&mut self) {
        let dt = get_frame_time();
        self.anim_time += dt;
        let sw = screen_width();
        let sh = screen_height();
        for p in &mut self.particles {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            if p.x < 0.0 {
                p.x += sw;
            }
            if p.x > sw {
                p.x -= sw;
            }
            if p.y < 0.0 {
                p.y += sh;
            }
            if p.y > sh {
                p.y -= sh;
            }
        }
    }

    pub fn draw_background(&self) {
        clear_background(MENU_BG);

        for p in &self.particles {
            draw_circle(p.x, p.y, p.size, Color::new(0.50, 0.55, 0.80, p.alpha));
        }

        let sw = screen_width();
        let sh = screen_height();
        let mut y = 0.0;
        while y < sh {
            draw_line(0.0, y, sw, y, 1.0, Color::new(0.0, 0.0, 0.0, 0.06));
            y += 4.0;
        }
    }

    pub(super) fn draw_title(&self) {
        let sw = screen_width();
        let sh = screen_height();
        let title = "MULTIPLAYER FPS";
        let ts: u16 = 52;
        let tw = measure_game_text(title, ts);
        let tx = (sw - tw) / 2.0;
        let ty = sh * 0.15;

        draw_text_ex(
            title,
            tx + 2.0,
            ty + 2.0,
            text_params(ts, Color::new(0.0, 0.0, 0.0, 0.5)),
        );
        let g = ((self.anim_time * 0.8).sin() * 0.12 + 0.88).clamp(0.76, 1.0);
        draw_text_ex(title, tx, ty, text_params(ts, Color::new(g, g, g, 1.0)));

        let lw = tw * 0.55;
        let lx = (sw - lw) / 2.0;
        let ly = ty + 14.0;
        draw_rectangle(lx, ly, lw, 2.0, ACCENT);
    }
}
