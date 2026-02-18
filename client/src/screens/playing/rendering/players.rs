use crate::weapons::combat::HitZone;

use macroquad::prelude::*;
use macroquad::window::get_internal_gl;

use super::super::PlayingState;

const WORLD_PLAYER_RENDER_SCALE: f32 = 0.94;
// Mixamo-style rigs are authored facing +Z, while gameplay yaw uses +X as forward.
// Keep this offset centralized so yaw logic and movement math stay in gameplay space.
const PLAYER_MODEL_YAW_OFFSET: f32 = std::f32::consts::FRAC_PI_2;

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
        let render_pos = p.render_position();
        let render_yaw = p.render_yaw;
        let desired_key = p.skin_type.model_key();
        let desired_anim = desired_key.and_then(|k| self.animated_models.get(k));
        let desired_static = desired_key.and_then(|k| self.models.get(k));

        let anim_model = desired_anim.or_else(|| self.animated_models.get("anim_rifle_idle"));
        let static_mesh = desired_static.or_else(|| self.models.get("anim_rifle_idle"));

        if p.skin_type.is_model_based() {
            if let Some(anim) = anim_model {
                let s = anim.normalize_scale * WORLD_PLAYER_RENDER_SCALE;
                let y_offset = anim.y_offset * WORLD_PLAYER_RENDER_SCALE;
                let transform =
                    Mat4::from_translation(render_pos + vec3(0.0, y_offset, 0.0))
                    * Mat4::from_rotation_y(-render_yaw + PLAYER_MODEL_YAW_OFFSET)
                    * Mat4::from_scale(vec3(s, s, s));
                let gl = unsafe { get_internal_gl() };
                gl.quad_gl.push_model_matrix(transform);
                draw_mesh(anim.mesh());
                gl.quad_gl.pop_model_matrix();
                return;
            }
            if let Some(mesh) = static_mesh {
                let transform = Mat4::from_translation(render_pos)
                    * Mat4::from_rotation_y(-render_yaw + PLAYER_MODEL_YAW_OFFSET)
                    * Mat4::from_scale(vec3(
                        WORLD_PLAYER_RENDER_SCALE,
                        WORLD_PLAYER_RENDER_SCALE,
                        WORLD_PLAYER_RENDER_SCALE,
                    ));
                let gl = unsafe { get_internal_gl() };
                gl.quad_gl.push_model_matrix(transform);
                draw_mesh(mesh);
                gl.quad_gl.pop_model_matrix();
                return;
            }
        }

        let color = if p.skin.is_some() { WHITE } else { p.color };
        draw_cube(render_pos + vec3(0.0, 0.6, 0.0), vec3(0.6, 1.2, 0.4), None, color);
        draw_cube(render_pos + vec3(0.0, 1.4, 0.0), vec3(0.4, 0.4, 0.4), None, color);
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

    pub(crate) fn draw_player_hitboxes(&self) {
        if !self.dev_mode.show_hitboxes {
            return;
        }

        for p in &self.other_players {
            Self::draw_entity_hitboxes(p.render_position(), &p.hitboxes);
        }
        for bot in &self.practice_bots {
            Self::draw_entity_hitboxes(bot.player.render_position(), &bot.player.hitboxes);
        }
    }

    fn draw_entity_hitboxes(origin: Vec3, hitboxes: &[crate::weapons::combat::Hitbox]) {
        for hb in hitboxes {
            let center = origin + hb.center_offset;
            let size = hb.half_extents * 2.0;
            let wire_color = match hb.zone {
                HitZone::Head => Color::from_rgba(255, 120, 120, 255),
                HitZone::Torso => Color::from_rgba(120, 200, 255, 255),
                HitZone::Limbs => Color::from_rgba(120, 255, 120, 255),
            };
            draw_cube_wires(center, size, wire_color);
        }
    }
}
