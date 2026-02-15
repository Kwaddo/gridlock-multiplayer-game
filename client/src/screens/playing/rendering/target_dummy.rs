use super::super::PlayingState;
use crate::weapons::combat::HitZone;
use macroquad::prelude::*;

impl PlayingState {
    pub(super) fn draw_target_dummy(&self) {
        let d = &self.target_dummy;
        let hit_flash = d.hit_timer > 0.0;

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
        let measure = measure_text(label, None, 14, 1.0);
        draw_text(label, sx - measure.width / 2.0, sy - 4.0, 14.0, WHITE);
    }
}
