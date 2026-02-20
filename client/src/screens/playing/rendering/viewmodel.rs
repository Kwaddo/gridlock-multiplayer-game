use crate::weapons::stats::CrosshairStyle;

use macroquad::models::Mesh;
use macroquad::prelude::*;
use macroquad::window::get_internal_gl;

use super::super::PlayingState;

const HANDS_BASE_SCALE: f32 = 0.35;
const INSPECT_START_SEC: f32 = 0.35;
const INSPECT_SWEEP_SEC: f32 = 0.60;
const INSPECT_HOLD_SEC: f32 = 0.35;
const INSPECT_RETURN_SEC: f32 = 0.45;
const INSPECT_LOOK_DOWN_Y: f32 = -0.26;
const INSPECT_POS_A: Vec3 = Vec3::from_array([0.02, -0.03, -0.04]);
const INSPECT_ROT_A: Vec3 = Vec3::from_array([-0.22, 0.45, 0.16]);
const INSPECT_POS_B: Vec3 = Vec3::from_array([-0.03, -0.02, -0.06]);
const INSPECT_ROT_B: Vec3 = Vec3::from_array([-0.18, -0.60, -0.18]);

impl PlayingState {
    pub(crate) fn draw_viewmodel(&self) {
        let weapon = self.weapons.active();
        let stats = weapon.stats();

        if matches!(stats.crosshair_style, CrosshairStyle::Scope) && weapon.ads_progress > 0.85 {
            return;
        }

        /*
            ATTEMPT TO GET HAND ANIM MODEL
        */
        let hands_anim = self.animated_models.get("fp_hands");
        let hands_static = self.models.get("fp_hands");

        if let Some(model_key) = stats.viewmodel_model {
            if let Some(mesh) = self.models.get(model_key) {
                self.draw_3d_viewmodel(
                    mesh,
                    1.0,
                    0.0,
                    hands_anim,
                    hands_static,
                    weapon,
                    stats
                );
                return;
            } else if let Some(animated_model) = self.animated_models.get(model_key) {
                self.draw_3d_viewmodel(
                    animated_model.mesh(),
                    animated_model.normalize_scale,
                    animated_model.y_offset,
                    hands_anim,
                    hands_static,
                    weapon,
                    stats
                );
                return;
            } else {
                println!("Model '{}' not found in cache, using 2D fallback", model_key);
            }
        }

        self.draw_2d_viewmodel(weapon, stats);
    }

    fn draw_3d_viewmodel(
        &self,
        mesh: &Mesh,
        mesh_normalize_scale: f32,
        mesh_y_offset: f32,
        hands_anim: Option<&crate::types::animated_model::AnimatedModel>,
        hands_static: Option<&Mesh>,
        weapon: &crate::weapons::WeaponState,
        stats: &crate::weapons::stats::WeaponStats
    ) {
        if stats.crosshair_style == CrosshairStyle::Scope && weapon.ads_progress > 0.9 {
            return;
        }

        let (base_x, base_y, base_z) = stats.viewmodel_position;
        let (rot_pitch, rot_yaw, rot_roll) = stats.viewmodel_rotation;
        let (inspect_pos, inspect_rot, inspect_look_alpha) = match self.inspect_phase {
            super::super::state::InspectPhase::None => (Vec3::ZERO, Vec3::ZERO, 0.0),
            super::super::state::InspectPhase::Start => {
                let t = (self.inspect_timer / INSPECT_START_SEC).clamp(0.0, 1.0);
                (
                    Vec3::ZERO.lerp(INSPECT_POS_A, t),
                    Vec3::ZERO.lerp(INSPECT_ROT_A, t),
                    t
                )
            }
            super::super::state::InspectPhase::Sweep => {
                let t = (self.inspect_timer / INSPECT_SWEEP_SEC).clamp(0.0, 1.0);
                (
                    INSPECT_POS_A.lerp(INSPECT_POS_B, t),
                    INSPECT_ROT_A.lerp(INSPECT_ROT_B, t),
                    1.0
                )
            }
            super::super::state::InspectPhase::Hold => {
                let _ = INSPECT_HOLD_SEC;
                (INSPECT_POS_B, INSPECT_ROT_B, 1.0)
            }
            super::super::state::InspectPhase::Return => {
                let t = (self.inspect_timer / INSPECT_RETURN_SEC).clamp(0.0, 1.0);
                (
                    INSPECT_POS_B.lerp(Vec3::ZERO, t),
                    INSPECT_ROT_B.lerp(Vec3::ZERO, t),
                    1.0 - t
                )
            }
        };
        // camera and projection setup
        let vm_fov = (60.0f32).to_radians();
        let vm_camera = Camera3D {
            position: vec3(0.0, 0.04 * inspect_look_alpha, 0.0),
            target: vec3(0.0, INSPECT_LOOK_DOWN_Y * inspect_look_alpha, 1.0),
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

        let hands_weapon_offset = vec3(
            stats.viewmodel_hands_offset.0,
            stats.viewmodel_hands_offset.1,
            stats.viewmodel_hands_offset.2,
        );
        let hands_base_anchor = vec3(
            -0.35,
            -0.25,
            0.58,
        ) + hands_weapon_offset;
        let weapon_base_anchor = vec3(-0.35, -0.25, 0.58);
        let ads_offset_x = 0.08 * weapon.ads_progress;
        let ads_offset_y = 0.03 * weapon.ads_progress;
        let weapon_local_offset = vec3(base_x + ads_offset_x, base_y + ads_offset_y, base_z);

        let mut motion_offset = Vec3::ZERO;
        let horizontal_speed = vec2(
            self.movement_state.velocity.x,
            self.movement_state.velocity.z
        ).length();
        if horizontal_speed > 0.1 && self.movement_state.is_grounded {
            let t = self.bob_timer;
            motion_offset.x += t.sin() * 0.008;
            motion_offset.y += t.cos() * 0.006;
        }

        if weapon.is_reloading {
            let reload_fraction = (weapon.reload_timer / stats.reload_time).clamp(0.0, 1.0);
            motion_offset.y -= (reload_fraction * std::f32::consts::PI).sin() * 0.08;
        }

        let kick = weapon.muzzle_flash_timer / 0.08;
        motion_offset.z += kick * 0.02;
        motion_offset.y -= kick * 0.01;
        let hands_anchor = hands_base_anchor + motion_offset + inspect_pos;
        let weapon_anchor = weapon_base_anchor + motion_offset + inspect_pos;
        let pos = weapon_anchor + weapon_local_offset;

        let normalize_mat = Mat4::from_translation(vec3(0.0, mesh_y_offset, 0.0))
            * Mat4::from_scale(Vec3::splat(mesh_normalize_scale.max(0.0001)));
        let scale_mat = Mat4::from_scale(
            vec3(
                stats.viewmodel_scale * 28.0,
                stats.viewmodel_scale * 28.0,
                stats.viewmodel_scale * 28.0
            )
        );
        let weapon_rot_mat = Mat4::from_euler(
            EulerRot::XYZ,
            rot_pitch + inspect_rot.x,
            rot_yaw + inspect_rot.y,
            rot_roll + inspect_rot.z,
        );
        let trans_mat = Mat4::from_translation(pos);
        let transform = trans_mat * weapon_rot_mat * scale_mat * normalize_mat;

        gl_use_default_material();
        let gl = unsafe { get_internal_gl() }; // unsafe is required to access internal GL for custom shader uniforms
        gl.quad_gl.depth_test(true);

        // draw gun mesh
        let mut transformed_verts = mesh.vertices.clone();
        for v in &mut transformed_verts {
            v.position = transform.transform_point3(v.position);
        }
        let temp_mesh = Mesh {
            vertices: transformed_verts,
            indices: mesh.indices.clone(),
            texture: mesh.texture.clone(),
        };
        draw_mesh(&temp_mesh);

        /*
            DRAWING ANIMATED HANDS HERE
        */

        let draw_hands = |hands_mesh: &Mesh, normalize_scale: f32, y_offset: f32| {
            // Keep hands scale independent from weapon scale so tiny weapon tuning
            // does not make hands disappear.
            let hands_scale = HANDS_BASE_SCALE * stats.viewmodel_hands_scale.max(0.01);
            let hands_scale_mat = Mat4::from_scale(Vec3::splat(hands_scale));
            let hands_trans = Mat4::from_translation(hands_anchor);
            let hands_rot_mat = Mat4::from_euler(
                EulerRot::XYZ,
                stats.viewmodel_hands_rotation.0 + inspect_rot.x,
                stats.viewmodel_hands_rotation.1 + inspect_rot.y,
                stats.viewmodel_hands_rotation.2 + inspect_rot.z,
            );
            // Some FPS hand exports include hidden/far helper geometry that can poison
            // auto-normalization (very tiny normalize_scale), so clamp to a sane band.
            let clamped_normalize = normalize_scale.clamp(0.2, 5.0);
            let hands_normalize_mat = Mat4::from_translation(vec3(0.0, y_offset, 0.0))
                * Mat4::from_scale(Vec3::splat(clamped_normalize));
            let hands_transform = hands_trans * hands_rot_mat * hands_scale_mat * hands_normalize_mat;

            let mut hands_verts = hands_mesh.vertices.clone();
            for v in &mut hands_verts {
                v.position = hands_transform.transform_point3(v.position);
                // Viewmodel hands should be fully opaque regardless of source material alpha.
                v.color[3] = 255;
            }
            let hands_temp = Mesh {
                vertices: hands_verts,
                indices: hands_mesh.indices.clone(),
                texture: hands_mesh.texture.clone(),
            };
            draw_mesh(&hands_temp);
        };

        if let Some(hands_model) = hands_anim {
            draw_hands(
                hands_model.mesh(),
                hands_model.normalize_scale,
                hands_model.y_offset
            );
        } else if let Some(hands_mesh) = hands_static {
            draw_hands(hands_mesh, 1.0, 0.0);
        }

        let muzzle_local = vec3(
            stats.viewmodel_muzzle_offset.0,
            stats.viewmodel_muzzle_offset.1,
            stats.viewmodel_muzzle_offset.2
        );
        let muzzle_world = transform.transform_point3(muzzle_local);

        gl.quad_gl.depth_test(true);

        set_default_camera();

        if weapon.muzzle_flash_timer > 0.0 {
            if let Some((fx, fy)) = Self::project_to_screen(&vm_camera, muzzle_world) {
                let t = (weapon.muzzle_flash_timer / 0.08).min(1.0);
                let alpha = (220.0 * t) as u8;
                let radius = 18.0 * stats.viewmodel_scale * 28.0 * (0.4 + 0.6 * t);

                draw_circle(fx, fy, radius * 1.4, Color::from_rgba(255, 190, 80, alpha / 2));
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
        stats: &crate::weapons::stats::WeaponStats
    ) {
        let key = stats.viewmodel_asset_key.unwrap_or(stats.icon_asset_key);
        let tex = match self.assets.get(key) {
            Some(t) => t,
            None => {
                return;
            }
        };

        let screen_w = screen_width();
        let screen_h = screen_height();

        let target_h = (screen_h * (0.46 - weapon.ads_progress * 0.08)).clamp(
            160.0,
            screen_h * 0.8
        );
        let scale = (target_h / tex.height()).max(0.2) * stats.viewmodel_scale;
        let size = vec2(tex.width() * scale, tex.height() * scale);

        let hip_pos = vec2(screen_w * 0.68, screen_h * 0.88);
        let ads_pos = vec2(screen_w * 0.53, screen_h * 0.78);
        let mut pos = hip_pos.lerp(ads_pos, weapon.ads_progress);

        let horizontal_speed = vec2(
            self.movement_state.velocity.x,
            self.movement_state.velocity.z
        ).length();
        if horizontal_speed > 0.1 && self.movement_state.is_grounded {
            let t = self.bob_timer;
            pos.x += t.sin() * 5.0;
            pos.y += t.cos() * 3.0;
        }

        if weapon.is_reloading {
            let reload_fraction = (weapon.reload_timer / weapon.stats().reload_time).clamp(
                0.0,
                1.0
            );
            pos.y += (reload_fraction * std::f32::consts::PI).sin() * 12.0;
        }

        let draw_pos = vec2(pos.x - size.x, pos.y - size.y);

        draw_texture_ex(tex, draw_pos.x, draw_pos.y, WHITE, DrawTextureParams {
            dest_size: Some(size),
            ..Default::default()
        });

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
                Color::from_rgba(255, 190, 80, alpha / 2)
            );
            draw_circle(flash_pos.x, flash_pos.y, radius, Color::from_rgba(255, 240, 180, alpha));
        }
    }
}
