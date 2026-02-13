use macroquad::prelude::*;
use macroquad::models::Vertex;
use macroquad::window::get_internal_gl;
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

        // always show zone-colored hitboxes for the target dummy
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

    pub(super) fn draw_tracers(&self, camera_pos: Vec3) {
        for t in &self.tracers {
            let dir = t.end - t.start;
            let dist = dir.length();
            if dist < 0.001 {
                continue;
            }

            let head = t.start + dir * t.progress;
            let tail_t = (t.progress - t.tracer_length / dist).max(0.0);
            let tail = t.start + dir * tail_t;

            // if head and tail are the same point, nothing to draw
            if (head - tail).length() < 0.001 {
                continue;
            }

            let dir_norm = dir.normalize();
            let mid = (head + tail) * 0.5;
            let to_cam = camera_pos - mid;
            let to_cam_len = to_cam.length();
            if to_cam_len < 0.001 {
                continue;
            }
            let to_cam_norm = to_cam / to_cam_len;

            // cross product gives the "right" vector perpendicular to both
            // tracer direction and camera direction — this is what makes it billboard
            let right_raw = dir_norm.cross(to_cam_norm);
            let right_len_sq = right_raw.length_squared();

            // if cross product is degenerate (tracer points directly at/away from camera),
            // fall back to world-up to get a valid right vector
            let right = if right_len_sq < 0.0001 {
                let fallback = dir_norm.cross(vec3(0.0, 1.0, 0.0));
                let fb_len_sq = fallback.length_squared();
                if fb_len_sq < 0.0001 {
                    // tracer is perfectly vertical — use world-right
                    vec3(1.0, 0.0, 0.0)
                } else {
                    fallback / fb_len_sq.sqrt()
                }
            } else {
                right_raw / right_len_sq.sqrt()
            };

            let alpha = (t.lifetime * 255.0).min(255.0) as u8;
            let [r, g, b, _] = t.tracer_color;
            let half_w = t.tracer_thickness;
            let uv = vec2(0.0, 0.0);

            // second axis perpendicular to both right and direction — crossed billboards
            let up = right.cross(dir_norm).normalize();

            let color = Color::from_rgba(r, g, b, alpha);
            let head_color = Color::from_rgba(r, g, b, 0);
            let indices: [u16; 6] = [0, 1, 2, 2, 1, 0];

            let glow_alpha = ((alpha as f32) * 0.3) as u8;
            let glow_color = Color::from_rgba(r, g, b, glow_alpha);
            let glow_w = half_w * 0.6;

            // draw two crossed planes (right + up) for 3D depth illusion
            for axis in [right, up] {
                // inner bright streak
                let vertices = [
                    Vertex::new2(tail - axis * half_w, uv, color),
                    Vertex::new2(tail + axis * half_w, uv, color),
                    Vertex::new2(head, uv, head_color),
                ];
                let gl = unsafe { get_internal_gl() };
                gl.quad_gl.texture(None);
                gl.quad_gl.draw_mode(DrawMode::Triangles);
                gl.quad_gl.geometry(&vertices, &indices);

                // outer glow
                let glow_vertices = [
                    Vertex::new2(tail - axis * glow_w, uv, glow_color),
                    Vertex::new2(tail + axis * glow_w, uv, glow_color),
                    Vertex::new2(head, uv, Color::from_rgba(r, g, b, 0)),
                ];
                let gl = unsafe { get_internal_gl() };
                gl.quad_gl.texture(None);
                gl.quad_gl.draw_mode(DrawMode::Triangles);
                gl.quad_gl.geometry(&glow_vertices, &indices);
            }
        }
    }
}   
