use macroquad::models::Vertex;
use macroquad::prelude::*;
use macroquad::window::get_internal_gl;

use super::super::PlayingState;

impl PlayingState {
    pub(crate) fn draw_tracers(&self, camera_pos: Vec3) {
        for t in &self.tracers {
            let dir = t.end - t.start;
            let dist = dir.length();
            if dist < 0.001 {
                continue;
            }

            let head = t.start + dir * t.progress;
            let tail_t = (t.progress - t.tracer_length / dist).max(0.0);
            let tail = t.start + dir * tail_t;

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

            let right_raw = dir_norm.cross(to_cam_norm);
            let right_len_sq = right_raw.length_squared();

            let right = if right_len_sq < 0.0001 {
                let fallback = dir_norm.cross(vec3(0.0, 1.0, 0.0));
                let fb_len_sq = fallback.length_squared();
                if fb_len_sq < 0.0001 {
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

            let up = right.cross(dir_norm).normalize();

            let color = Color::from_rgba(r, g, b, alpha);
            let head_color = Color::from_rgba(r, g, b, 0);
            let indices: [u16; 6] = [0, 1, 2, 2, 1, 0];

            let glow_alpha = ((alpha as f32) * 0.3) as u8;
            let glow_color = Color::from_rgba(r, g, b, glow_alpha);
            let glow_w = half_w * 0.6;

            for axis in [right, up] {
                let vertices = [
                    Vertex::new2(tail - axis * half_w, uv, color),
                    Vertex::new2(tail + axis * half_w, uv, color),
                    Vertex::new2(head, uv, head_color),
                ];
                let gl = unsafe { get_internal_gl() };
                gl.quad_gl.texture(None);
                gl.quad_gl.draw_mode(DrawMode::Triangles);
                gl.quad_gl.geometry(&vertices, &indices);

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
