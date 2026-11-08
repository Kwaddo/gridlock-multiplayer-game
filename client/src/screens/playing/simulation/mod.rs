mod audio;
mod constants;
mod event_sounds;
mod hands;
mod hitboxes;
mod players;
mod timers;

use crate::core::GameConfig;
use crate::physics::collision;
use crate::types::maze::Maze;
use crate::types::player::PlayerSkin;
use crate::types::world::WorldId;
use crate::weapons::combat::CombatEntity;

use macroquad::prelude::*;

use super::PlayingState;

pub use event_sounds::EventSoundState;

thread_local! {
    /// The set `player_anims_in_use` fills, kept between frames so it is
    /// cleared and refilled instead of allocated every frame.
    static PLAYER_ANIMS_IN_USE: std::cell::RefCell<std::collections::HashSet<&'static str>> =
        std::cell::RefCell::new(std::collections::HashSet::new());
}

impl PlayingState {
    /// Player animations something is drawing now: other players, practice
    /// bots, and our own corpse during the death camera. Only these need
    /// skinning; their joints also drive those players' hitboxes.
    fn player_anims_in_use(&self) -> std::collections::HashSet<&'static str> {
        let mut keys = PLAYER_ANIMS_IN_USE.take();
        keys.clear();
        let skins = self
            .other_players
            .iter()
            .map(|p| p.skin_type)
            .chain(self.practice_bots.iter().map(|b| b.player.skin_type));
        keys.extend(
            skins
                .filter_map(|skin| skin.model_key())
                .map(|key| if self.animated_models.contains_key(key) { key } else { "anim_rifle_idle" }),
        );
        // Players drawn with a missing model fall back to the idle clip.
        keys.insert("anim_rifle_idle");
        if !matches!(self.death_camera_phase, super::state::DeathCameraPhase::None) {
            keys.extend(
                [PlayerSkin::StandDeath, PlayerSkin::CrouchDeath, PlayerSkin::ProneDeath]
                    .into_iter()
                    .filter_map(|skin| skin.model_key()),
            );
        }
        keys
    }

    /// Hand the set from `player_anims_in_use` back for the next frame.
    fn recycle_player_anims_in_use(keys: std::collections::HashSet<&'static str>) {
        PLAYER_ANIMS_IN_USE.set(keys);
    }

    #[inline(always)]
    pub fn simulate(&mut self, config: &GameConfig, maze: Option<&Maze>) {
        self.audio.set_levels(config.master_volume, config.mute_audio);
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


        self.play_event_sounds(maze);

        if self.is_dead {
            self.simulate_dead(delta);
            return;
        }

        self.weapons.update(delta);

        if self.match_mode == Some(crate::net::protocols::MODE_GUN_GAME) {
            // Gun Game never runs dry: every gun keeps a full reserve, so
            // only the magazine (and reloading it) matters.
            self.weapons.refill_reserves();
        } else {
            // Auto-switch to another weapon when current is empty
            self.weapons.auto_switch_on_empty();
        }

        self.update_hands_animations(delta);

        self.target_dummy.update(delta);

        self.update_remote_players(delta, config, maze);
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

        if !was_grounded && self.movement_state.is_grounded {
            self.just_landed = true;
        }
        self.prev_was_reloading = self.weapons.active().is_reloading;

        self.update_local_footsteps(delta, config);
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
        let in_use = self.player_anims_in_use();
        for (key, model) in self.animated_models.iter_mut() {
            if in_use.contains(key.as_str()) {
                model.update(delta);
            } else if key.starts_with("anim_") {
                model.advance(delta);
            }
        }
        Self::recycle_player_anims_in_use(in_use);
        for bot in &mut self.practice_bots {
            bot.simulate_yaw(delta, Some(self.position));
            bot.update(delta);
        }
    }
}
