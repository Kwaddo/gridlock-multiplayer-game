//! Drawing and in-between animation for the emote wheel: the ring grows and
//! fades in on open, the highlight glides between slices, and the picked slice
//! stays lit while the wheel fades back out.

use std::f32::consts::{FRAC_PI_2, TAU};

use macroquad::prelude::*;

use crate::types::player::PlayerSkin;
use crate::ui::draw_arc;
use crate::ui::fonts::{measure_game_text, text_params};

use super::constants::*;
use super::{EmoteWheel, SLOTS};

impl EmoteWheel {
    /// Step the open/close and hover transitions by `dt` seconds.
    pub(super) fn animate(&mut self, dt: f32) {
        let step = dt / OPEN_SECS;
        self.open_t = if self.open { self.open_t + step } else { self.open_t - step }.clamp(0.0, 1.0);
        let lit = self.lit_slot();
        let k = 1.0 - (-HOVER_SPEED * dt).exp(); // frame-rate independent ease
        for (i, h) in self.hover.iter_mut().enumerate() {
            let target = if lit == Some(i) { 1.0 } else { 0.0 };
            *h += (target - *h) * k;
        }
    }

    /// Animate and draw; `s` is the GUI scale. Draws nothing once fully closed.
    pub fn draw(&mut self, s: f32, dt: f32) {
        self.animate(dt);
        if self.open_t <= 0.0 {
            return;
        }
        let e = ease_out_cubic(self.open_t);
        let k = s * (CLOSED_SCALE + (1.0 - CLOSED_SCALE) * e);
        let centre = vec2(screen_width() / 2.0, screen_height() / 2.0);

        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), fade(OVERLAY, e));
        for i in 0..SLOTS {
            self.draw_slice(i, centre, k, e);
        }
        self.draw_pointer(centre, k, e);
        self.draw_hub(centre, k, e);
    }

    fn draw_slice(&self, i: usize, centre: Vec2, k: f32, e: f32) {
        let h = self.hover[i];
        let a = i as f32 * TAU / SLOTS as f32; // clockwise from up
        let dir = vec2(a.sin(), -a.cos());
        let c = centre + dir * HOVER_PUSH * h * k;
        let (start, end) = slice_span(a);
        let mid = (RING_INNER + RING_OUTER) / 2.0 * k;
        let thick = (RING_OUTER - RING_INNER) * k;

        draw_arc(c.x, c.y, mid, start, end, thick, ARC_SEGMENTS, fade(PANEL_BG, e));
        draw_arc(c.x, c.y, mid, start, end, thick, ARC_SEGMENTS, fade(ACCENT_DIM, e * h));
        draw_arc(c.x, c.y, RING_OUTER * k, start, end, 1.0, ARC_SEGMENTS, fade(PANEL_BORDER, e));
        draw_arc(c.x, c.y, RING_INNER * k, start, end, 1.0, ARC_SEGMENTS, fade(PANEL_BORDER, e));
        // accent rim on the hovered slice, the same orange edge the menus use
        draw_arc(c.x, c.y, RING_OUTER * k - 1.5, start, end, 3.0, ARC_SEGMENTS, fade(ACCENT, e * h));

        if self.playing as usize == i + 1 {
            let dot = c + dir * (RING_OUTER - 14.0) * k;
            draw_circle(dot.x, dot.y, 3.0 * k, fade(ACCENT, e));
        }

        let label = PlayerSkin::EMOTES[i].label();
        let size = font_size(LABEL_FONT, k);
        let w = measure_game_text(label, size);
        let p = c + dir * mid;
        let color = fade(mix(TEXT_LABEL, ACCENT, h), e);
        draw_text_ex(label, p.x - w / 2.0, p.y + size as f32 * 0.35, text_params(size, color));
    }

    /// Short accent arc on the inner edge, pointing where the cursor is.
    fn draw_pointer(&self, centre: Vec2, k: f32, e: f32) {
        let len = self.cursor.length();
        let strength = ((len - DEADZONE) / DEADZONE).clamp(0.0, 1.0);
        if strength <= 0.0 {
            return;
        }
        let a = self.cursor.x.atan2(-self.cursor.y);
        let theta = FRAC_PI_2 - a;
        let r = (RING_INNER - 6.0) * k;
        draw_arc(centre.x, centre.y, r, theta - 0.2, theta + 0.2, 3.0, 8, fade(ACCENT, e * strength));
    }

    /// Centre disc: a dim title and the name of what releasing will do.
    fn draw_hub(&self, centre: Vec2, k: f32, e: f32) {
        draw_circle(centre.x, centre.y, HUB_RADIUS * k, fade(PANEL_BG, e));
        draw_circle_lines(centre.x, centre.y, HUB_RADIUS * k, 1.0, fade(PANEL_BORDER, e));

        let title = "EMOTE";
        let ts = font_size(HUB_TITLE_FONT, k);
        let tw = measure_game_text(title, ts);
        draw_text_ex(title, centre.x - tw / 2.0, centre.y - 8.0 * k, text_params(ts, fade(TEXT_DIM, e)));

        let (name, color) = match self.lit_slot() {
            Some(i) => (PlayerSkin::EMOTES[i].label(), ACCENT),
            None => ("Cancel", TEXT_DIM),
        };
        let ns = font_size(HUB_NAME_FONT, k);
        let nw = measure_game_text(name, ns);
        draw_text_ex(name, centre.x - nw / 2.0, centre.y + 14.0 * k, text_params(ns, fade(color, e)));
    }
}

/// `draw_arc` angles (counter-clockwise from +x, y up) covering slice centred at `a`.
fn slice_span(a: f32) -> (f32, f32) {
    let half = TAU / SLOTS as f32 / 2.0 - SLICE_GAP / 2.0;
    let theta = FRAC_PI_2 - a;
    (theta - half, theta + half)
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

fn fade(c: Color, alpha: f32) -> Color {
    Color::new(c.r, c.g, c.b, c.a * alpha)
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

fn font_size(base: u16, k: f32) -> u16 {
    (base as f32 * k).round().max(1.0) as u16
}
