//! "?" / "!" over a bot that has just noticed you (issue #62). The server
//! sends `BotAlert` to the spotted player alone: "?" when the bot gets
//! suspicious, "!" when it has spotted you and fights. The mark pops in,
//! holds about a second and fades. A bot off screen gets a small marker on
//! the screen's edge in its direction instead, aimed with the damage
//! indicator's angle maths.

use macroquad::prelude::*;

use super::super::PlayingState;
use super::damage_dir::screen_angle;
use crate::net::protocols::BOT_ALERT_ENGAGED;

/// Growing in, with a little overshoot.
const POP_SECS: f32 = 0.12;
/// Settling back from the overshoot.
const SETTLE_SECS: f32 = 0.1;
const POP_OVERSHOOT: f32 = 1.3;
/// Held at full strength (after the pop)...
const HOLD_SECS: f32 = 1.0;
/// ...then fading out.
const FADE_SECS: f32 = 0.5;
const LIFE_SECS: f32 = POP_SECS + HOLD_SECS + FADE_SECS;
/// At most this many marks at once (the oldest gives way).
const MAX_ALERTS: usize = 8;
/// Above the bot's feet: over its nametag (1.7).
const MARK_HEIGHT: f32 = 2.2;
/// Mark size in pixels at full scale, over a head and on the edge.
const HEAD_SIZE: f32 = 40.0;
const EDGE_SIZE: f32 = 28.0;
/// The edge marker keeps this far in from the screen's edge.
const EDGE_MARGIN: f32 = 40.0;
const SUSPICIOUS_COLOR: (u8, u8, u8) = (255, 205, 60);
const ENGAGED_COLOR: (u8, u8, u8) = (240, 50, 40);

/// One bot's mark.
pub(crate) struct BotAlert {
    bot_id: u32,
    engaged: bool,
    age: f32,
}

/// Scale and opacity of a mark `age` seconds old.
fn alert_look(age: f32) -> (f32, f32) {
    let scale = if age < POP_SECS {
        POP_OVERSHOOT * (age / POP_SECS).max(0.0)
    } else if age < POP_SECS + SETTLE_SECS {
        POP_OVERSHOOT + (1.0 - POP_OVERSHOOT) * (age - POP_SECS) / SETTLE_SECS
    } else {
        1.0
    };
    let alpha = ((LIFE_SECS - age) / FADE_SECS).clamp(0.0, 1.0);
    (scale, alpha)
}

/// Where a marker at `angle` (0 = straight ahead, the top of the screen,
/// clockwise) sits on a `w` x `h` screen, `margin` in from its edge.
fn edge_point(w: f32, h: f32, margin: f32, angle: f32) -> Vec2 {
    let center = vec2(w * 0.5, h * 0.5);
    let half = vec2((w * 0.5 - margin).max(1.0), (h * 0.5 - margin).max(1.0));
    let dir = vec2(angle.sin(), -angle.cos());
    let tx = if dir.x.abs() > 1e-6 { half.x / dir.x.abs() } else { f32::INFINITY };
    let ty = if dir.y.abs() > 1e-6 { half.y / dir.y.abs() } else { f32::INFINITY };
    center + dir * tx.min(ty)
}

impl PlayingState {
    /// The server says bot `bot_id` noticed us (`BOT_ALERT_*`): show it. A
    /// newer mark for the same bot replaces its old one.
    pub fn note_bot_alert(&mut self, bot_id: u32, level: u8) {
        let engaged = level >= BOT_ALERT_ENGAGED;
        if let Some(a) = self.bot_alerts.iter_mut().find(|a| a.bot_id == bot_id) {
            a.engaged = engaged;
            a.age = 0.0;
            return;
        }
        if self.bot_alerts.len() >= MAX_ALERTS {
            if let Some(oldest) = self.bot_alerts.iter().enumerate().max_by(|a, b| a.1.age.total_cmp(&b.1.age)) {
                let i = oldest.0;
                self.bot_alerts.swap_remove(i);
            }
        }
        self.bot_alerts.push(BotAlert { bot_id, engaged, age: 0.0 });
    }

    pub(crate) fn tick_bot_alerts(&mut self, delta: f32) {
        if self.bot_alerts.is_empty() {
            return;
        }
        for a in &mut self.bot_alerts {
            a.age += delta;
        }
        self.bot_alerts.retain(|a| a.age < LIFE_SECS);
    }

    pub(crate) fn clear_bot_alerts(&mut self) {
        self.bot_alerts.clear();
    }

    /// Draw every live mark: over the bot's head when it is on screen, on
    /// the screen's edge toward it when not. Call in screen space.
    pub(crate) fn draw_bot_alerts(&self, camera: &Camera3D) {
        if self.bot_alerts.is_empty() {
            return;
        }
        let (w, h) = (screen_width(), screen_height());
        let forward = (camera.target - camera.position).normalize();
        let right = forward.cross(camera.up).normalize();
        let up = right.cross(forward).normalize();
        let fov_scale = h / (2.0 * (camera.fovy / 2.0).tan());
        let project = |world: Vec3| -> Option<Vec2> {
            let rel = world - camera.position;
            let dot = rel.dot(forward);
            if dot < 0.1 {
                return None;
            }
            Some(vec2(w / 2.0 + rel.dot(right) / dot * fov_scale, h / 2.0 - rel.dot(up) / dot * fov_scale))
        };

        for a in &self.bot_alerts {
            let bot = self
                .player_index
                .get(&a.bot_id)
                .and_then(|&i| self.other_players.get(i))
                .filter(|p| p.id == a.bot_id)
                .or_else(|| self.other_players.iter().find(|p| p.id == a.bot_id));
            // a bot that just died (by your hand, say) no longer watches you
            let Some(bot) = bot.filter(|p| p.is_alive && p.death_timer.is_none()) else { continue };
            let feet = bot.render_position();
            let (scale, alpha) = alert_look(a.age);
            let on_screen = project(feet + vec3(0.0, MARK_HEIGHT, 0.0))
                .filter(|p| p.x >= 0.0 && p.x <= w && p.y >= 0.0 && p.y <= h);
            match on_screen {
                Some(at) => draw_mark(at, HEAD_SIZE * scale, a.engaged, alpha, None),
                None => {
                    let me = vec2(self.position.x, self.position.z);
                    let angle = screen_angle(self.yaw, me, vec2(feet.x, feet.z));
                    let at = edge_point(w, h, EDGE_MARGIN, angle);
                    draw_mark(at, EDGE_SIZE * scale, a.engaged, alpha, Some(angle));
                }
            }
        }
    }
}

/// A "?" or "!" of height `size` centred on `at`, on a dark disc so it reads
/// over anything. With `pointing` (an edge marker) a small arrow on the disc
/// points that way (0 = up, clockwise).
fn draw_mark(at: Vec2, size: f32, engaged: bool, alpha: f32, pointing: Option<f32>) {
    use crate::ui::fonts::{measure_game_text, text_params};
    if size < 1.0 || alpha <= 0.0 {
        return;
    }
    let (r, g, b) = if engaged { ENGAGED_COLOR } else { SUSPICIOUS_COLOR };
    let color = Color::from_rgba(r, g, b, (alpha * 255.0) as u8);
    let radius = size * 0.62;
    crate::ui::prims::draw_circle(at.x, at.y, radius, Color::new(0.0, 0.0, 0.0, 0.45 * alpha));
    crate::ui::prims::draw_circle_lines(at.x, at.y, radius, (size * 0.07).max(1.5), color);
    if let Some(angle) = pointing {
        let dir = vec2(angle.sin(), -angle.cos());
        let side = vec2(-dir.y, dir.x);
        let tip = at + dir * (radius + size * 0.35);
        let base = at + dir * (radius - 1.0);
        draw_triangle(tip, base + side * size * 0.22, base - side * size * 0.22, color);
    }
    let glyph = if engaged { "!" } else { "?" };
    let font_size = size.round().clamp(8.0, 96.0) as u16;
    let tw = measure_game_text(glyph, font_size);
    // text sits on its baseline: drop it by about a third of its height to centre it
    draw_text_ex(glyph, at.x - tw / 2.0, at.y + size * 0.36, text_params(font_size, color));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mark_pops_holds_and_fades() {
        assert_eq!(alert_look(0.0), (0.0, 1.0));
        let (peak, _) = alert_look(POP_SECS);
        assert!((peak - POP_OVERSHOOT).abs() < 1e-4);
        assert_eq!(alert_look(POP_SECS + SETTLE_SECS + 0.01), (1.0, 1.0));
        assert_eq!(alert_look(POP_SECS + HOLD_SECS).1, 1.0, "held about a second");
        let (_, fading) = alert_look(LIFE_SECS - FADE_SECS / 2.0);
        assert!((fading - 0.5).abs() < 1e-4);
        assert_eq!(alert_look(LIFE_SECS).1, 0.0);
    }

    #[test]
    fn edge_markers_hug_the_side_toward_the_bot() {
        let (w, h, m) = (1600.0, 900.0, 40.0);
        let close = |a: Vec2, b: Vec2| (a - b).length() < 1e-2;
        // ahead: top middle; right: right middle; behind: bottom middle
        assert!(close(edge_point(w, h, m, 0.0), vec2(800.0, 40.0)));
        assert!(close(edge_point(w, h, m, std::f32::consts::FRAC_PI_2), vec2(1560.0, 450.0)));
        assert!(close(edge_point(w, h, m, std::f32::consts::PI), vec2(800.0, 860.0)));
        assert!(close(edge_point(w, h, m, -std::f32::consts::FRAC_PI_2), vec2(40.0, 450.0)));
        // every direction lands on the inset rectangle's border
        for i in 0..64 {
            let p = edge_point(w, h, m, i as f32 * 0.1);
            let on_x = (p.x - m).abs() < 0.01 || (p.x - (w - m)).abs() < 0.01;
            let on_y = (p.y - m).abs() < 0.01 || (p.y - (h - m)).abs() < 0.01;
            assert!(on_x || on_y, "{p:?} is not on the edge");
            assert!(p.x >= m - 0.01 && p.x <= w - m + 0.01 && p.y >= m - 0.01 && p.y <= h - m + 0.01);
        }
    }
}
