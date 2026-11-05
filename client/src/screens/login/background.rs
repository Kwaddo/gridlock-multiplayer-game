use crate::ui::fonts::{measure_game_text, text_params};

use macroquad::prelude::*;

use super::LoginScreen;
use super::{ACCENT, MENU_BG, PANEL_BG, PANEL_BORDER, TEXT_DIM, TEXT_LABEL};

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

    pub fn draw_title(&self) {
        let sw = screen_width();
        let sh = screen_height();
        let title = "GRIDLOCK";
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

    /// A quiet note in the corner while models download in the background.
    pub fn draw_preload(&self, done: u32, total: u32) {
        let text = format!("Downloading game assets {done} / {total}");
        let w = measure_game_text(&text, 13);
        draw_text_ex(&text, screen_width() - w - 16.0, screen_height() - 16.0, text_params(13, TEXT_DIM));
    }

    /// `download` is how far the asset being waited on has downloaded, as
    /// (received, total) bytes, when that is known.
    pub fn draw_loading(
        &self,
        total: usize,
        loaded: usize,
        current: &str,
        started: bool,
        download: Option<(f64, f64)>,
    ) {
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
        } else if download.is_some_and(|(received, size)| received < size) {
            "Downloading assets..."
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
        draw_rectangle(bx, by, bw, bh, super::INPUT_BG);
        draw_rectangle_lines(bx, by, bw, bh, 1.0, PANEL_BORDER);
        if total > 0 {
            // The asset in progress counts for the part already downloaded.
            let partial = download.map_or(0.0, |(received, size)| (received / size) as f32);
            let frac = ((loaded as f32 + partial.clamp(0.0, 1.0)) / total as f32).min(1.0);
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
            let line = match download {
                Some((received, size)) if received < size => format!(
                    "{}  {:.1} / {:.1} MB",
                    current,
                    received / 1_048_576.0,
                    size / 1_048_576.0
                ),
                _ => current.to_string(),
            };
            draw_text_ex(&line, bx, by + bh + 18.0, text_params(13, TEXT_DIM));
        }
    }
}
