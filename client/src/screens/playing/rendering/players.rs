use crate::weapons::combat::HitZone;

use macroquad::prelude::*;
use macroquad::window::get_internal_gl;

use super::super::PlayingState;

const WORLD_PLAYER_RENDER_SCALE: f32 = 0.90;
// Mixamo-style rigs are authored facing +Z, while gameplay yaw uses +X as forward.
// Keep this offset centralized so yaw logic and movement math stay in gameplay space.
const PLAYER_MODEL_YAW_OFFSET: f32 = std::f32::consts::FRAC_PI_2;
// additional yaw (radians) to rotate world weapons rightwards
const WORLD_WEAPON_YAW_ADJUST: f32 = std::f32::consts::PI / 2.0; // 45° right
                                                                 // additional forward translation in world space to push guns away from hand
const WORLD_WEAPON_FORWARD_OFFSET: f32 = 0.1; // units, tweak for desired spacing

impl PlayingState {
    #[inline(always)]
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
        let static_mesh_list = desired_static.or_else(|| self.models.get("anim_rifle_idle"));

        // draw the player body; remember whether we actually rendered a model so
        // we can fall back to cubes and still attach a weapon later.
        let mut drawn_body = false;
        // we also keep some variables around for hand offset calculation
        let mut hand_world_pos: Option<Vec3> = None;
        // these are only valid when the animated model branch runs
        let mut anim_scale = WORLD_PLAYER_RENDER_SCALE;
        let mut _anim_y_offset = 0.0f32;

        if p.skin_type.is_model_based() {
            if let Some(anim) = anim_model {
                drawn_body = true;
                let s = anim.normalize_scale * WORLD_PLAYER_RENDER_SCALE;
                anim_scale = s;
                let y_offset = anim.y_offset * WORLD_PLAYER_RENDER_SCALE;
                _anim_y_offset = y_offset;
                let transform = Mat4::from_translation(render_pos + vec3(0.0, y_offset, 0.0))
                    * Mat4::from_rotation_y(-render_yaw + PLAYER_MODEL_YAW_OFFSET)
                    * Mat4::from_scale(vec3(s, s, s));
                let gl = unsafe { get_internal_gl() };
                gl.quad_gl.push_model_matrix(transform);
                // Tint model by player color when a skin texture isn't used.
                // Clone vertices and modulate vertex colors by `p.color` so
                // textured and untextured meshes both receive the tint.
                for base_mesh in anim.meshes() {
                    if p.color != WHITE {
                        let mut verts = base_mesh.vertices.clone();
                        for v in &mut verts {
                            // v.color is [u8;4] in macroquad Vertex
                            let r = (v.color[0] as f32 / 255.0) * p.color.r;
                            let g = (v.color[1] as f32 / 255.0) * p.color.g;
                            let b = (v.color[2] as f32 / 255.0) * p.color.b;
                            let a = v.color[3];
                            v.color[0] = (r.clamp(0.0, 1.0) * 255.0) as u8;
                            v.color[1] = (g.clamp(0.0, 1.0) * 255.0) as u8;
                            v.color[2] = (b.clamp(0.0, 1.0) * 255.0) as u8;
                            v.color[3] = a;
                        }
                        let temp = macroquad::models::Mesh {
                            vertices: verts,
                            indices: base_mesh.indices.clone(),
                            texture: base_mesh.texture.clone(),
                        };
                        draw_mesh(&temp);
                    } else {
                        draw_mesh(base_mesh);
                    }
                }
                gl.quad_gl.pop_model_matrix();

                // compute hand pos from joint (set hand_world_pos later when
                // drawing weapon)
                if let Some(r_hand) = anim
                    .joint_position_with_tokens(&[&"right", &"hand"])
                    .or_else(|| anim.joint_position_with_tokens(&[&"right", &"wrist"]))
                {
                    let offset = Mat3::from_rotation_y(-render_yaw + PLAYER_MODEL_YAW_OFFSET)
                        * (r_hand * s)
                        + vec3(0.0, y_offset, 0.0);
                    hand_world_pos = Some(render_pos + offset);
                }
            } else if let Some(meshes) = static_mesh_list {
                drawn_body = true;
                let transform = Mat4::from_translation(render_pos)
                    * Mat4::from_rotation_y(-render_yaw + PLAYER_MODEL_YAW_OFFSET)
                    * Mat4::from_scale(vec3(
                        WORLD_PLAYER_RENDER_SCALE,
                        WORLD_PLAYER_RENDER_SCALE,
                        WORLD_PLAYER_RENDER_SCALE,
                    ));
                let gl = unsafe { get_internal_gl() };
                gl.quad_gl.push_model_matrix(transform);
                for mesh in meshes {
                    if p.color != WHITE {
                        let mut verts = mesh.vertices.clone();
                        for v in &mut verts {
                            let r = (v.color[0] as f32 / 255.0) * p.color.r;
                            let g = (v.color[1] as f32 / 255.0) * p.color.g;
                            let b = (v.color[2] as f32 / 255.0) * p.color.b;
                            let a = v.color[3];
                            v.color[0] = (r.clamp(0.0, 1.0) * 255.0) as u8;
                            v.color[1] = (g.clamp(0.0, 1.0) * 255.0) as u8;
                            v.color[2] = (b.clamp(0.0, 1.0) * 255.0) as u8;
                            v.color[3] = a;
                        }
                        let temp = macroquad::models::Mesh {
                            vertices: verts,
                            indices: mesh.indices.clone(),
                            texture: mesh.texture.clone(),
                        };
                        draw_mesh(&temp);
                    } else {
                        draw_mesh(mesh);
                    }
                }
                gl.quad_gl.pop_model_matrix();
            }
        }

        if !drawn_body {
            let color = if p.skin.is_some() { WHITE } else { p.color };
            draw_cube(
                render_pos + vec3(0.0, 0.6, 0.0),
                vec3(0.6, 1.2, 0.4),
                None,
                color,
            );
            draw_cube(
                render_pos + vec3(0.0, 1.4, 0.0),
                vec3(0.4, 0.4, 0.4),
                None,
                color,
            );
        }

        // === weapon rendering (runs regardless of body type) ===
        if hand_world_pos.is_none() {
            // no joint calculated earlier; use generic shoulder/hand offset
            let forward = vec3(render_yaw.cos(), 0.0, render_yaw.sin());
            let right = vec3(-forward.z, 0.0, forward.x);
            hand_world_pos =
                Some(render_pos + right * 0.3 * anim_scale + vec3(0.0, 1.0 * anim_scale, 0.0));
        }

        if let Some(weapon_id) = crate::weapons::stats::WeaponId::from_name(&p.current_weapon) {
            if let Some(model_key) = weapon_id.stats().viewmodel_model {
                // for world drawing we prefer a dedicated static model if one exists
                // (e.g. `fp_hands_minigun` → `minigun_model`). this keeps other
                // players from seeing our first‑person arms when wielding heavy
                // weapons.
                // compute an owned key string so that we can safely reference it
                // for the duration of this block. `world_key` will borrow from the
                // owned string via `as_str()` when used below.
                let world_key_owned: String = if model_key.starts_with("fp_hands_") {
                    let candidate = format!("{}_model", &model_key[9..]);
                    if self.models.contains_key(&candidate) {
                        candidate
                    } else {
                        model_key.to_string()
                    }
                } else {
                    model_key.to_string()
                };
                let world_key = world_key_owned.as_str();

                let wgl = unsafe { get_internal_gl() };
                if let Some(hand_world) = hand_world_pos {
                    // reduce overall model scale; sawnoff tends to come in huge so
                    // apply an additional shrink factor when its key is detected.
                    let mut scale_factor = anim_scale * 0.1;
                    if world_key.contains("sawnoff") {
                        scale_factor *= 0.1;
                    }

                    // rotate most guns slightly to the right when the player
                    // is moving (walking/sprinting) or aiming down sights. idle
                    // players should not have the yaw adjustment applied.
                    let mut yaw_rot = -render_yaw + PLAYER_MODEL_YAW_OFFSET;
                    let is_moving = p.is_sprinting
                        || p.is_crouching
                        || matches!(
                            p.skin_type,
                            crate::types::player::PlayerSkin::RifleWalk
                                | crate::types::player::PlayerSkin::WalkBack
                                | crate::types::player::PlayerSkin::WalkLeft
                                | crate::types::player::PlayerSkin::WalkRight
                                | crate::types::player::PlayerSkin::SprintForward
                                | crate::types::player::PlayerSkin::SprintLeft
                                | crate::types::player::PlayerSkin::SprintRight
                                | crate::types::player::PlayerSkin::CrouchForward
                                | crate::types::player::PlayerSkin::CrouchBack
                                | crate::types::player::PlayerSkin::CrouchLeft
                                | crate::types::player::PlayerSkin::CrouchRight
                        );
                    if is_moving || p.is_ads {
                        let adjust = if self.dev_mode.is_unlocked() {
                            // allow tweaking movement vs ADS separately
                            if is_moving {
                                self.dev_mode.weapon_yaw_adjust_move
                            } else if p.is_ads {
                                self.dev_mode.weapon_yaw_adjust_ads
                            } else {
                                self.dev_mode.weapon_rot_offset.1 // fallback
                            }
                        } else {
                            WORLD_WEAPON_YAW_ADJUST
                        };
                        yaw_rot -= adjust;
                    }
                    // exceptions that are already oriented correctly (hands-based)
                    if world_key.contains("sawnoff")
                        || world_key.contains("minigun")
                        || world_key.contains("knife")
                    {
                        yaw_rot = -render_yaw + PLAYER_MODEL_YAW_OFFSET;
                    }

                    // calculate weapon position with full 3D offset when dev mode is active
                    let forward_dir = vec3(render_yaw.cos(), 0.0, render_yaw.sin());
                    // calculate weapon position with offsets
                    // weapons that use hand animations shouldn't be moved at all – they
                    // already include their own translation in the glTF data. revert to
                    // the previous behavior by simply using hand_world in that case.
                    let weapon_pos = if world_key.contains("sawnoff")
                        || world_key.contains("minigun")
                        || world_key.contains("knife")
                    {
                        hand_world
                    } else {
                        // when the player is walking/running/aiming, use the smaller
                        // default offset instead of whatever the sliders are set to.
                        let (off_x, off_y, off_z) = if is_moving || p.is_ads {
                            (-0.03, -0.03, -0.01)
                        } else {
                            (
                                self.dev_mode.weapon_pos_offset.0,
                                self.dev_mode.weapon_pos_offset.1,
                                self.dev_mode.weapon_pos_offset.2,
                            )
                        };
                        let zval = if is_moving || p.is_ads { off_z } else { -off_z };
                        let right_dir = vec3(-forward_dir.z, 0.0, forward_dir.x);
                        let pos = hand_world
                            + forward_dir * zval
                            + right_dir * off_x
                            + vec3(0.0, 1.0, 0.0) * off_y;
                        let dyn_forward = if is_moving || p.is_ads {
                            WORLD_WEAPON_FORWARD_OFFSET
                        } else {
                            -0.05
                        };
                        pos + forward_dir * dyn_forward
                    };
                    let weapon_transform = if self.dev_mode.is_unlocked() {
                        let (rp, _, rr) = self.dev_mode.weapon_rot_offset;
                        Mat4::from_translation(weapon_pos)
                            * Mat4::from_rotation_y(yaw_rot)
                            * Mat4::from_rotation_x(rp)
                            * Mat4::from_rotation_z(rr)
                            * Mat4::from_scale(Vec3::splat(scale_factor))
                    } else {
                        Mat4::from_translation(weapon_pos)
                            * Mat4::from_rotation_y(yaw_rot)
                            * Mat4::from_scale(Vec3::splat(scale_factor))
                    };
                    wgl.quad_gl.push_model_matrix(weapon_transform);
                    if let Some(weapon_meshes) = self.models.get(world_key) {
                        for mesh in weapon_meshes {
                            draw_mesh(mesh);
                        }
                    } else if let Some(animated) = self.animated_models.get(world_key) {
                        for mesh in animated.meshes() {
                            draw_mesh(mesh);
                        }
                    }
                    wgl.quad_gl.pop_model_matrix();
                }
            }
        }
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
            let origin: Vec3 = p.render_position();
            Self::draw_entity_hitboxes(origin, &p.hitboxes);
        }
        for bot in &self.practice_bots {
            let origin: Vec3 = bot.player.render_position();
            Self::draw_entity_hitboxes(origin, &bot.player.hitboxes);
        }
    }

    /// Draw a corpse model at the death position during the death camera sequence.
    pub(crate) fn draw_death_corpse(&self) {
        use crate::screens::playing::state::DeathCameraPhase;

        let (corpse_pos, corpse_yaw) = match self.death_camera_phase {
            DeathCameraPhase::ZoomOut { corpse_pos, corpse_yaw, .. }
            | DeathCameraPhase::Watching { corpse_pos, corpse_yaw, .. } => (corpse_pos, corpse_yaw),
            _ => return,
        };

        // Determine death animation based on what stance we were in when we died
        let skin = if self.movement_state.is_prone {
            crate::types::player::PlayerSkin::ProneDeath
        } else if self.movement_state.is_crouching {
            crate::types::player::PlayerSkin::CrouchDeath
        } else {
            crate::types::player::PlayerSkin::StandDeath
        };

        // Create a temporary OtherPlayer to render the corpse
        let mut corpse = crate::types::player::OtherPlayer::new(
            u32::MAX, // dummy ID
            String::new(),
            self.current_world,
        );
        corpse.x = corpse_pos.x;
        corpse.y = corpse_pos.y;
        corpse.z = corpse_pos.z;
        corpse.render_x = corpse_pos.x;
        corpse.render_y = corpse_pos.y;
        corpse.render_z = corpse_pos.z;
        corpse.yaw = corpse_yaw;
        corpse.render_yaw = corpse_yaw;
        corpse.skin_type = skin;
        corpse.color = WHITE;

        self.draw_player_model(&corpse);
    }

    #[inline(always)]
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
