use macroquad::prelude::*;
use crate::weapons::combat::HitZone;

use super::PlayingState;

impl PlayingState {
    pub(super) fn draw_other_players(&self) {
        for p in &self.other_players {
            let color = if p.skin.is_some() { WHITE } else { p.color };
            draw_cube(vec3(p.x, p.y + 0.6, p.z), vec3(0.6, 1.2, 0.4), None, color);
            draw_cube(vec3(p.x, p.y + 1.4, p.z), vec3(0.4, 0.4, 0.4), None, color);
        }
    }

    pub(super) fn draw_target_dummy(&self) {
        let d = &self.target_dummy;
        let hit_flash = d.hit_timer > 0.0;

        if self.dev_mode.show_hitboxes {
            // show zone-colored hitboxes with wireframes
            for hb in &d.hitboxes {
                let center = d.position + hb.center_offset;
                let size = hb.half_extents * 2.0;

                let base_color = match hb.zone {
                    HitZone::Head => Color::from_rgba(255, 80, 80, 200),
                    HitZone::Torso => Color::from_rgba(80, 180, 255, 200),
                    HitZone::Limbs => Color::from_rgba(80, 255, 80, 200),
                };

                let color = if hit_flash { WHITE } else { base_color };

                let wire_color = match hb.zone {
                    HitZone::Head => Color::from_rgba(255, 120, 120, 255),
                    HitZone::Torso => Color::from_rgba(120, 200, 255, 255),
                    HitZone::Limbs => Color::from_rgba(120, 255, 120, 255),
                };

                draw_cube(center, size, None, color);
                draw_cube_wires(center, size, wire_color);
            }
        } else {
            // simple neutral dummy model
            let body_color = if hit_flash { WHITE } else { Color::from_rgba(180, 140, 100, 255) };
            draw_cube(d.position + vec3(0.0, 0.6, 0.0), vec3(0.6, 1.2, 0.4), None, body_color);
            draw_cube(d.position + vec3(0.0, 1.4, 0.0), vec3(0.4, 0.4, 0.4), None, body_color);
        }
    }

    pub(super) fn draw_target_dummy_hud(&self, camera: &Camera3D) {
        let d = &self.target_dummy;
        // project 3D position above head to screen space
        let world_pos = d.position + vec3(0.0, 2.0, 0.0);
        let rel = world_pos - camera.position;
        let forward = (camera.target - camera.position).normalize();
        let right = forward.cross(camera.up).normalize();
        let up = right.cross(forward).normalize();

        let dot = rel.dot(forward);
        if dot < 0.1 {
            return;
        } // behind camera

        let fov_rad = camera.fovy; // already in radians
        let fov_scale = screen_height() / (2.0 * (fov_rad / 2.0).tan());

        let sx = screen_width() / 2.0 + (rel.dot(right) / dot) * fov_scale;
        let sy = screen_height() / 2.0 - (rel.dot(up) / dot) * fov_scale;

        // health bar
        let bar_w = 60.0;
        let bar_h = 6.0;
        let hp_frac = d.hp / d.max_hp;

        draw_rectangle(sx - bar_w / 2.0, sy, bar_w, bar_h, Color::from_rgba(40, 40, 40, 200));
        let hp_color = if hp_frac > 0.5 {
            Color::from_rgba(80, 200, 80, 255)
        } else if hp_frac > 0.25 {
            Color::from_rgba(220, 180, 40, 255)
        } else {
            RED
        };
        draw_rectangle(sx - bar_w / 2.0, sy, bar_w * hp_frac, bar_h, hp_color);
        draw_rectangle_lines(sx - bar_w / 2.0, sy, bar_w, bar_h, 1.0, WHITE);

        // label
        let label = "TARGET DUMMY";
        let measure = measure_text(label, None, 14, 1.0);
        draw_text(label, sx - measure.width / 2.0, sy - 4.0, 14.0, WHITE);
    }

    pub(super) fn draw_tracers(&self) {
        for t in &self.tracers {
            let dir = t.end - t.start;
            let dist = dir.length();

            let head = t.start + dir * t.progress; // current end point of the tracer
            let tail_t = (t.progress - t.tracer_length / dist).max(0.0);
            let tail = t.start + dir * tail_t; // start point of the tracer

            let _mid = (head + tail) * 0.5;
            let _length = (head - tail).length();
            let dir_norm = dir.normalize();

            // rotate cube to align with direction
            // size = thin on X/Y Axis, stretched on Z axis (length)
            // draw 2-3 lines offset by the up and right vectors since we dont have thickness in macroquad's draw_line_3d

            let offset = 0.02; // thickness of the tracer
            let up = vec3(0.0, 1.0, 0.0);
            let side = dir_norm.cross(up).normalize() * offset;

            let alpha = (t.lifetime * 255.0).min(255.0) as u8;
            let color = Color::from_rgba(255, 200, 80, alpha); // bright orange with fading alpha

            draw_line_3d(tail, head, color);
            draw_line_3d(tail + side, head + side, color);
            draw_line_3d(tail - side, head - side, color);
            draw_line_3d(tail + up * offset, head + up * offset, color);
        }
    }
}
