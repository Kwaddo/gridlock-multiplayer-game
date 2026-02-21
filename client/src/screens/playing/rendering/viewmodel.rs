use crate::weapons::stats::CrosshairStyle;

use macroquad::models::Mesh;
use macroquad::prelude::*;
use macroquad::window::get_internal_gl;

use super::super::PlayingState;

impl PlayingState {
    pub(crate) fn draw_viewmodel(&self) {
        let weapon = self.weapons.active();
        let stats = weapon.stats();

        if matches!(stats.crosshair_style, CrosshairStyle::Scope) && weapon.ads_progress > 0.85 {
            return;
        }

        if let Some(model_key) = stats.viewmodel_model {
            if let Some(mesh) = self.models.get(model_key) {
                self.draw_3d_viewmodel(mesh, weapon, stats);
                return;
            } else {
                println!(
                    "Model '{}' not found in cache, using 2D fallback",
                    model_key
                );
            }
        }

        self.draw_2d_viewmodel(weapon, stats);
    }

    fn draw_3d_viewmodel(
        &self,
        mesh: &Mesh,
        weapon: &crate::weapons::WeaponState,
        stats: &crate::weapons::stats::WeaponStats,
    ) {
        if stats.crosshair_style == CrosshairStyle::Scope && weapon.ads_progress > 0.9 {
            return;
        }

        let vm_fov = (60.0f32).to_radians();
        let vm_camera = Camera3D {
            position: vec3(0.0, 0.0, 0.0),
            target: vec3(0.0, 0.0, 1.0),
            up: vec3(0.0, 1.0, 0.0),
            fovy: vm_fov,
            z_far: 10.0,
            z_near: 0.01,
            aspect: Some(screen_width() / screen_height()),
            projection: Projection::Perspective,
            render_target: None,
            viewport: None,
        };

        set_camera(&vm_camera);

        let (base_x, base_y, base_z) = stats.viewmodel_position;
        let (rot_pitch, rot_yaw, rot_roll) = stats.viewmodel_rotation;

        let ads_offset_x = 0.08 * weapon.ads_progress;
        let ads_offset_y = 0.03 * weapon.ads_progress;
        let mut pos = vec3(
            -0.35 + base_x + ads_offset_x,
            -0.25 + base_y + ads_offset_y,
            0.58 + base_z,
        );

        let horizontal_speed = vec2(
            self.movement_state.velocity.x,
            self.movement_state.velocity.z,
        )
        .length();
        if horizontal_speed > 0.1 && self.movement_state.is_grounded {
            let t = self.bob_timer;
            pos.x += t.sin() * 0.008;
            pos.y += t.cos() * 0.006;
        }

        if weapon.is_reloading {
            let reload_fraction = (weapon.reload_timer / stats.reload_time).clamp(0.0, 1.0);
            pos.y -= (reload_fraction * std::f32::consts::PI).sin() * 0.08;
        }

        let kick = weapon.muzzle_flash_timer / 0.08;
        pos.z += kick * 0.02;
        pos.y -= kick * 0.01;

        let scale_mat = Mat4::from_scale(vec3(
            stats.viewmodel_scale * 28.0,
            stats.viewmodel_scale * 28.0,
            stats.viewmodel_scale * 28.0,
        ));
        let rot_mat = Mat4::from_euler(EulerRot::XYZ, rot_pitch, rot_yaw, rot_roll);
        let trans_mat = Mat4::from_translation(pos);
        let transform = trans_mat * rot_mat * scale_mat;

        gl_use_default_material();
        let gl = unsafe { get_internal_gl() };
        gl.quad_gl.depth_test(false);

        // avoid allocating a transformed mesh every frame by using the GL model matrix
        gl.quad_gl.push_model_matrix(transform);
        draw_mesh(mesh);
        gl.quad_gl.pop_model_matrix();

        let muzzle_local = vec3(
            stats.viewmodel_muzzle_offset.0,
            stats.viewmodel_muzzle_offset.1,
            stats.viewmodel_muzzle_offset.2,
        );
        let muzzle_world = transform.transform_point3(muzzle_local);

        gl.quad_gl.depth_test(true);

        set_default_camera();

        if weapon.muzzle_flash_timer > 0.0 {
            if let Some((fx, fy)) = Self::project_to_screen(&vm_camera, muzzle_world) {
                let t = (weapon.muzzle_flash_timer / 0.08).min(1.0);
                let alpha = (220.0 * t) as u8;
                let radius = 18.0 * stats.viewmodel_scale * 28.0 * (0.4 + 0.6 * t);

                draw_circle(
                    fx,
                    fy,
                    radius * 1.4,
                    Color::from_rgba(255, 190, 80, alpha / 2),
                );
                draw_circle(fx, fy, radius, Color::from_rgba(255, 240, 180, alpha));
            }
        }
    }

    fn project_to_screen(cam: &Camera3D, world_pos: Vec3) -> Option<(f32, f32)> {
        let rel = world_pos - cam.position;
        let forward = (cam.target - cam.position).normalize();
        let right = forward.cross(cam.up).normalize();
        let up = right.cross(forward).normalize();

        let dot = rel.dot(forward);
        if dot < 0.1 {
            return None;
        }

        let fov_scale = screen_height() / (2.0 * (cam.fovy / 2.0).tan());
        let sx = screen_width() / 2.0 + (rel.dot(right) / dot) * fov_scale;
        let sy = screen_height() / 2.0 - (rel.dot(up) / dot) * fov_scale;
        Some((sx, sy))
    }

    fn draw_2d_viewmodel(
        &self,
        weapon: &crate::weapons::WeaponState,
        stats: &crate::weapons::stats::WeaponStats,
    ) {
        let key = stats.viewmodel_asset_key.unwrap_or(stats.icon_asset_key);
        let tex: &Texture2D = match self.assets.get(key) {
            Some(t) => t,
            None => {
                return;
            }
        };

        let screen_w = screen_width();
        let screen_h = screen_height();

        let target_h =
            (screen_h * (0.46 - weapon.ads_progress * 0.08)).clamp(160.0, screen_h * 0.8);
        let scale = (target_h / tex.height()).max(0.2) * stats.viewmodel_scale;
        let size = vec2(tex.width() * scale, tex.height() * scale);

        let hip_pos = vec2(screen_w * 0.68, screen_h * 0.88);
        let ads_pos = vec2(screen_w * 0.53, screen_h * 0.78);
        let mut pos = hip_pos.lerp(ads_pos, weapon.ads_progress);

        let horizontal_speed = vec2(
            self.movement_state.velocity.x,
            self.movement_state.velocity.z,
        )
        .length();
        if horizontal_speed > 0.1 && self.movement_state.is_grounded {
            let t = self.bob_timer;
            pos.x += t.sin() * 5.0;
            pos.y += t.cos() * 3.0;
        }

        if weapon.is_reloading {
            let reload_fraction =
                (weapon.reload_timer / weapon.stats().reload_time).clamp(0.0, 1.0);
            pos.y += (reload_fraction * std::f32::consts::PI).sin() * 12.0;
        }

        let draw_pos = vec2(pos.x - size.x, pos.y - size.y);

        draw_texture_ex(
            tex,
            draw_pos.x,
            draw_pos.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(size),
                ..Default::default()
            },
        );

        if weapon.muzzle_flash_timer > 0.0 {
            let t = (weapon.muzzle_flash_timer / 0.08).min(1.0);
            let alpha = (200.0 * t) as u8;
            let radius = 18.0 * t;
            let muzzle_offset = vec2(size.x * 0.88, size.y * 0.32);
            let flash_pos = draw_pos + muzzle_offset;

            draw_circle(
                flash_pos.x,
                flash_pos.y,
                radius * 1.5,
                Color::from_rgba(255, 190, 80, alpha / 2),
            );
            draw_circle(
                flash_pos.x,
                flash_pos.y,
                radius,
                Color::from_rgba(255, 240, 180, alpha),
            );
        }
    }
}
