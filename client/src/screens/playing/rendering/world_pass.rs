use macroquad::prelude::*;

use crate::types::maze::Maze;
use crate::types::world::WorldId;
use crate::world::{ draw_maze, lobby::draw_lobby };

use super::super::PlayingState;

impl PlayingState {
    /// everything that exists in the 3D world drawn by the main camera AND by every mirror camera. does not include mirror bodies, HUD, viewmodel, nametags.
    /// 'eye' = where the camera drawing this sits (tracers face it).
    pub(crate) fn draw_world(&self, maze: Option<&Maze>, eye: Vec3) {
        //l 1. the map
        match self.current_world {
            WorldId::Lobby => {
                draw_lobby(self.assets.get("floor_texture"));
            }
            WorldId::Maze => {
                if let Some(m) = maze {
                    draw_maze(
                        Some(m),
                        self.position,
                        crate::core::VIEW_DISTANCE,
                        self.dev_mode.show_hitboxes,
                        &self.destroyed_lights,
                    );
                }
            }
        }

        // 2. S&D markers
        // S&D: render bomb site marker and dropped/planted bomb in 3D
        if self.match_mode == Some(crate::net::protocols::MODE_SEARCH_AND_DESTROY)
            && self.current_world == WorldId::Maze
            && self.snd_round > 0
        {
            self.draw_snd_world_markers();
        }

        // 3. things in the world
        if self.current_world == WorldId::Lobby {
            self.draw_target_dummy();
        }

        // cardboard boxes, only if the list is for this world (an old one can show up mid-teleport)
        if self.cardboard.world == self.current_world {
            for b in &self.cardboard.boxes {
                let (at, yaw, roll, floor) = self.box_draw(b);
                // a camera this close is inside our own box
                if self.cardboard.in_box == Some(b.id) && (eye - at).length() < 0.9 {
                    continue;
                }
                let model = self.cardboard.look.model_key().and_then(|k| self.models.get(k)).map(|m| m.as_slice());
                crate::world::render::cardboard_box::draw_cardboard_box(at, yaw, roll, model);
                crate::world::render::cardboard_box::draw_box_shadow(floor, yaw, at.y - floor.y);
            }
        }

        self.draw_box_bursts();

        self.draw_other_players();
        self.draw_death_corpse();
        self.draw_player_hitboxes();
        self.draw_tracers(eye);
        self.draw_world_muzzle_flashes();
        self.draw_bullet_holes();
    }
}
