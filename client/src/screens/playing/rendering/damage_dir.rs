//! Directional damage indicator: a wide red arc around the crosshair, with an
//! arrowhead, pointing to where the shot that hit you came from. The arc
//! keeps pointing at that spot as you turn, and fades out after a moment.

use std::cell::RefCell;

use macroquad::models::Vertex;
use macroquad::prelude::*;
use macroquad::window::get_internal_gl;

use super::super::PlayingState;

/// How long an indicator stays up after the last hit from that attacker.
const INDICATOR_SECS: f32 = 1.6;
/// The last part of that time it spends fading out.
const FADE_SECS: f32 = 0.6;
/// At most this many indicators at once (the oldest gives way).
const MAX_INDICATORS: usize = 6;
/// Half the angle the arc spans.
const HALF_SPAN: f32 = 24.0 * std::f32::consts::PI / 180.0;
/// Segments along the arc.
const SEGMENTS: usize = 16;
const COLOR: (u8, u8, u8) = (220, 28, 28);

/// Where one hit came from.
pub(crate) struct DamageIndicator {
    attacker_id: u32,
    /// The attacker's position when the hit landed (feet).
    source: Vec3,
    timer: f32,
}

thread_local! {
    static SCRATCH: RefCell<(Vec<Vertex>, Vec<u16>)> = const { RefCell::new((Vec::new(), Vec::new())) };
}

impl PlayingState {
    /// Remember where a hit from `attacker_id` came from. Attackers we cannot
    /// place (unknown, or yourself: you are not among the other players) get
    /// no indicator.
    pub(crate) fn note_damage_direction(&mut self, attacker_id: u32) {
        let source = self
            .player_index
            .get(&attacker_id)
            .and_then(|&i| self.other_players.get(i))
            .filter(|p| p.id == attacker_id)
            .or_else(|| self.other_players.iter().find(|p| p.id == attacker_id))
            .or_else(|| self.practice_bots.iter().map(|b| &b.player).find(|p| p.id == attacker_id))
            .map(|p| p.render_position());
        let Some(source) = source else { return };

        if let Some(existing) = self.damage_indicators.iter_mut().find(|d| d.attacker_id == attacker_id) {
            existing.source = source;
            existing.timer = INDICATOR_SECS;
            return;
        }
        if self.damage_indicators.len() >= MAX_INDICATORS {
            // drop the one closest to fading out
            if let Some(oldest) = self
                .damage_indicators
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.timer.total_cmp(&b.1.timer))
                .map(|(i, _)| i)
            {
                self.damage_indicators.swap_remove(oldest);
            }
        }
        self.damage_indicators.push(DamageIndicator { attacker_id, source, timer: INDICATOR_SECS });
    }

    pub(crate) fn tick_damage_indicators(&mut self, delta: f32) {
        if self.damage_indicators.is_empty() {
            return;
        }
        for d in &mut self.damage_indicators {
            d.timer -= delta;
        }
        self.damage_indicators.retain(|d| d.timer > 0.0);
    }

    pub(crate) fn clear_damage_indicators(&mut self) {
        self.damage_indicators.clear();
    }

    /// Draw every live indicator around the screen centre. Call in screen
    /// space (default camera).
    pub(crate) fn draw_damage_indicators(&self) {
        if self.damage_indicators.is_empty() {
            return;
        }
        let (w, h) = (screen_width(), screen_height());
        let center = vec2(w * 0.5, h * 0.5);
        let radius = w.min(h) * 0.16;
        let thickness = radius * 0.14;

        // the player's forward and right on the ground plane (as audio panning uses them)
        let forward = vec2(self.yaw.cos(), self.yaw.sin());
        let right = vec2(-self.yaw.sin(), self.yaw.cos());

        for d in &self.damage_indicators {
            let to_source = vec2(d.source.x - self.position.x, d.source.z - self.position.z);
            if to_source.length_squared() < 1e-4 {
                continue;
            }
            // clockwise from straight ahead (the top of the screen)
            let angle = to_source.dot(right).atan2(to_source.dot(forward));
            let alpha = (d.timer / FADE_SECS).min(1.0);
            draw_indicator(center, radius, thickness, angle, alpha);
        }
    }
}

/// A curved band from `radius` to `radius + thickness`, centred on `angle`
/// (0 = up, clockwise), brightest in the middle and on its outer edge, plus
/// an arrowhead on the outside.
fn draw_indicator(center: Vec2, radius: f32, thickness: f32, angle: f32, alpha: f32) {
    let (r, g, b) = COLOR;
    let at = |a: f32, dist: f32| center + vec2(a.sin(), -a.cos()) * dist;
    let color = |a: f32| Color::from_rgba(r, g, b, (a.clamp(0.0, 1.0) * 255.0) as u8);

    SCRATCH.with_borrow_mut(|(vertices, indices)| {
        vertices.clear();
        indices.clear();
        let uv = Vec2::ZERO;
        for i in 0..=SEGMENTS {
            let u = i as f32 / SEGMENTS as f32;
            let a = angle - HALF_SPAN + 2.0 * HALF_SPAN * u;
            // fades to nothing at both ends of the arc
            let along = (u * std::f32::consts::PI).sin().powf(0.7);
            let inner = at(a, radius);
            let outer = at(a, radius + thickness);
            vertices.push(Vertex::new(inner.x, inner.y, 0.0, uv.x, uv.y, color(alpha * along * 0.35)));
            vertices.push(Vertex::new(outer.x, outer.y, 0.0, uv.x, uv.y, color(alpha * along * 0.9)));
            if i < SEGMENTS {
                let k = (2 * i) as u16;
                indices.extend_from_slice(&[k, k + 1, k + 2, k + 2, k + 1, k + 3]);
            }
        }
        // arrowhead pointing outwards from the middle of the arc
        let base = vertices.len() as u16;
        let head_half = HALF_SPAN * 0.32;
        let tip = at(angle, radius + thickness * 2.3);
        let left = at(angle - head_half, radius + thickness);
        let right = at(angle + head_half, radius + thickness);
        for p in [left, right, tip] {
            vertices.push(Vertex::new(p.x, p.y, 0.0, uv.x, uv.y, color(alpha * 0.9)));
        }
        indices.extend_from_slice(&[base, base + 1, base + 2]);

        let gl = unsafe { get_internal_gl() };
        gl.quad_gl.texture(None);
        gl.quad_gl.draw_mode(DrawMode::Triangles);
        gl.quad_gl.geometry(vertices, indices);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Same maths as draw_damage_indicators: angle clockwise from ahead.
    fn screen_angle(yaw: f32, me: Vec2, source: Vec2) -> f32 {
        let forward = vec2(yaw.cos(), yaw.sin());
        let right = vec2(-yaw.sin(), yaw.cos());
        let to = source - me;
        to.dot(right).atan2(to.dot(forward))
    }

    #[test]
    fn points_ahead_right_behind_left() {
        let yaw = 0.7f32;
        let fwd = vec2(yaw.cos(), yaw.sin());
        let right = vec2(-yaw.sin(), yaw.cos());
        let me = vec2(3.0, -2.0);
        let close = |a: f32, b: f32| (a - b).abs() < 1e-4;
        assert!(close(screen_angle(yaw, me, me + fwd * 5.0), 0.0));
        assert!(close(screen_angle(yaw, me, me + right * 5.0), std::f32::consts::FRAC_PI_2));
        assert!(close(screen_angle(yaw, me, me - fwd * 5.0).abs(), std::f32::consts::PI));
        assert!(close(screen_angle(yaw, me, me - right * 5.0), -std::f32::consts::FRAC_PI_2));
    }
}
