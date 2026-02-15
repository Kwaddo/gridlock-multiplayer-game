use crate::core::GameConfig;
use crate::physics::collision;
use crate::weapons::combat::CombatEntity;
use crate::types::maze::Maze;
use crate::types::world::WorldId;
use macroquad::prelude::*;

use super::PlayingState;

impl PlayingState {
    pub fn simulate(&mut self, config: &GameConfig, maze: Option<&Maze>) {
        let delta = get_frame_time();

        if self.is_dead {
            if self.death_timer > 0.0 {
                self.death_timer -= delta;
            } else if self.current_world == WorldId::Lobby && is_key_pressed(KeyCode::Space) {
                self.pending_respawn = true;
            }
            return;
        }

        for entry in &mut self.kill_feed {
            entry.timer -= delta;
        }
        self.kill_feed.retain(|entry| entry.timer > 0.0);

        self.weapons.update(delta);
        self.target_dummy.update(delta);
        for p in &mut self.other_players {
            p.update(delta);
        }
        for bot in &mut self.practice_bots {
            bot.update(delta);
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

        self.update_other_players_smoothing(delta);

        self.update_stamina(config, delta);
        self.update_crouch_height(config, delta);
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
                self.apply_movement(delta);
                if let Some(m) = maze {
                    collision::resolve_maze(
                        &mut self.position,
                        &mut self.movement_state.velocity,
                        m,
                    );
                }
            }
        }
    }
}
