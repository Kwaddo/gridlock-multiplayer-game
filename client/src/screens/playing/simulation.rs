use crate::core::GameConfig;
use crate::physics::collision;
use crate::types::maze::Maze;
use crate::types::world::WorldId;
use crate::weapons::combat::CombatEntity;

use macroquad::prelude::*;

use super::PlayingState;

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
            bot.update(delta);
            bot.simulate_yaw(delta, local_pos);
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
}
