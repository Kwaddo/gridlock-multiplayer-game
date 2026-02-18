use crate::core::GameConfig;
use crate::types::maze::Maze;
use crate::types::world::WorldId;
use crate::ui::hud::draw_hud;
use crate::world::{draw_maze, lobby::draw_lobby};

use macroquad::prelude::*;

use super::PlayingState;

impl PlayingState {
    pub fn render(
        &mut self,
        config: &GameConfig,
        username: &str,
        player_id: u32,
        maze: Option<&Maze>,
    ) {
        self.update_head_bob(get_frame_time());
        let camera = self.build_camera(config);

        clear_background(Color::from_rgba(16, 20, 28, 255));
        set_camera(&camera);

        match self.current_world {
            WorldId::Lobby => {
                draw_lobby(self.assets.get("floor_texture"));
            }
            WorldId::Maze => {
                if let Some(m) = maze {
                    draw_maze(
                        Some(m),
                        self.position,
                        config.render_distance,
                        self.dev_mode.show_hitboxes,
                    );
                }
            }
        }

        self.draw_teleporter(maze);

        if self.current_world == WorldId::Lobby {
            self.draw_target_dummy();
        }

        self.draw_other_players();
        self.draw_tracers(camera.position);

        set_default_camera();

        if self.current_world == WorldId::Lobby {
            self.draw_target_dummy_hud(&camera);
        }

        self.draw_entity_health_bars(&camera);
        self.draw_teleporter_prompt(maze);
        self.draw_viewmodel();

        let forward = vec3(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        );
        let right = vec3(-self.yaw.sin(), 0.0, self.yaw.cos());

        draw_hud(
            username,
            player_id,
            self.position,
            self.yaw,
            forward,
            right,
            self.pitch,
            &self.movement_state,
            &self.other_players,
            &self.weapons,
            &self.assets,
            self.hit_marker_timer,
            self.hit_marker_headshot,
            &self.damage_numbers,
            self.hit_marker_pellet_mask,
            self.hp,
            self.max_hp,
            self.shield,
            self.max_shield,
            &self.kill_feed,
            maze,
            self.current_world,
            config,
        );

        if self.is_dead {
            self.draw_death_screen();
        }
    }
}
