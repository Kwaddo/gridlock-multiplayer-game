use crate::core::GameConfig;
use crate::screens::input;
use crate::types::world::WorldId;

use macroquad::prelude::*;

use super::PlayingState;

impl PlayingState {
    /// Mouse look, movement keys (sprint/slide/crouch/prone/jump) and
    /// weapon switching for a live player.
    pub(super) fn process_look_and_movement(&mut self, config: &GameConfig, weapon_input_locked: bool) {
        let weapon = self.weapons.active();
        let stats = weapon.stats();
        let ads_mult = stats.ads_sensitivity.unwrap_or(1.0);
        let base_fov_rad = config.raw_fov().to_radians();
        let ads_fov_rad = stats.ads_zoom_fov.to_radians();
        let fov_scale = (ads_fov_rad / 2.0).tan() / (base_fov_rad / 2.0).tan();
        let ads_factor = 1.0 + (fov_scale * ads_mult - 1.0) * weapon.ads_progress;
        let sens = config.raw_sensitivity() * ads_factor;

        input::process_mouse(
            &mut self.last_mouse_pos,
            &mut self.yaw,
            &mut self.pitch,
            sens,
        );

        self.move_dir = input::process_movement_keys(self.yaw, config);
        // prone: no a/d, only crawl forward or back (there are no sideways prone clips anymore)
        // w+a still crawls forward at full speed instead of drifting, before this you could strafe while lying down
        if self.movement_state.is_prone {
            let forward = vec3(self.yaw.cos(), 0.0, self.yaw.sin());
            let along = self.move_dir.dot(forward);
            self.move_dir = if along.abs() > 1e-4 { forward * along.signum() } else { Vec3::ZERO };
        }
        if self.dev_mode.ruler_set_start_requested {
            self.dev_mode.ruler_start = Some(self.position);
            self.dev_mode.ruler_set_start_requested = false;
        }
        if self.dev_mode.ruler_set_end_requested {
            self.dev_mode.ruler_end = Some(self.position);
            self.dev_mode.ruler_set_end_requested = false;
        }
        self.movement_state.is_sprinting =
            input::process_sprint(config, &self.movement_state, self.move_dir);
        self.movement_state.is_slow_walking = input::process_slow_walk(
            config,
            self.movement_state.is_slow_walking,
            self.movement_state.is_sprinting,
            self.movement_state.is_prone,
            self.movement_state.is_sliding,
        );
        // slow walk: forward only, s a and d do nothing (ahmed's rule), like prone but no backing up either
        if self.movement_state.is_slow_walking {
            let forward = vec3(self.yaw.cos(), 0.0, self.yaw.sin());
            self.move_dir = if self.move_dir.dot(forward) > 1e-4 { forward } else { Vec3::ZERO };
        }
        let crouch_just_pressed = config.bind_crouch.is_pressed();
        if crouch_just_pressed
            && self.movement_state.is_sprinting
            && !self.movement_state.is_sliding
            && self.movement_state.slide_cooldown_timer <= 0.0
            && self.movement_state.stamina >= self.movement_config.slide_stamina_cost
            && self.movement_state.is_grounded
            && !self.movement_state.is_prone
        {
            self.movement_state.is_sliding = true;
            self.movement_state.slide_timer = self.movement_config.slide_duration;
            self.movement_state.stamina -= self.movement_config.slide_stamina_cost;
            self.movement_state.is_sprinting = false;
            self.movement_state.is_crouching = true;
            self.movement_state.is_slow_walking = false;
            self.movement_state.slide_direction = if self.move_dir.length_squared() > 0.0 {
                self.move_dir.normalize()
            } else {
                vec3(self.yaw.cos(), 0.0, self.yaw.sin())
            };
            self.movement_state.slide_speed_current = self.movement_config.slide_initial_speed;
        }

        if self.movement_state.is_sliding {
            self.movement_state.is_sprinting = false;
            self.movement_state.is_slow_walking = false;
            self.movement_state.is_crouching = true;
        } else {
            self.movement_state.is_crouching =
                input::process_crouch(config, self.movement_state.is_crouching);
        }
        self.jump_requested =
            input::process_jump(config, self.movement_state.is_grounded, &mut self.weapons)
                && self.movement_state.jump_cooldown_timer <= 0.0; // just landed, wait a moment

        if self.movement_state.is_sliding
            && (self.jump_requested || self.movement_state.slide_timer <= 0.0)
        {
            self.movement_state.is_sliding = false;
            self.movement_state.slide_timer = 0.0;
            self.movement_state.slide_cooldown_timer = self.movement_config.slide_cooldown;
            self.movement_state.is_crouching = true;
            self.movement_state.slide_speed_current = 0.0;
        }

        if !self.movement_state.is_sliding {
            let wants_prone = input::process_prone(config, self.movement_state.is_prone);
            if wants_prone != self.movement_state.is_prone {
                self.movement_state.is_prone = wants_prone;
                self.movement_state.prone_transition_timer =
                    self.movement_config.prone_transition_time;
                if wants_prone {
                    self.movement_state.is_sprinting = false;
                    self.movement_state.is_crouching = false;
                    self.movement_state.is_slow_walking = false;
                }
            }
        }

        // cancel prone on jump
        if self.jump_requested && self.movement_state.is_prone {
            self.movement_state.is_prone = false;
            self.movement_state.prone_transition_timer = self.movement_config.prone_transition_time;
            self.jump_requested = false;
        }

        if self.movement_state.is_prone {
            self.movement_state.is_sprinting = false;
            self.movement_state.is_crouching = false;
            self.movement_state.is_slow_walking = false;
            self.movement_state.is_sliding = false;
        }
        // If we are in the Maze and the post-teleport freeze is active, block movement input
        if self.post_teleport_freeze_timer > 0.0 && self.current_world == WorldId::Maze {
            self.move_dir = Vec3::ZERO;
            self.movement_state.is_sprinting = false;
            self.movement_state.is_slow_walking = false;
            self.movement_state.is_crouching = false;
            self.movement_state.is_sliding = false;
            self.jump_requested = false;
        }
        if weapon_input_locked {
            self.movement_state.is_sprinting = false;
        }
        self.weapons.active_mut().ads_time_multiplier = if self.movement_state.is_sliding {
            self.movement_config.slide_ads_time_mult
        } else {
            1.0
        };
        let prev_switch_cooldown = self.weapons.switch_cooldown;
        if !weapon_input_locked {
            input::process_weapon_input(&mut self.weapons, config);
        }
        if self.movement_state.is_stamina_depleted
            && prev_switch_cooldown <= 0.0
            && self.weapons.switch_cooldown > 0.0
            && self.movement_config.stamina_depleted_speed_mult > 0.0
        {
            // Depleted stamina penalty: slower weapon switching.
            self.weapons.switch_cooldown /= self.movement_config.stamina_depleted_speed_mult;
        }
    }
}
