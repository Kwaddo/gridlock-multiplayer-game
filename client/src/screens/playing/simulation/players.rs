use std::collections::HashMap;

use crate::types::animated_model::AnimatedModel;
use crate::types::player::OtherPlayer;
use crate::types::player_anim::AnimInput;
use crate::core::GameConfig;
use crate::weapons::combat::CombatEntity;

use macroquad::prelude::*;

use super::hitboxes::refresh_player_hitboxes;
use super::PlayingState;

impl PlayingState {
    /// Other players and practice bots: movement, proximity footstep/slide
    /// sounds (placed around the listener, muffled through walls), the shared
    /// slide clip, and their pose-driven hitboxes.
    pub(super) fn update_remote_players(&mut self, delta: f32, _config: &GameConfig) {
        for p in &mut self.other_players {
            p.update(delta);
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
