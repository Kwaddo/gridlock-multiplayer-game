use crate::core::GameConfig;
use crate::physics::collision;
use crate::types::animated_model::AnimatedModel;
use crate::types::maze::Maze;
use crate::types::player::OtherPlayer;
use crate::types::world::WorldId;
use crate::weapons::combat::{default_player_hitboxes, CombatEntity, HitZone, Hitbox};

use macroquad::prelude::*;
use std::collections::HashMap;

use super::PlayingState;

const WORLD_PLAYER_RENDER_SCALE: f32 = 0.90;
const PLAYER_MODEL_YAW_OFFSET: f32 = std::f32::consts::FRAC_PI_2;

impl PlayingState {
    pub fn simulate(&mut self, config: &GameConfig, maze: Option<&Maze>) {
        let raw_delta = get_frame_time();
        if raw_delta > 0.20 {
            self.move_dir = Vec3::ZERO;
            self.jump_requested = false;
            self.just_jumped = false;
            self.just_landed = false;
            self.movement_state.velocity = Vec3::ZERO;
            return;
        }

        let delta = raw_delta.min(0.05);
        self.just_jumped = false;
        self.just_landed = false;

        for entry in &mut self.kill_feed {
            entry.timer -= delta;
        }
        self.kill_feed.retain(|entry| entry.timer > 0.0);

        if self.is_dead {
            if self.death_timer > 0.0 {
                self.death_timer -= delta;
            } else if self.current_world == WorldId::Lobby && is_key_pressed(KeyCode::Space) {
                self.pending_respawn = true;
            }
            return;
        }

        self.weapons.update(delta);
        for model in self.animated_models.values_mut() {
            model.update(delta);
        }

        self.target_dummy.update(delta);
        for p in &mut self.other_players {
            p.update(delta);
        }
        let local_pos = Some(self.position);
        for bot in &mut self.practice_bots {
            bot.simulate_yaw(delta, local_pos);
            bot.update(delta);
        }
        let animated_models = &self.animated_models;
        for p in &mut self.other_players {
            Self::refresh_player_hitboxes(animated_models, p);
        }
        for bot in &mut self.practice_bots {
            Self::refresh_player_hitboxes(animated_models, &mut bot.player);
        }
        self.practice_bots.retain(|bot| !bot.is_dead());
        if self.hit_marker_timer > 0.0 {
            self.hit_marker_timer = (self.hit_marker_timer - delta).max(0.0);
        }

        for dn in &mut self.damage_numbers {
            dn.lifetime -= delta;
            dn.screen_y -= 60.0 * delta;
        }
        self.damage_numbers.retain(|dn| dn.lifetime > 0.0);

        for t in &mut self.tracers {
            if t.progress < 1.0 {
                let dist = (t.end - t.start).length();
                t.progress = (t.progress + (t.speed * delta) / dist).min(1.0);
            }
            t.lifetime -= delta;
        }
        self.tracers.retain(|t| t.lifetime > 0.0);

        self.update_stamina(config, delta);
        self.update_crouch_height(config, delta);
        let was_grounded = self.movement_state.is_grounded;
        self.apply_jump();
        self.apply_gravity(delta);

        match self.current_world {
            WorldId::Lobby => {
                collision::resolve_vertical(
                    &mut self.position,
                    &mut self.movement_state.velocity,
                    self.movement_state.height,
                    &mut self.movement_state.is_grounded,
                );
                self.apply_movement(delta);
                collision::resolve_horizontal(
                    &mut self.position,
                    &mut self.movement_state.velocity,
                    self.movement_state.height,
                );
            }
            WorldId::Maze => {
                if self.position.y <= 0.0 {
                    self.position.y = 0.0;
                    self.movement_state.is_grounded = true;
                    self.movement_state.velocity.y = 0.0;
                }
                let prev_position = self.position;
                self.apply_movement(delta);
                if let Some(m) = maze {
                    let clamped =
                        m.clamp_move(prev_position, self.position, collision::PLAYER_RADIUS);
                    if (clamped.x - self.position.x).abs() > f32::EPSILON {
                        self.movement_state.velocity.x = 0.0;
                    }
                    if (clamped.z - self.position.z).abs() > f32::EPSILON {
                        self.movement_state.velocity.z = 0.0;
                    }
                    self.position = clamped;
                }
            }
        }

        if !was_grounded && self.movement_state.is_grounded {
            self.just_landed = true;
        }
    }

    fn refresh_player_hitboxes(
        animated_models: &HashMap<String, AnimatedModel>,
        player: &mut OtherPlayer,
    ) {
        let desired_key = player.skin_type.model_key();
        let anim = desired_key
            .and_then(|k| animated_models.get(k))
            .or_else(|| animated_models.get("anim_rifle_idle"));

        let Some(anim) = anim else {
            player.hitboxes = default_player_hitboxes();
            return;
        };

        let scale = anim.normalize_scale * WORLD_PLAYER_RENDER_SCALE;
        let y_offset = anim.y_offset * WORLD_PLAYER_RENDER_SCALE;
        if let Some(hitboxes) = Self::build_standing_hitboxes_from_joints(anim, player.render_yaw, scale, y_offset) {
            player.hitboxes = hitboxes;
        } else {
            player.hitboxes = default_player_hitboxes();
        }
    }

    fn build_standing_hitboxes_from_joints(
        anim: &AnimatedModel,
        yaw: f32,
        scale: f32,
        y_offset: f32,
    ) -> Option<Vec<Hitbox>> {
        fn any_joint(anim: &AnimatedModel, patterns: &[&[&str]]) -> Option<Vec3> {
            patterns
                .iter()
                .find_map(|tokens| anim.joint_position_with_tokens(tokens))
        }

        let to_offset = |p: Vec3| -> Vec3 {
            let rotated = Mat3::from_rotation_y(-yaw + PLAYER_MODEL_YAW_OFFSET) * (p * scale);
            rotated + vec3(0.0, y_offset, 0.0)
        };

        let head = any_joint(anim, &[&["head"], &["neck", "head"]])?;
        let chest = any_joint(anim, &[&["spine2"], &["chest"], &["spine1"]])?;
        let hips = any_joint(anim, &[&["hips"], &["pelvis"]])?;

        let l_foot = any_joint(anim, &[&["left", "foot"], &["left", "toe"]]).unwrap_or(hips);
        let r_foot = any_joint(anim, &[&["right", "foot"], &["right", "toe"]]).unwrap_or(hips);
        let l_shoulder = any_joint(anim, &[&["left", "shoulder"], &["left", "arm"]]).unwrap_or(chest);
        let r_shoulder = any_joint(anim, &[&["right", "shoulder"], &["right", "arm"]]).unwrap_or(chest);

        let head_o = to_offset(head);
        let chest_o = to_offset(chest);
        let hips_o = to_offset(hips);
        let l_foot_o = to_offset(l_foot);
        let r_foot_o = to_offset(r_foot);
        let l_shoulder_o = to_offset(l_shoulder);
        let r_shoulder_o = to_offset(r_shoulder);

        // Segment-driven AABB approximation works across standing/crouch/prone/death poses.
        // It avoids relying on Y-only "height" deltas, which collapse on horizontal animations.
        let segment_box = |a: Vec3, b: Vec3, pad: Vec3, min_half: Vec3, zone: HitZone| -> Hitbox {
            let center = (a + b) * 0.5;
            let axis_half = (b - a).abs() * 0.5;
            let half = vec3(
                (axis_half.x + pad.x).max(min_half.x),
                (axis_half.y + pad.y).max(min_half.y),
                (axis_half.z + pad.z).max(min_half.z),
            );
            Hitbox {
                center_offset: center,
                half_extents: half,
                zone,
            }
        };

        let shoulder_width = (l_shoulder_o - r_shoulder_o).length().clamp(0.25, 0.65);
        let head_radius = ((head_o - chest_o).length() * 0.40).clamp(0.12, 0.20);
        let torso_min = vec3(shoulder_width * 0.24, 0.12, shoulder_width * 0.20);
        let limb_min = vec3(0.10, 0.10, 0.10);
        let head_up = (head_o - chest_o).normalize_or_zero();
        let head_center = head_o + head_up * (head_radius * 0.25);

        Some(vec![
            Hitbox {
                center_offset: head_center,
                half_extents: vec3(head_radius, head_radius, head_radius),
                zone: HitZone::Head,
            },
            segment_box(chest_o, hips_o, vec3(0.08, 0.08, 0.08), torso_min, HitZone::Torso),
            segment_box(l_shoulder_o, hips_o, vec3(0.05, 0.05, 0.05), torso_min * 0.8, HitZone::Torso),
            segment_box(r_shoulder_o, hips_o, vec3(0.05, 0.05, 0.05), torso_min * 0.8, HitZone::Torso),
            segment_box(hips_o, l_foot_o, vec3(0.05, 0.05, 0.05), limb_min, HitZone::Limbs),
            segment_box(hips_o, r_foot_o, vec3(0.05, 0.05, 0.05), limb_min, HitZone::Limbs),
        ])
    }
}
