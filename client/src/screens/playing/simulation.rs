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
            // here we refresh the hitbox positions every frame based on the current animation pose of the player so hit detection is accurate if they crouch, prone, slide, etc.
            Self::refresh_player_hitboxes(animated_models, p);
        }
        for bot in &mut self.practice_bots {
            Self::refresh_player_hitboxes(animated_models, &mut bot.player);
        }
        self.practice_bots.retain(|bot| !bot.is_dead());
        if self.hit_marker_timer > 0.0 {
            self.hit_marker_timer = (self.hit_marker_timer - delta).max(0.0);
        }

        // Respawn feedback timer
        if self.spawn_feedback_timer > 0.0 {
            self.spawn_feedback_timer = (self.spawn_feedback_timer - delta).max(0.0);
            if self.spawn_feedback_timer <= 0.0 {
                self.spawn_feedback_message = None;
            }
        }

        // Lobby console feedback timer
        if self.lobby_console_message_timer > 0.0 {
            self.lobby_console_message_timer = (self.lobby_console_message_timer - delta).max(0.0);
            if self.lobby_console_message_timer <= 0.0 {
                self.lobby_console_message = None;
            }
        }

        // Match-start countdown (server-initiated)
        if self.match_start_countdown_timer > 0.0 {
            self.match_start_countdown_timer = (self.match_start_countdown_timer - delta).max(0.0);
            if self.match_start_countdown_timer <= 0.0 {
                // small 'Go' announcement
                self.announcements.push(("GO!".to_string(), 1.5));
            }
        }

        // Match-end countdown (server-initiated)
        if self.match_end_countdown_timer > 0.0 {
            self.match_end_countdown_timer = (self.match_end_countdown_timer - delta).max(0.0);
            if self.match_end_countdown_timer <= 0.0 {
                // clear winner toast once countdown completes
                self.match_end_winner = None;
            }
        }

        // Announcements timers
        for ann in &mut self.announcements {
            ann.1 = (ann.1 - delta).max(0.0);
        }
        self.announcements.retain(|ann| ann.1 > 0.0);

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

    /*
        this is the function that actually updates the players hitbox
        The way it works is by finding the animated model for the player based on their skin, then finds the position if important joints and builds hitboxes based on these
        it is possible due to the skinning system and the fact that we cache joint positions every frame in the animated model.
    */

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

        /*
            this is the scale factor for the hitbox positions, it is based on the normalize scale of the animation (which is based on the original model size)
            and a tunable render scale factor to adjust how big the hitboxes are overall
            makes it easy to fine tune without needing to adjust the original model or animation.
            We also apply the same scale factor to the y_offset which is used to adjust the hitbox positions up or down based on the players height (crouching, standing, etc)
        */
        let scale = anim.normalize_scale * WORLD_PLAYER_RENDER_SCALE;
        let y_offset = anim.y_offset * WORLD_PLAYER_RENDER_SCALE;
        if let Some(hitboxes) =
            Self::build_standing_hitboxes_from_joints(anim, player.render_yaw, scale, y_offset)
        {
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
        /*
            Here we are using the same model scaling as the render so visual hitbox indicators line up with the actual hitboxes, and we also apply the same scaling to
            the y_offset which is used to adjust the hitbox positions up or down based on the players height (crouching, standing, etc)
        */
        let to_offset = |p: Vec3| -> Vec3 {
            let rotated = Mat3::from_rotation_y(-yaw + PLAYER_MODEL_YAW_OFFSET) * (p * scale);
            rotated + vec3(0.0, y_offset, 0.0)
        };

        let head = any_joint(anim, &[&["head"], &["neck", "head"]])?;
        let chest = any_joint(anim, &[&["spine2"], &["chest"], &["spine1"]])?;
        let hips = any_joint(anim, &[&["hips"], &["pelvis"]])?;

        let l_foot = any_joint(anim, &[&["left", "foot"], &["left", "toe"]]).unwrap_or(hips);
        let r_foot = any_joint(anim, &[&["right", "foot"], &["right", "toe"]]).unwrap_or(hips);
        let l_shoulder =
            any_joint(anim, &[&["left", "shoulder"], &["left", "arm"]]).unwrap_or(chest);
        let r_shoulder =
            any_joint(anim, &[&["right", "shoulder"], &["right", "arm"]]).unwrap_or(chest);
        let l_elbow = any_joint(
            anim,
            &[
                &["left", "forearm"],
                &["left", "lowerarm"],
                &["left", "elbow"],
            ],
        )
        .unwrap_or(l_shoulder);
        let r_elbow = any_joint(
            anim,
            &[
                &["right", "forearm"],
                &["right", "lowerarm"],
                &["right", "elbow"],
            ],
        )
        .unwrap_or(r_shoulder);
        let l_hand = any_joint(anim, &[&["left", "hand"], &["left", "wrist"]]).unwrap_or(l_elbow);
        let r_hand = any_joint(anim, &[&["right", "hand"], &["right", "wrist"]]).unwrap_or(r_elbow);

        let head_o = to_offset(head);
        let chest_o = to_offset(chest);
        let hips_o = to_offset(hips);
        let l_foot_o = to_offset(l_foot);
        let r_foot_o = to_offset(r_foot);
        let l_shoulder_o = to_offset(l_shoulder);
        let r_shoulder_o = to_offset(r_shoulder);
        let l_elbow_o = to_offset(l_elbow);
        let r_elbow_o = to_offset(r_elbow);
        let l_hand_o = to_offset(l_hand);
        let r_hand_o = to_offset(r_hand);

        /*
            Here we are bulding AABB hitboxes from a bone segment (hand, leg, etc) then we also add padding so it works for each state (stand, crouch, etc)
            and we also set a minimum size so hitboxes dont get too small if the model has very thin arms or legs.
        */
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

        let mut hitboxes = vec![
            Hitbox {
                center_offset: head_center,
                half_extents: vec3(head_radius, head_radius, head_radius),
                zone: HitZone::Head,
            },
            segment_box(
                chest_o,
                hips_o,
                vec3(0.08, 0.08, 0.08),
                torso_min,
                HitZone::Torso,
            ),
            segment_box(
                l_shoulder_o,
                hips_o,
                vec3(0.05, 0.05, 0.05),
                torso_min * 0.8,
                HitZone::Torso,
            ),
            segment_box(
                r_shoulder_o,
                hips_o,
                vec3(0.05, 0.05, 0.05),
                torso_min * 0.8,
                HitZone::Torso,
            ),
            segment_box(
                hips_o,
                l_foot_o,
                vec3(0.05, 0.05, 0.05),
                limb_min,
                HitZone::Limbs,
            ),
            segment_box(
                hips_o,
                r_foot_o,
                vec3(0.05, 0.05, 0.05),
                limb_min,
                HitZone::Limbs,
            ),
        ];

        let arm_min = vec3(0.08, 0.08, 0.08);
        let arm_pad = vec3(0.04, 0.04, 0.04);
        /*
            For each segment we creeated earlier, we push into the hitbox list by creating an AABB that encapsulates all of it
        */
        hitboxes.push(segment_box(
            l_shoulder_o,
            l_elbow_o,
            arm_pad,
            arm_min,
            HitZone::Limbs,
        ));
        hitboxes.push(segment_box(
            l_elbow_o,
            l_hand_o,
            arm_pad,
            arm_min,
            HitZone::Limbs,
        ));
        hitboxes.push(segment_box(
            r_shoulder_o,
            r_elbow_o,
            arm_pad,
            arm_min,
            HitZone::Limbs,
        ));
        hitboxes.push(segment_box(
            r_elbow_o,
            r_hand_o,
            arm_pad,
            arm_min,
            HitZone::Limbs,
        ));

        Some(hitboxes)
    }
}
