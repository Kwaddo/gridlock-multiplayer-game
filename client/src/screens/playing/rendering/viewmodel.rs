use crate::weapons::stats::{CrosshairStyle, WeaponCategory, WeaponId};

use macroquad::models::Mesh;
use macroquad::prelude::*;
use macroquad::window::get_internal_gl;

use super::super::PlayingState;

const HANDS_BASE_SCALE: f32 = 0.35;

impl PlayingState {
    fn preferred_hands_key_for_viewmodel(id: WeaponId) -> &'static str {
        if id == WeaponId::Minigun {
            return "fp_hands_minigun";
        }
        if id == WeaponId::Sawnoff {
            return "fp_hands_sawnoff";
        }
        match id.category() {
            WeaponCategory::Pistol => "fp_hands_pistol",
            WeaponCategory::Smg => "fp_hands_smg",
            WeaponCategory::Sniper => "fp_hands_sniper",
            _ => "fp_hands",
        }
    }

    fn draw_punch_sprite(&self, tex: &Texture2D) {
        let sw = screen_width();
        let sh = screen_height();
        let target_h = sh * 0.65;
        let scale = target_h / tex.height().max(1.0);
        let size = vec2(tex.width() * scale, tex.height() * scale);
        let x = (sw - size.x) * 0.5;
        let y = sh - size.y * 0.9;
        draw_texture_ex(
            tex,
            x,
            y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(size),
                ..Default::default()
            },
        );
    }

    pub(crate) fn draw_viewmodel(&self) {
        let weapon = self.weapons.active();
        let stats = weapon.stats();

        // Punch: when knife_active, show the appropriate punch frame sprite.
        // Only render during the attack lock (the actual punch motion).
        // No idle viewmodel when just standing with punch ready.
        if weapon.id == WeaponId::Knife {
            if self.knife_attack_locked {
                let frame_key = self.punch_frame_key();
                if let Some(tex) = self.assets.get(frame_key) {
                    self.draw_punch_sprite(tex);
                }
            }
            // No rendering when not actively punching (clean — no knife standing idle)
            return;
        }

        let force_ads = self.dev_mode.is_unlocked() && self.dev_mode.force_ads;
        if matches!(stats.crosshair_style, CrosshairStyle::Scope)
            && weapon.ads_progress > 0.85
            && !force_ads
        {
            return;
        }

        let preferred_hands_key = Self::preferred_hands_key_for_viewmodel(weapon.id);
        let hands_anim = self
            .animated_models
            .get(preferred_hands_key)
            .or_else(|| self.animated_models.get("fp_hands"));
        let hands_static = self
            .models
            .get(preferred_hands_key)
            .and_then(|mvec| mvec.first())
            .or_else(|| self.models.get("fp_hands").and_then(|mvec| mvec.first()));
        let draw_hands = !matches!(weapon.id, WeaponId::Sawnoff | WeaponId::Minigun);

        if let Some(model_key) = stats.viewmodel_model {
            // Hands and muzzle flash go with the first mesh only: a weapon
            // made of many meshes used to redraw the hands for every one.
            if let Some(meshes) = self.models.get(model_key) {
                for (i, mesh) in meshes.iter().enumerate() {
                    self.draw_3d_viewmodel(
                        mesh,
                        1.0,
                        0.0,
                        hands_anim,
                        hands_static,
                        weapon,
                        stats,
                        draw_hands,
                        i == 0,
                    );
                }
                return;
            } else if let Some(animated_model) = self.animated_models.get(model_key) {
                for (i, mesh) in animated_model.meshes().enumerate() {
                    self.draw_3d_viewmodel(
                        mesh,
                        animated_model.normalize_scale,
                        animated_model.y_offset,
                        hands_anim,
                        hands_static,
                        weapon,
                        stats,
                        draw_hands,
                        i == 0,
                    );
                }
                return;
            }
            // Not loaded yet (it arrives in the background after joining) or
            // failed to load: the 2D sprite below stands in.
        }

        self.draw_2d_viewmodel(weapon, stats);
    }

    /// Select which punch frame to show based on how far through the animation we are.
    fn punch_frame_key(&self) -> &'static str {
        let total = crate::weapons::stats::melee::KNIFE_LIGHT_ANIM_DURATION;
        let elapsed = (total - self.knife_attack_lock_timer).max(0.0);
        let progress = (elapsed / total).clamp(0.0, 1.0);
        if progress < 0.33 {
            "punch1"
        } else if progress < 0.66 {
            "punch2"
        } else {
            "punch3"
        }
    }

    fn draw_3d_viewmodel(
        &self,
        mesh: &Mesh,
        mesh_normalize_scale: f32,
        mesh_y_offset: f32,
        hands_anim: Option<&crate::types::animated_model::AnimatedModel>,
        hands_static: Option<&Mesh>,
        weapon: &crate::weapons::WeaponState,
        stats: &crate::weapons::stats::WeaponStats,
        should_draw_hands: bool,
        first_mesh: bool,
    ) {
        let force_ads_preview = self.dev_mode.is_unlocked() && self.dev_mode.force_ads;
        if stats.crosshair_style == CrosshairStyle::Scope
            && weapon.ads_progress > 0.9
            && !force_ads_preview
        {
            return;
        }

        let (base_x, base_y, base_z) = stats.viewmodel_position;
        let (rot_pitch, rot_yaw, rot_roll) = stats.viewmodel_rotation;
        let inspect_pos = Vec3::ZERO;
        let inspect_rot = Vec3::ZERO;
        // camera and projection setup
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

        let ads_p = if self.dev_mode.is_unlocked() && self.dev_mode.force_ads {
            1.0
        } else {
            weapon.ads_progress
        };

        // Pair offset only applies during hip-fire — fades to zero as ADS comes in
        let pair_scale = 1.0 - ads_p;
        let pair_pos = vec3(
            weapon.pair_offset_pos.0 * pair_scale,
            weapon.pair_offset_pos.1 * pair_scale,
            weapon.pair_offset_pos.2 * pair_scale,
        );
        let pair_rot = (
            weapon.pair_offset_rot.0 * pair_scale,
            weapon.pair_offset_rot.1 * pair_scale,
            weapon.pair_offset_rot.2 * pair_scale,
        );
        let ads_rot = weapon.ads_rot_offset;
        let ads_hands_rot = weapon.ads_hands_rot_offset;
        let ads_hands_pos = vec3(
            weapon.ads_hands_pos_offset.0 * ads_p,
            weapon.ads_hands_pos_offset.1 * ads_p,
            weapon.ads_hands_pos_offset.2 * ads_p,
        );
        let hands_weapon_offset = vec3(
            stats.viewmodel_hands_offset.0,
            stats.viewmodel_hands_offset.1,
            stats.viewmodel_hands_offset.2,
        );

        let hands_base_anchor =
            vec3(-0.35, -0.25, 0.58) + hands_weapon_offset + pair_pos + ads_hands_pos;
        let weapon_base_anchor = vec3(-0.35, -0.25, 0.58);
        let weapon_local_offset = vec3(
            base_x + weapon.ads_pos_offset.0 * ads_p,
            base_y + weapon.ads_pos_offset.1 * ads_p,
            base_z + weapon.ads_pos_offset.2 * ads_p,
        ) + pair_pos;

        let mut motion_offset = Vec3::ZERO;
        if weapon.id.category() == WeaponCategory::Pistol
            && weapon.is_reloading
            && stats.reload_time > 0.0
        {
            let reload_fraction = (weapon.reload_timer / stats.reload_time).clamp(0.0, 1.0);
            motion_offset.y -= (reload_fraction * std::f32::consts::PI).sin() * 0.08;
        }

        let kick = weapon.muzzle_flash_timer / 0.08;
        motion_offset.z += kick * 0.02;
        motion_offset.y -= kick * 0.01;
        let hands_anchor = hands_base_anchor + motion_offset + inspect_pos;
        let weapon_anchor = weapon_base_anchor + motion_offset + inspect_pos;
        let pos = weapon_anchor + weapon_local_offset;
        let vm_scale = stats.viewmodel_scale
            + (stats.viewmodel_ads_scale - stats.viewmodel_scale) * ads_p;
        
        /*
            Normalize and transform weapon mesh to world position. We apply normalization first so that
            the viewmodel_scale stat can be used to tune overall weapon size without affecting the mesh's position offset.
        */
        let normalize_mat = Mat4::from_translation(vec3(0.0, mesh_y_offset, 0.0))
            * Mat4::from_scale(Vec3::splat(mesh_normalize_scale.max(0.0001)));
        let scale_mat = Mat4::from_scale(vec3(
            vm_scale * 28.0,
            vm_scale * 28.0,
            vm_scale * 28.0,
        ));
        let weapon_rot_mat = Mat4::from_euler(
            EulerRot::XYZ,
            rot_pitch + pair_rot.0 + ads_rot.0 * ads_p + inspect_rot.x,
            rot_yaw + pair_rot.1 + ads_rot.1 * ads_p + inspect_rot.y,
            rot_roll + pair_rot.2 + ads_rot.2 * ads_p + inspect_rot.z,
        );
        let trans_mat = Mat4::from_translation(pos);
        let transform = trans_mat * weapon_rot_mat * scale_mat * normalize_mat;

        gl_use_default_material();
        let gl = unsafe { get_internal_gl() }; // unsafe is required to access internal GL for custom shader uniforms
        gl.quad_gl.depth_test(true);

        // avoid allocating a transformed mesh every frame by using the GL model matrix
        gl.quad_gl.push_model_matrix(transform);
        draw_mesh(mesh);
        gl.quad_gl.pop_model_matrix();

        /*
            DRAWING ANIMATED HANDS HERE
        */

        let mut draw_hands = |hands_mesh: &Mesh, normalize_scale: f32, y_offset: f32| {
            // Keep hands scale independent from weapon scale so tiny weapon tuning
            // does not make hands disappear.
            let hands_scale = HANDS_BASE_SCALE * stats.viewmodel_hands_scale.max(0.01);
            let hands_scale_mat = Mat4::from_scale(Vec3::splat(hands_scale));
            let hands_trans = Mat4::from_translation(hands_anchor);
            let hands_rot_mat = Mat4::from_euler(
                EulerRot::XYZ,
                stats.viewmodel_hands_rotation.0
                    + pair_rot.0
                    + ads_hands_rot.0 * ads_p
                    + inspect_rot.x,
                stats.viewmodel_hands_rotation.1
                    + pair_rot.1
                    + ads_hands_rot.1 * ads_p
                    + inspect_rot.y,
                stats.viewmodel_hands_rotation.2
                    + pair_rot.2
                    + ads_hands_rot.2 * ads_p
                    + inspect_rot.z,
            );
            // Some FPS hand exports include hidden/far helper geometry that can poison
            // auto-normalization (very tiny normalize_scale), so clamp to a sane band.
            let clamped_normalize = normalize_scale.clamp(0.2, 5.0);
            let hands_normalize_mat = Mat4::from_translation(vec3(0.0, y_offset, 0.0))
                * Mat4::from_scale(Vec3::splat(clamped_normalize));
            let hands_transform =
                hands_trans * hands_rot_mat * hands_scale_mat * hands_normalize_mat;

            // Viewmodel hands should be fully opaque regardless of source material
            // alpha; only a mesh that is not needs its own copy.
            gl.quad_gl.push_model_matrix(hands_transform);
            if hands_mesh.vertices.iter().all(|v| v.color[3] == 255) {
                draw_mesh(hands_mesh);
            } else {
                let mut hands_verts = hands_mesh.vertices.clone();
                for v in &mut hands_verts {
                    v.color[3] = 255;
                }
                draw_mesh(&Mesh {
                    vertices: hands_verts,
                    indices: hands_mesh.indices.clone(),
                    texture: hands_mesh.texture.clone(),
                });
            }
            gl.quad_gl.pop_model_matrix();
        };

        if should_draw_hands && first_mesh {
            if let Some(hands_model) = hands_anim {
                for mesh in hands_model.meshes() {
                    draw_hands(mesh, hands_model.normalize_scale, hands_model.y_offset);
                }
            } else if let Some(hands_mesh) = hands_static {
                draw_hands(hands_mesh, 1.0, 0.0);
            }
        }

        let muzzle_local = vec3(
            stats.viewmodel_muzzle_offset.0,
            stats.viewmodel_muzzle_offset.1,
            stats.viewmodel_muzzle_offset.2,
        );
        let muzzle_world = transform.transform_point3(muzzle_local);

        gl.quad_gl.depth_test(true);

        set_default_camera();

        if weapon.muzzle_flash_timer > 0.0 && first_mesh {
            if let Some((fx, fy)) = Self::project_to_screen(&vm_camera, muzzle_world) {
                let t = (weapon.muzzle_flash_timer / 0.08).min(1.0);
                let alpha = (220.0 * t) as u8;
                let radius = 18.0 * vm_scale * 28.0 * (0.4 + 0.6 * t);

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
        let vm_scale = stats.viewmodel_scale
            + (stats.viewmodel_ads_scale - stats.viewmodel_scale) * weapon.ads_progress;
        let scale = (target_h / tex.height()).max(0.2) * vm_scale;
        let size = vec2(tex.width() * scale, tex.height() * scale);

        let hip_pos = vec2(screen_w * 0.68, screen_h * 0.88);
        let ads_pos = vec2(screen_w * 0.53, screen_h * 0.78);
        let pos = hip_pos.lerp(ads_pos, weapon.ads_progress);

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
