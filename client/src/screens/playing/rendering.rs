use crate::weapons::combat::HitZone;
use crate::weapons::stats::CrosshairStyle;
use macroquad::models::{ Mesh, Vertex };
use macroquad::prelude::*;
use macroquad::window::get_internal_gl;

use super::PlayingState;

impl PlayingState {
    pub(super) fn draw_other_players(&self) {
        for p in &self.other_players {
            self.draw_player_model(p);
        }

        for bot in &self.practice_bots {
            self.draw_player_model(&bot.player);
        }
    }

    fn draw_player_model(&self, p: &crate::types::player::OtherPlayer) {
        let desired_key = p.skin_type.model_key();

        // Try animated model first, then static, then fallback to rifle_idle
        let anim_model = desired_key
            .and_then(|k| self.animated_models.get(k))
            .or_else(|| self.animated_models.get("rifle_idle"));
        let static_mesh = desired_key
            .and_then(|k| self.models.get(k))
            .or_else(|| self.models.get("rifle_idle"));

        if p.skin_type.is_model_based() {
            if let Some(anim) = anim_model {
                let s = anim.normalize_scale;
                let transform =
                    Mat4::from_translation(vec3(p.x, p.y + anim.y_offset, p.z)) *
                    Mat4::from_rotation_y(-p.yaw) *
                    Mat4::from_scale(vec3(s, s, s));
                let gl = unsafe { get_internal_gl() };
                gl.quad_gl.push_model_matrix(transform);
                draw_mesh(anim.mesh());
                gl.quad_gl.pop_model_matrix();
                return;
            }
            if let Some(mesh) = static_mesh {
                let transform =
                    Mat4::from_translation(vec3(p.x, p.y, p.z)) *
                    Mat4::from_rotation_y(-p.yaw);
                let gl = unsafe { get_internal_gl() };
                gl.quad_gl.push_model_matrix(transform);
                draw_mesh(mesh);
                gl.quad_gl.pop_model_matrix();
                return;
            }
        }

        // Fallback: colored cubes
        let color = if p.skin.is_some() { WHITE } else { p.color };
        draw_cube(vec3(p.x, p.y + 0.6, p.z), vec3(0.6, 1.2, 0.4), None, color);
        draw_cube(vec3(p.x, p.y + 1.4, p.z), vec3(0.4, 0.4, 0.4), None, color);
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

    pub(super) fn draw_entity_health_bars(&self, camera: &Camera3D) {
        if !self.dev_mode.show_health_bars {
            return;
        }

        let forward = (camera.target - camera.position).normalize();
        let right = forward.cross(camera.up).normalize();
        let up = right.cross(forward).normalize();
        let fov_rad = camera.fovy;
        let fov_scale = screen_height() / (2.0 * (fov_rad / 2.0).tan());

        // draw health bar for target dummy
        if self.current_world == crate::types::world::WorldId::Lobby {
            let world_pos = self.target_dummy.position + vec3(0.0, 2.0, 0.0);
            let rel = world_pos - camera.position;
            let dot = rel.dot(forward);
            if dot >= 0.1 {
                let sx = screen_width() / 2.0 + (rel.dot(right) / dot) * fov_scale;
                let sy = screen_height() / 2.0 - (rel.dot(up) / dot) * fov_scale;
                Self::draw_health_bar(
                    sx,
                    sy,
                    self.target_dummy.hp,
                    self.target_dummy.max_hp,
                    "Target Dummy"
                );
            }
        }

        // draw health bars for other players
        for p in &self.other_players {
            let world_pos = vec3(p.x, p.y + 2.0, p.z);
            let rel = world_pos - camera.position;
            let dot = rel.dot(forward);
            if dot >= 0.1 {
                let sx = screen_width() / 2.0 + (rel.dot(right) / dot) * fov_scale;
                let sy = screen_height() / 2.0 - (rel.dot(up) / dot) * fov_scale;
                Self::draw_health_bar(sx, sy, p.hp, p.max_hp, &p.username);
            }
        }

        // draw health bars for practice bots
        for bot in &self.practice_bots {
            let p = &bot.player;
            let world_pos = vec3(p.x, p.y + 2.0, p.z);
            let rel = world_pos - camera.position;
            let dot = rel.dot(forward);
            if dot >= 0.1 {
                let sx = screen_width() / 2.0 + (rel.dot(right) / dot) * fov_scale;
                let sy = screen_height() / 2.0 - (rel.dot(up) / dot) * fov_scale;
                Self::draw_health_bar(sx, sy, p.hp, p.max_hp, &p.username);
            }
        }
    }

    fn draw_health_bar(sx: f32, sy: f32, hp: f32, max_hp: f32, label: &str) {
        use crate::ui::fonts::{ measure_game_text, text_params };

        let bar_w = 60.0;
        let bar_h = 6.0;
        let hp_frac = (hp / max_hp).clamp(0.0, 1.0);

        // background
        draw_rectangle(sx - bar_w / 2.0, sy, bar_w, bar_h, Color::from_rgba(40, 40, 40, 200));

        // health fill
        let hp_color = if hp_frac > 0.5 {
            Color::from_rgba(80, 200, 80, 255)
        } else if hp_frac > 0.25 {
            Color::from_rgba(220, 180, 40, 255)
        } else {
            RED
        };
        draw_rectangle(sx - bar_w / 2.0, sy, bar_w * hp_frac, bar_h, hp_color);

        // border
        draw_rectangle_lines(sx - bar_w / 2.0, sy, bar_w, bar_h, 1.0, WHITE);

        // label
        let label_w = measure_game_text(label, 12);
        draw_text_ex(label, sx - label_w / 2.0, sy - 4.0, text_params(12, WHITE));
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

    pub(super) fn draw_viewmodel(&self) {
        let weapon = self.weapons.active();
        let stats = weapon.stats();

        // When fully scoped, the sniper overlay fills the screen; hide the gun model to avoid overlap.
        if matches!(stats.crosshair_style, CrosshairStyle::Scope) && weapon.ads_progress > 0.85 {
            return;
        }

        // Debug output
        if let Some(model_key) = stats.viewmodel_model {
            if let Some(mesh) = self.models.get(model_key) {
                self.draw_3d_viewmodel(mesh, weapon, stats);
                return;
            } else {
                // Model not loaded, fallback to 2D
                println!("Model '{}' not found in cache, using 2D fallback", model_key);
            }
        }

        // Fallback to 2D sprite rendering
        self.draw_2d_viewmodel(weapon, stats);
    }

    fn draw_3d_viewmodel(
        &self,
        mesh: &Mesh,
        weapon: &crate::weapons::WeaponState,
        stats: &crate::weapons::stats::WeaponStats
    ) {
        // Hide weapon model when fully scoped (sniper scope overlay replaces it)
        if stats.crosshair_style == CrosshairStyle::Scope && weapon.ads_progress > 0.9 {
            return;
        }

        // Separate viewmodel camera with wider FOV and very close near plane
        let vm_fov = (60.0f32).to_radians();
        let vm_camera = Camera3D {
            position: vec3(0.0, 0.0, 0.0),
            target: vec3(0.0, 0.0, 1.0),
            up: vec3(0.0, 1.0, 0.0),
            fovy: vm_fov,
            aspect: Some(screen_width() / screen_height()),
            projection: Projection::Perspective,
            render_target: None,
            viewport: None,
        };

        set_camera(&vm_camera);

        let (base_x, base_y, base_z) = stats.viewmodel_position;
        let (rot_pitch, rot_yaw, rot_roll) = stats.viewmodel_rotation;

        // Hip to ADS positioning (slide inward and up when aiming)
        let ads_offset_x = 0.08 * weapon.ads_progress;
        let ads_offset_y = 0.03 * weapon.ads_progress;
        let mut pos = vec3(
            -0.35 + base_x + ads_offset_x,
            -0.25 + base_y + ads_offset_y,
            0.58 + base_z
        );

        // Weapon bob synced to movement
        let horizontal_speed = vec2(
            self.movement_state.velocity.x,
            self.movement_state.velocity.z
        ).length();
        if horizontal_speed > 0.1 && self.movement_state.is_grounded {
            let t = self.bob_timer;
            pos.x += t.sin() * 0.008;
            pos.y += t.cos() * 0.006;
        }

        // Reload dip
        if weapon.is_reloading {
            let reload_fraction = (weapon.reload_timer / stats.reload_time).clamp(0.0, 1.0);
            pos.y -= (reload_fraction * std::f32::consts::PI).sin() * 0.08;
        }

        // Recoil kick
        let kick = weapon.muzzle_flash_timer / 0.08;
        pos.z += kick * 0.02;
        pos.y -= kick * 0.01;

        // Build transform matrix
        let scale_mat = Mat4::from_scale(
            vec3(
                stats.viewmodel_scale * 28.0,
                stats.viewmodel_scale * 28.0,
                stats.viewmodel_scale * 28.0
            )
        );
        let rot_mat = Mat4::from_euler(EulerRot::XYZ, rot_pitch, rot_yaw, rot_roll);
        let trans_mat = Mat4::from_translation(pos);
        let transform = trans_mat * rot_mat * scale_mat;

        // Draw mesh - transform vertices manually since macroquad doesn't easily support model matrices
        gl_use_default_material();
        let gl = unsafe { get_internal_gl() };
        gl.quad_gl.depth_test(false);

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

        // Compute muzzle position in viewmodel space and project to screen for flash
        let muzzle_local = vec3(
            stats.viewmodel_muzzle_offset.0,
            stats.viewmodel_muzzle_offset.1,
            stats.viewmodel_muzzle_offset.2
        );
        let muzzle_world = transform.transform_point3(muzzle_local);

        gl.quad_gl.depth_test(true);

        // Reset to default camera for 2D overlays
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

        // auto-scale sprite to occupy a fraction of screen height, with per-weapon tuning
        let target_h = (screen_h * (0.46 - weapon.ads_progress * 0.08)).clamp(
            160.0,
            screen_h * 0.8
        );
        let scale = (target_h / tex.height()).max(0.2) * stats.viewmodel_scale;
        let size = vec2(tex.width() * scale, tex.height() * scale);

        // hip vs ADS placement (slide towards center when aiming)
        let hip_pos = vec2(screen_w * 0.68, screen_h * 0.88);
        let ads_pos = vec2(screen_w * 0.53, screen_h * 0.78);
        let mut pos = hip_pos.lerp(ads_pos, weapon.ads_progress);

        // light bob synced to movement
        let horizontal_speed = vec2(
            self.movement_state.velocity.x,
            self.movement_state.velocity.z
        ).length();
        if horizontal_speed > 0.1 && self.movement_state.is_grounded {
            let t = self.bob_timer;
            pos.x += t.sin() * 5.0;
            pos.y += t.cos() * 3.0;
        }

        // subtle reload dip
        if weapon.is_reloading {
            let reload_fraction = (weapon.reload_timer / weapon.stats().reload_time).clamp(
                0.0,
                1.0
            );
            pos.y += (reload_fraction * std::f32::consts::PI).sin() * 12.0;
        }

        // Position uses bottom-right anchor for gun sprites (typical FPS positioning)
        let draw_pos = vec2(pos.x - size.x, pos.y - size.y);

        draw_texture_ex(tex, draw_pos.x, draw_pos.y, WHITE, DrawTextureParams {
            dest_size: Some(size),
            ..Default::default()
        });

        // simple muzzle flash overlay anchored near the sprite's muzzle
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

