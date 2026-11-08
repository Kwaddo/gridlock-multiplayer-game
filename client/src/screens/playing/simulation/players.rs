use crate::core::GameConfig;
use crate::types::maze::Maze;
use crate::types::player::PlayerSkin;
use crate::weapons::combat::CombatEntity;

use macroquad::prelude::*;

use super::constants::{
    FOOTSTEP_INTERVAL_CROUCH, FOOTSTEP_INTERVAL_SPRINT, FOOTSTEP_INTERVAL_WALK,
    FOOTSTEP_VOLUME_CROUCH, FOOTSTEP_VOLUME_SPRINT, FOOTSTEP_VOLUME_WALK, SLIDE_CLIP_END_SEC,
    SLIDE_CLIP_START_SEC, SLIDE_SOUND_VOLUME,
};
use super::hitboxes::refresh_player_hitboxes;
use super::PlayingState;

impl PlayingState {
    /// Other players and practice bots: movement, proximity footstep/slide
    /// sounds (placed around the listener, muffled through walls), the shared
    /// slide clip, and their pose-driven hitboxes.
    pub(super) fn update_remote_players(&mut self, delta: f32, _config: &GameConfig, maze: Option<&Maze>) {
        // update other players and collect footstep sounds to play
        let mut footstep_sounds: Vec<(Vec3, Vec3, f32)> = Vec::new(); // (pos, vel, volume)
        let mut slide_sounds: Vec<(Vec3, Vec3)> = Vec::new();
        for p in &mut self.other_players {
            p.update(delta);

            // skip dead players
            if p.hp <= 0.0 {
                continue;
            }

            // determine movement state for footstep sounds
            let is_moving = p.last_move_dir != crate::types::animation_state::MoveDir::None;
            let is_sliding = p.is_sliding;

            if is_sliding {
                // slide sound plays once at start (timer acts as "played" flag)
                if p.footstep_timer <= 0.0 {
                    let pos = p.render_position();
                    let vel = vec3(p.render_x - p.prev_x, 0.0, p.render_z - p.prev_z) / delta.max(0.001);
                    slide_sounds.push((pos, vel));
                    p.footstep_timer = 1.0; // prevent re-triggering during same slide
                }
            } else if is_moving {
                p.footstep_timer -= delta;
                if p.footstep_timer <= 0.0 {
                    let (interval, volume) = if p.is_sprinting {
                        (FOOTSTEP_INTERVAL_SPRINT, FOOTSTEP_VOLUME_SPRINT)
                    } else if p.is_crouching {
                        (FOOTSTEP_INTERVAL_CROUCH, FOOTSTEP_VOLUME_CROUCH)
                    } else {
                        (FOOTSTEP_INTERVAL_WALK, FOOTSTEP_VOLUME_WALK)
                    };
                    let pos = p.render_position();
                    let vel = vec3(p.render_x - p.prev_x, 0.0, p.render_z - p.prev_z) / delta.max(0.001);
                    footstep_sounds.push((pos, vel, volume));
                    p.footstep_timer = interval;
                }
            } else {
                // not moving — reset timer so first step plays immediately on next move
                p.footstep_timer = 0.0;
            }
        }

        // play collected footstep sounds (outside mutable borrow)
        for (pos, vel, volume) in footstep_sounds {
            self.play_spatial_sound("footstep", pos, vel, volume, maze);
        }
        for (pos, vel) in slide_sounds {
            self.play_spatial_sound("slide", pos, vel, SLIDE_SOUND_VOLUME, maze);
        }

        let local_pos = Some(self.position);
        for bot in &mut self.practice_bots {
            bot.simulate_yaw(delta, local_pos);
            bot.update(delta);
        }
        let any_slide_skin_active =
            self.other_players.iter().any(|p| p.skin_type == PlayerSkin::Slide) ||
            self.practice_bots.iter().any(|b| b.player.skin_type == PlayerSkin::Slide);
        if let Some(slide_anim) = self.animated_models.get_mut("anim_slide") {
            if any_slide_skin_active && !self.prev_any_slide_anim_active {
                slide_anim.play_segment(SLIDE_CLIP_START_SEC, SLIDE_CLIP_END_SEC, false);
            } else if !any_slide_skin_active {
                // Reset to frame 0 when no one is sliding, so the next slide starts cleanly.
                slide_anim.stop_at_start();
            }
        }
        self.prev_any_slide_anim_active = any_slide_skin_active;

        let animated_models = &self.animated_models;
        for p in &mut self.other_players {
            // here we refresh the hitbox positions every frame based on the current animation pose of the player so hit detection is accurate if they crouch, prone, slide, etc.
            refresh_player_hitboxes(animated_models, p);
        }
        for bot in &mut self.practice_bots {
            refresh_player_hitboxes(animated_models, &mut bot.player);
        }
        self.practice_bots
            .retain(|bot: &crate::types::ai::simulated_player::SimulatedPlayer| !bot.is_dead());
    }
}
