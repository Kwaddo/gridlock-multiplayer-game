use crate::types::player_anim::AnimInput;
mod audio;
mod constants;
mod hands;
mod hitboxes;
mod players;
mod timers;

use crate::core::GameConfig;
use crate::physics::collision;
use crate::types::maze::Maze;
use crate::types::world::WorldId;
use crate::weapons::combat::CombatEntity;

use macroquad::prelude::*;

use super::PlayingState;

// player_anims_in_use is gone, it picked which shared clip clocks to tick each frame
// now every body ticks its own clock (PlayerAnim), so nothing has to pick

impl PlayingState {
    #[inline(always)]
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

        let delta: f32 = raw_delta.min(0.05);
        self.just_jumped = false;
        self.just_landed = false;

        if self.dev_mode.ruler_set_start_requested {
            self.dev_mode.ruler_start = Some(self.position);
            self.dev_mode.ruler_set_start_requested = false;
        }
        if self.dev_mode.ruler_set_end_requested {
            self.dev_mode.ruler_end = Some(self.position);
            self.dev_mode.ruler_set_end_requested = false;
        }
        // skin test from the dev menu: wear the next loaded character, every animation follows
        if self.dev_mode.next_character_requested {
            self.dev_mode.next_character_requested = false;
            self.next_character();
        }
        if self.dev_mode.default_character_requested {
            self.dev_mode.default_character_requested = false;
            self.set_character(Self::DEFAULT_CHARACTER); // straight back to the default soldier
        }
        if self.dev_mode.is_unlocked() {
            self.dev_mode.character_label = self.character_label(); // kept fresh so "still loading" counts down
        }

        // update kill feed timers and drop expired entries in a single pass
        self.kill_feed.retain_mut(|entry| {
            entry.timer -= delta;
            entry.timer > 0.0
        });

        // tick kill confirmation timer
        if self.kill_confirm_timer > 0.0 {
            self.kill_confirm_timer = (self.kill_confirm_timer - delta).max(0.0);
        }

        // tick death animation timers on other players; remove when expired
        let mut need_rebuild = false;
        self.other_players.retain_mut(|p| {
            if let Some(ref mut t) = p.death_timer {
                *t -= delta;
                if *t <= 0.0 {
                    need_rebuild = true;
                    return false; // remove
                }
            }
            true
        });
        if need_rebuild {
            self.rebuild_player_index();
        }


        if self.is_dead {
            self.simulate_dead(delta);
            return;
        }

        self.weapons.update(delta);

        // Auto-switch to another weapon when current is empty (all modes except Gun Game)
        if self.match_mode != Some(crate::net::protocols::MODE_GUN_GAME) {
            self.weapons.auto_switch_on_empty();
        }

        self.update_hands_animations(delta);

        self.target_dummy.update(delta);

        self.update_remote_players(delta, config);
        self.tick_timers(delta, maze);

        self.update_stamina(config, delta);
        if self.movement_state.prone_transition_timer > 0.0 {
            self.movement_state.prone_transition_timer =
                (self.movement_state.prone_transition_timer - delta).max(0.0);
        }
        if self.movement_state.slide_cooldown_timer > 0.0 {
            self.movement_state.slide_cooldown_timer =
                (self.movement_state.slide_cooldown_timer - delta).max(0.0);
        }
        if self.movement_state.jump_cooldown_timer > 0.0 {
            self.movement_state.jump_cooldown_timer =
                (self.movement_state.jump_cooldown_timer - delta).max(0.0);
        }
        self.update_crouch_height(config, delta);
        let was_grounded = self.movement_state.is_grounded;
        // Only allow jumping when not frozen after teleport
        if self.post_teleport_freeze_timer <= 0.0 {
            self.apply_jump();
        } else {
            // clear any pending jump while frozen
            self.jump_requested = false;
            self.just_jumped = false;
        }
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
                    self.movement_state.is_grounded,
                );
            }
            WorldId::Maze => {
                if self.position.y <= 0.0 {
                    self.position.y = 0.0;
                    self.movement_state.is_grounded = true;
                    self.movement_state.velocity.y = 0.0;
                }
                let prev_position = self.position;

                // while in the short post-teleport freeze, prevent movement on the client
                if self.post_teleport_freeze_timer <= 0.0 {
                    self.apply_movement(delta);
                } else {
                    // keep player stationary horizontally while still allowing gravity to apply
                    self.movement_state.velocity.x = 0.0;
                    self.movement_state.velocity.z = 0.0;
                }

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
        // mirrors are solid in every world
        collision::resolve_mirrors(
            &mut self.position,
            &mut self.movement_state.velocity,
            self.movement_state.height,
            &self.mirrors,
        );

        if !was_grounded && self.movement_state.is_grounded {
            self.just_landed = true;
            // short pause before the next jump, holding space jumps again as soon as it runs out
            self.movement_state.jump_cooldown_timer = self.movement_config.jump_land_cooldown;
        }
        self.prev_was_reloading = self.weapons.active().is_reloading;

        self.update_local_footsteps(delta, config);

        // your one third person body follows you, after movement so the mirror isnt a frame behind
        self.update_self_avatar(delta);
    }

    /// The dead player's frame: respawn and death-camera timers, plus the
    /// other players and bots kept moving while spectating.
    fn simulate_dead(&mut self, delta: f32) {
        if self.death_timer > 0.0 {
            self.death_timer -= delta;
        } else if self.current_world == WorldId::Maze
            && self.match_mode != Some(crate::net::protocols::MODE_SEARCH_AND_DESTROY)
        {
            // Auto-respawn once the death timer expires (not in S&D — round start handles it)
            self.pending_respawn = true;
        }

        // Tick death camera phase
        use super::state::DeathCameraPhase;
        match self.death_camera_phase {
            DeathCameraPhase::ZoomOut { timer, corpse_pos, corpse_yaw } => {
                let new_timer = timer + delta;
                if new_timer >= 0.5 {
                    self.death_camera_phase = DeathCameraPhase::Watching {
                        timer: 0.0,
                        corpse_pos,
                        corpse_yaw,
                    };
                } else {
                    self.death_camera_phase = DeathCameraPhase::ZoomOut {
                        timer: new_timer,
                        corpse_pos,
                        corpse_yaw,
                    };
                }
            }
            DeathCameraPhase::Watching { timer, corpse_pos, corpse_yaw } => {
                let new_timer = timer + delta;
                if new_timer >= 2.0 {
                    self.death_camera_phase = DeathCameraPhase::Done;
                } else {
                    self.death_camera_phase = DeathCameraPhase::Watching {
                        timer: new_timer,
                        corpse_pos,
                        corpse_yaw,
                    };
                }
            }
            _ => {}
        }
        // Keep other players' positions smoothed and their animations running
        // while we're spectating — skipping this made everyone look frozen.
        for p in &mut self.other_players {
            p.update(delta);
        }
        for bot in &mut self.practice_bots {
            bot.simulate_yaw(delta, Some(self.position));
            bot.update(delta);
        }
        // same as the normal frame, keep every body's own clock going while we spectate
        let animated_models = &self.animated_models;
        for p in &mut self.other_players {
            let pos = p.render_position();
            let input = AnimInput { skin: p.skin_type, pos, pitch: p.render_pitch, aiming: p.is_ads, crouching: p.is_crouching, fired: std::mem::take(&mut p.fired) };
            p.anim.tick(&input, delta, animated_models);
        }
        for bot in &mut self.practice_bots {
            let pos = bot.player.render_position();
            let b = &bot.player;
            let input = AnimInput { skin: b.skin_type, pos, pitch: b.render_pitch, aiming: b.is_ads, crouching: b.is_crouching, fired: b.fired };
            bot.player.anim.tick(&input, delta, animated_models);
        }
        // your body becomes the corpse here and plays its death from the start
        self.update_self_avatar(delta);
    }
}
