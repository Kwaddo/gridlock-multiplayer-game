use macroquad::prelude::*;

use super::super::PlayingState;

impl PlayingState {
    pub(crate) fn draw_target_dummy_hud(&self, camera: &Camera3D) {
        let d = &self.target_dummy;
        let world_pos = d.position + vec3(0.0, 2.0, 0.0);
        let rel = world_pos - camera.position;
        let forward = (camera.target - camera.position).normalize();
        let right = forward.cross(camera.up).normalize();
        let up = right.cross(forward).normalize();

        let dot = rel.dot(forward);
        if dot < 0.1 {
            return;
        }

        let fov_rad = camera.fovy;
        let fov_scale = screen_height() / (2.0 * (fov_rad / 2.0).tan());

        let sx = screen_width() / 2.0 + (rel.dot(right) / dot) * fov_scale;
        let sy = screen_height() / 2.0 - (rel.dot(up) / dot) * fov_scale;

        let bar_w = 60.0;
        let bar_h = 6.0;
        let hp_frac = d.hp / d.max_hp;

        draw_rectangle(
            sx - bar_w / 2.0,
            sy,
            bar_w,
            bar_h,
            Color::from_rgba(40, 40, 40, 200),
        );
        let hp_color = if hp_frac > 0.5 {
            Color::from_rgba(80, 200, 80, 255)
        } else if hp_frac > 0.25 {
            Color::from_rgba(220, 180, 40, 255)
        } else {
            RED
        };
        draw_rectangle(sx - bar_w / 2.0, sy, bar_w * hp_frac, bar_h, hp_color);
        draw_rectangle_lines(sx - bar_w / 2.0, sy, bar_w, bar_h, 1.0, WHITE);

        let label = "TARGET DUMMY";
        // use precomputed width stored in playing state to avoid expensive text measurement
        let measure_w = self.target_dummy_label_dims.0;
        draw_text(label, sx - measure_w / 2.0, sy - 4.0, 14.0, WHITE);
    }

    /// Draws nametags (always) and health bars (dev mode) above other players.
    /// Uses proper camera-based 3D-to-screen projection so tags stay directly
    /// above the player model.
    pub(crate) fn draw_player_overhead(&self, camera: &Camera3D) {
        let forward = (camera.target - camera.position).normalize();
        let right = forward.cross(camera.up).normalize();
        let up = right.cross(forward).normalize();
        let fov_rad = camera.fovy;
        let fov_scale = screen_height() / (2.0 * (fov_rad / 2.0).tan());

        let show_hp = self.dev_mode.show_health_bars;

        // helper: project a world-space point to screen and draw nametag + optional health bar
        let project = |world_pos: Vec3| -> Option<(f32, f32)> {
            let rel = world_pos - camera.position;
            let dot = rel.dot(forward);
            if dot < 0.1 {
                return None;
            }
            let sx = screen_width() / 2.0 + (rel.dot(right) / dot) * fov_scale;
            let sy = screen_height() / 2.0 - (rel.dot(up) / dot) * fov_scale;
            Some((sx, sy))
        };

        // Target dummy (lobby only)
        if show_hp && self.current_world == crate::types::world::WorldId::Lobby {
            let world_pos = self.target_dummy.position + vec3(0.0, 2.0, 0.0);
            if let Some((sx, sy)) = project(world_pos) {
                Self::draw_health_bar(
                    sx,
                    sy,
                    self.target_dummy.hp,
                    self.target_dummy.max_hp,
                    "Target Dummy",
                );
            }
        }

        // Other players
        for p in &self.other_players {
            let world_pos = p.render_position() + vec3(0.0, 2.0, 0.0);
            if let Some((sx, sy)) = project(world_pos) {
                if show_hp {
                    Self::draw_health_bar(sx, sy, p.hp, p.max_hp, &p.username);
                } else {
                    Self::draw_nametag(sx, sy, &p.username, p.color);
                }
            }
        }

        // Practice bots
        for bot in &self.practice_bots {
            let p = &bot.player;
            let world_pos = vec3(p.x, p.y + 2.0, p.z);
            if let Some((sx, sy)) = project(world_pos) {
                if show_hp {
                    Self::draw_health_bar(sx, sy, p.hp, p.max_hp, &p.username);
                } else {
                    Self::draw_nametag(sx, sy, &p.username, p.color);
                }
            }
        }
    }

    fn draw_nametag(sx: f32, sy: f32, label: &str, color: Color) {
        use crate::ui::fonts::{measure_game_text, text_params};
        let label_w = measure_game_text(label, 14);
        draw_text_ex(label, sx - label_w / 2.0, sy, text_params(14, color));
    }

    fn draw_health_bar(sx: f32, sy: f32, hp: f32, max_hp: f32, label: &str) {
        use crate::ui::fonts::{measure_game_text, text_params};

        let bar_w = 60.0;
        let bar_h = 6.0;
        let hp_frac = (hp / max_hp).clamp(0.0, 1.0);

        draw_rectangle(
            sx - bar_w / 2.0,
            sy,
            bar_w,
            bar_h,
            Color::from_rgba(40, 40, 40, 200),
        );

        let hp_color = if hp_frac > 0.5 {
            Color::from_rgba(80, 200, 80, 255)
        } else if hp_frac > 0.25 {
            Color::from_rgba(220, 180, 40, 255)
        } else {
            RED
        };
        draw_rectangle(sx - bar_w / 2.0, sy, bar_w * hp_frac, bar_h, hp_color);

        draw_rectangle_lines(sx - bar_w / 2.0, sy, bar_w, bar_h, 1.0, WHITE);

        let label_w = measure_game_text(label, 12);
        draw_text_ex(label, sx - label_w / 2.0, sy - 4.0, text_params(12, WHITE));
    }
}
