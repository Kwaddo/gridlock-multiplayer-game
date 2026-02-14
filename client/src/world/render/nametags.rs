use macroquad::prelude::*;

use crate::types::player::OtherPlayer;

pub fn draw_nametags(
    others: &[OtherPlayer],
    position: Vec3,
    forward: Vec3,
    right: Vec3,
    pitch: f32,
) {
    for p in others {
        let head_pos = vec3(p.x, p.y + 1.8, p.z);
        let rel = head_pos - position;
        let dot = rel.dot(forward);
        if dot > 0.1 {
            let right_dot = rel.dot(right);
            let up_dot = rel.y - position.y + pitch.sin() * dot;
            let fov_scale = screen_height() / (2.0 * (45.0_f32.to_radians() / 2.0).tan());
            let screen_x = screen_width() / 2.0 + (right_dot / dot) * fov_scale;
            let screen_y = screen_height() / 2.0 - (up_dot / dot) * fov_scale;
            let name_w = measure_text(&p.username, None, 16, 1.0).width;
            draw_text(
                &p.username,
                screen_x - name_w / 2.0,
                screen_y,
                16.0,
                p.color,
            );
        }
    }
}
