//! The kill marker and the headshot marker, drawn over the crosshair's own
//! hit marker. A body hit keeps the plain white X; a headshot adds gold
//! brackets and a ring that pops; a kill swaps the X for a bigger red one
//! (with a gold ring when the killing shot was a headshot).

use macroquad::prelude::*;

use super::super::PlayingState;
use super::KILL_MARKER_SECS;
use crate::ui::prims::draw_circle_lines;

/// How long the crosshair's hit marker lasts (set in the fire path).
const HIT_MARKER_SECS: f32 = 0.35;
const GOLD: (u8, u8, u8) = (255, 205, 40);
const KILL_RED: (u8, u8, u8) = (235, 40, 40);
/// The kill marker pops in this big and settles over `POP_SECS`.
const POP: f32 = 0.5;
const POP_SECS: f32 = 0.08;

fn rgba((r, g, b): (u8, u8, u8), alpha: f32) -> Color {
    Color::from_rgba(r, g, b, (alpha.clamp(0.0, 1.0) * 255.0) as u8)
}

/// Four diagonal strokes around (cx, cy), from `inner` to `outer` out.
fn diagonals(cx: f32, cy: f32, inner: f32, outer: f32, thick: f32, color: Color) {
    for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        draw_line(cx + sx * inner, cy + sy * inner, cx + sx * outer, cy + sy * outer, thick, color);
    }
}

impl PlayingState {
    /// Half the diagonal of the crosshair's hit-marker X, worked out the way
    /// the crosshair does it, so the markers here line up with it.
    fn hit_marker_reach(&self) -> f32 {
        let weapon = self.weapons.active();
        let xhair = &weapon.crosshair;
        let sh = screen_height();
        let ads = weapon.ads_progress;
        let gap = (xhair.center_gap * sh / 100.0) * (1.0 - ads) + (xhair.center_gap * sh / 100.0) * 0.4 * ads;
        let len = xhair.line_length * sh / 100.0;
        let size = (gap + len) * 2.0;
        let size = if size < 10.0 { sh * 3.0 / 100.0 } else { size };
        size * 0.4
    }

    /// The kill marker, or the headshot extras on a hit marker, at the
    /// middle of the screen. Call with the HUD, where the crosshair shows.
    pub(in crate::screens::playing) fn draw_hit_fx_hud(&self) {
        let (cx, cy) = (screen_width() / 2.0, screen_height() / 2.0);
        let reach = self.hit_marker_reach();
        let fx = &self.hit_fx;

        if fx.kill_marker > 0.0 {
            let t = fx.kill_marker / KILL_MARKER_SECS;
            let age = KILL_MARKER_SECS - fx.kill_marker;
            let pop = 1.0 + POP * (1.0 - age / POP_SECS).max(0.0);
            // holds, then fades over the last 45%
            let alpha = (t / 0.45).min(1.0);
            let (inner, outer) = (reach * 0.25 * pop, reach * 1.35 * pop);
            diagonals(cx, cy, inner, outer, 6.5, Color::new(0.0, 0.0, 0.0, 0.55 * alpha));
            diagonals(cx, cy, inner, outer, 3.5, rgba(KILL_RED, alpha));
            if fx.kill_headshot {
                draw_circle_lines(cx, cy, outer * 1.2, 2.5, rgba(GOLD, alpha));
            }
            return;
        }

        if self.hit_marker_timer > 0.0 && self.hit_marker_headshot {
            let t = (self.hit_marker_timer / HIT_MARKER_SECS).min(1.0);
            let pop = 1.0 + 0.4 * t;
            // gold brackets past the tips of the gold X, and a ring that opens out
            diagonals(cx, cy, reach * 1.2 * pop, reach * 1.65 * pop, 3.0, rgba(GOLD, t));
            draw_circle_lines(cx, cy, reach * (2.2 - 0.6 * t), 1.5, rgba(GOLD, t * 0.8));
        }
    }
}
