use crate::weapons::combat::HitZone;

use macroquad::prelude::*;
use macroquad::window::get_internal_gl;

use super::super::PlayingState;

impl PlayingState {
    pub(crate) fn draw_other_players(&self) {
        for p in &self.other_players {
            self.draw_player_model(p);
        }

        for bot in &self.practice_bots {
            self.draw_player_model(&bot.player);
        }
    }

    fn draw_player_model(&self, p: &crate::types::player::OtherPlayer) {
        let desired_key = p.skin_type.model_key();
        let desired_anim = desired_key.and_then(|k| self.animated_models.get(k));
        let desired_static = desired_key.and_then(|k| self.models.get(k));

        let anim_model = desired_anim.or_else(|| self.animated_models.get("rifle_idle"));
        let static_mesh = desired_static.or_else(|| self.models.get("rifle_idle"));

        if let Some(key) = desired_key {
            if desired_anim.is_some() || desired_static.is_some() {
                let source = if desired_anim.is_some() {
                    "animated"
                } else {
                    "static"
                };
                println!(
                    "[model-debug] key hit key='{}' source={} skin='{}' player_id={}",
                    key,
                    source,
                    p.skin_type.label(),
                    p.id
                );
            }
            if desired_anim.is_none() && desired_static.is_none() {
                println!(
                    "[model-debug] missing requested key='{}' skin='{}' player_id={} -> trying fallback 'rifle_idle'",
                    key,
                    p.skin_type.label(),
                    p.id
                );
            }
        }

        if p.skin_type.is_model_based() {
            if desired_anim.is_none() && anim_model.is_some() {
                println!(
                    "[model-debug] using animated fallback 'rifle_idle' for skin='{}' player_id={}",
                    p.skin_type.label(),
                    p.id
                );
            }
            if desired_anim.is_none() && desired_static.is_none() && static_mesh.is_some() {
                println!(
                    "[model-debug] using static fallback 'rifle_idle' for skin='{}' player_id={}",
                    p.skin_type.label(),
                    p.id
                );
            }
            if let Some(anim) = anim_model {
                let s = anim.normalize_scale;
                let transform = Mat4::from_translation(vec3(p.x, p.y + anim.y_offset, p.z))
                    * Mat4::from_rotation_y(-p.yaw)
                    * Mat4::from_scale(vec3(s, s, s));
                let gl = unsafe { get_internal_gl() };
                gl.quad_gl.push_model_matrix(transform);
                draw_mesh(anim.mesh());
                gl.quad_gl.pop_model_matrix();
                return;
            }
            if let Some(mesh) = static_mesh {
                let transform =
                    Mat4::from_translation(vec3(p.x, p.y, p.z)) * Mat4::from_rotation_y(-p.yaw);
                let gl = unsafe { get_internal_gl() };
                gl.quad_gl.push_model_matrix(transform);
                draw_mesh(mesh);
                gl.quad_gl.pop_model_matrix();
                return;
            }
        }

        let color = if p.skin.is_some() { WHITE } else { p.color };
        draw_cube(vec3(p.x, p.y + 0.6, p.z), vec3(0.6, 1.2, 0.4), None, color);
        draw_cube(vec3(p.x, p.y + 1.4, p.z), vec3(0.4, 0.4, 0.4), None, color);
    }

    pub(crate) fn draw_target_dummy(&self) {
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
}
