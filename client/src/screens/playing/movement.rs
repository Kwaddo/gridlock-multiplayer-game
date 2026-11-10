use crate::core::GameConfig;
use macroquad::prelude::*;

use super::PlayingState;

impl PlayingState {
    // pub(super) = visible to playing.rs (the parent module) but not the rest of the codebase
    pub(super) fn update_stamina(&mut self, config: &GameConfig, delta: f32) {
        // if stamina runs out while holding sprinting, no stamina will be regained until player releases sprint key, so you have to tap it again to start regaining stamina.
        //this prevents the player from just holding sprint and waiting for stamina to regen, which would be a bit too easy to abuse.
        let recover_rate = if
            self.movement_state.stamina <= self.movement_config.stamina_depleted_threshold
        {
            self.movement_config.stamina_depleted_recover
        } else {
            self.movement_config.stamina_recover
        };

        if self.movement_state.is_sprinting {
            self.movement_state.stamina -= self.movement_config.stamina_drain * delta;
        } else {
            if !config.bind_sprint.is_down() {
                self.movement_state.stamina = (
                    self.movement_state.stamina +
                    recover_rate * delta
                ).min(100.0);
            }
        }
        // stamina rework
        self.movement_state.stamina = self.movement_state.stamina.max(0.0);

        self.movement_state.is_stamina_depleted =
            self.movement_state.stamina <= self.movement_config.stamina_depleted_threshold;
    }

    pub(super) fn update_crouch_height(&mut self, config: &GameConfig, delta: f32) {
        let target = if self.movement_state.is_sliding {
            self.movement_config.slide_height
        } else if self.movement_state.is_prone {
            self.movement_config.prone_height
        } else if self.movement_state.is_crouching {
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
            self.just_jumped = true;
        }
    }

    pub(super) fn apply_gravity(&mut self, delta: f32) {
        self.movement_state.velocity.y -= self.movement_config.gravity * delta;
        self.position.y += self.movement_state.velocity.y * delta;
    }

    pub(super) fn apply_movement(&mut self, delta: f32) {
        if self.movement_state.is_sliding {
            self.movement_state.slide_speed_current = (self.movement_state.slide_speed_current
                - self.movement_config.slide_friction * delta)
                .max(0.0);

            let slide_vel =
                self.movement_state.slide_direction * self.movement_state.slide_speed_current;
            let saved_vy = self.movement_state.velocity.y;
            self.movement_state.velocity = slide_vel;
            self.movement_state.velocity.y = saved_vy;

            self.position.x += self.movement_state.velocity.x * delta;
            self.position.z += self.movement_state.velocity.z * delta;
            self.movement_state.slide_timer -= delta;
            return;
        }

        let speed = if self.movement_state.is_prone {
            self.movement_config.prone_speed
        } else if self.movement_state.is_sprinting {
            self.movement_config.sprint_speed
        } else if self.movement_state.is_crouching {
            self.movement_config.crouch_speed
        } else if self.movement_state.is_slow_walking {
            self.movement_config.slow_walk_speed
        } else {
            self.movement_config.walk_speed
        };

        let speed = if self.movement_state.is_stamina_depleted {
            speed * self.movement_config.stamina_depleted_speed_mult
        } else {
            speed
        };

        let speed = if self.weapons.knife_active {
            speed * crate::weapons::stats::melee::KNIFE_SPEED_BONUS
        } else {
            speed
        };

        let speed = if self.knife_miss_slow_timer > 0.0 {
            speed * crate::weapons::stats::melee::KNIFE_MISS_MOVE_SLOW
        } else {
            speed
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
        let moving = self.move_dir.length_squared() > 0.01;
        if self.movement_state.is_sprinting && self.movement_state.is_grounded {
            self.bob_timer += delta * 12.0;
        } else if self.movement_state.is_slow_walking && self.movement_state.is_grounded && moving {
            self.bob_timer += delta * 4.0; // slow careful sway while stalking, before this slow walk had no motion
        } else {
            self.bob_timer = 0.0;
        }
        // stealth focus, eases in while slow walking and back out
        let target = if self.movement_state.is_slow_walking { 1.0 } else { 0.0 };
        let step = delta / self.movement_config.slow_walk_zoom_time.max(0.01);
        self.slow_walk_zoom += (target - self.slow_walk_zoom).clamp(-step, step);
    }
}
