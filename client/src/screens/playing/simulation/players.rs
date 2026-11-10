use std::collections::HashMap;

use crate::types::animated_model::AnimatedModel;
use crate::types::player::OtherPlayer;
use crate::types::player_anim::AnimInput;
use crate::core::GameConfig;
use crate::types::maze::Maze;
use crate::weapons::combat::CombatEntity;

use macroquad::prelude::*;

use super::constants::{
    FOOTSTEP_INTERVAL_CROUCH, FOOTSTEP_INTERVAL_SPRINT, FOOTSTEP_INTERVAL_WALK,
    FOOTSTEP_VOLUME_CROUCH, FOOTSTEP_VOLUME_SPRINT, FOOTSTEP_VOLUME_WALK, SLIDE_SOUND_VOLUME,
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
            // how far the drawn body moved this frame, for the sounds' velocity
            // before this it read prev_x/prev_z which were never set (always 0), so the velocity was garbage
            let before = p.render_position();
            p.update(delta);
            let vel = (p.render_position() - before) * vec3(1.0, 0.0, 1.0) / delta.max(0.001);

            // skip dead players
            if p.hp <= 0.0 {
                continue;
            }

            // determine movement state for footstep sounds
            // other players slow walking are silent too (inferred from their speed)
            let is_moving = p.last_move_dir != crate::types::animation_state::MoveDir::None && !p.anim.slow_walking();
            let is_sliding = p.is_sliding;

            if is_sliding {
                // slide sound plays once at start (timer acts as "played" flag)
                if p.footstep_timer <= 0.0 {
                    let pos = p.render_position();
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
        // slides need no special handling anymore, each body starts its own slide at 0 (see PlayerAnim)
        // before this the one shared slide clip restarted only when nobody was sliding,
        // so a second slider joined the first one halfway through

        let animated_models = &self.animated_models;
        for p in &mut self.other_players {
            // move this body's own clock and work out its own pose, before this every body read the shared clip's clock
            tick_body(p, delta, animated_models);
            // here we refresh the hitbox positions every frame based on the current animation pose of the player so hit detection is accurate if they crouch, prone, slide, etc.
            refresh_player_hitboxes(animated_models, p);
        }
        for bot in &mut self.practice_bots {
            tick_body(&mut bot.player, delta, animated_models);
            refresh_player_hitboxes(animated_models, &mut bot.player);
        }
        self.practice_bots
            .retain(|bot: &crate::types::ai::simulated_player::SimulatedPlayer| !bot.is_dead());
    }
}

// one body's animation tick, players and bots alike, the shot flag is taken so one shot is one kick
// before this the input was built in 4 places and the bot copies read fired without clearing it
pub(super) fn tick_body(p: &mut OtherPlayer, dt: f32, models: &HashMap<String, AnimatedModel>) {
    let input = AnimInput {
        skin: p.skin_type,
        pos: p.render_position(),
        pitch: p.render_pitch,
        yaw: p.render_yaw,
        aiming: p.is_ads,
        crouching: p.is_crouching,
        fired: std::mem::take(&mut p.fired),
    };
    p.anim.tick(&input, dt, models);
}
