use macroquad::prelude::*;
use crate::core::GameConfig;

use super::PlayingState;

impl PlayingState {
    // pub(super) = visible to playing.rs (the parent module) but not the rest of the codebase
    pub(super) fn update_stamina(&mut self, config: &GameConfig, delta: f32) {
        // if stamina runs out while holding sprinting, no stamina will be regained until player releases sprint key, so you have to tap it again to start regaining stamina.
        //this prevents the player from just holding sprint and waiting for stamina to regen, which would be a bit too easy to abuse.

        if self.movement_state.is_sprinting {
            self.movement_state.stamina -= self.movement_config.stamina_drain * delta;
        } else {
            if !is_key_down(config.key_sprint) {
                self.movement_state.stamina = (
                    self.movement_state.stamina +
                    self.movement_config.stamina_recover * delta
                ).min(100.0);
            }
        }
    }

    pub(super) fn update_crouch_height(&mut self, config: &GameConfig, delta: f32) {
        let target = if self.movement_state.is_crouching {
            config.crouch_height
        } else {
            config.standing_height
        };
        self.movement_state.height += (target - self.movement_state.height) * 10.0 * delta;
    }

    pub(super) fn apply_jump(&mut self) {
        if self.jump_requested {
            self.movement_state.velocity.y = self.movement_config.jump_strength;
            self.movement_state.is_grounded = false;
            self.jump_requested = false;
        }
    }

    pub(super) fn apply_gravity(&mut self, delta: f32) {
        self.movement_state.velocity.y -= self.movement_config.gravity * delta;
        self.position.y += self.movement_state.velocity.y * delta;
    }

    pub(super) fn apply_movement(&mut self, delta: f32) {
        let speed = if self.movement_state.is_sprinting {
            self.movement_config.sprint_speed
        } else if self.movement_state.is_crouching {
            self.movement_config.crouch_speed
        } else {
            self.movement_config.walk_speed
        };

        let target_velocity = self.move_dir * speed;
        let rate = if !self.movement_state.is_grounded {
            2.0 // Low air control
        } else if self.move_dir != Vec3::ZERO {
            self.movement_config.acceleration
        } else {
            self.movement_config.friction
        };

        let saved_vy = self.movement_state.velocity.y;
        self.movement_state.velocity = self.movement_state.velocity.lerp(
            target_velocity,
            rate * delta
        );
        self.movement_state.velocity.y = saved_vy;

        self.position.x += self.movement_state.velocity.x * delta;
        self.position.z += self.movement_state.velocity.z * delta;
    }

    pub(super) fn update_head_bob(&mut self, delta: f32) {
        if self.movement_state.is_sprinting && self.movement_state.is_grounded {
            self.bob_timer += delta * 12.0;
        } else {
            self.bob_timer = 0.0;
        }
    }
}
