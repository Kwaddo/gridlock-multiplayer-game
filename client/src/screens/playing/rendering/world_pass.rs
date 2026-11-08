use macroquad::prelude::*;

use crate::core::GameConfig;
use crate::types::maze::Maze;
use crate::types::world::WorldId;
use crate::world::{ draw_maze, lobby::draw_lobby };

use super::super::PlayingState;

impl PlayingState {
    /// everything that exists in the 3D world drawn by the main camera AND by every mirror camera. does not include mirror bodies, HUD, viewmodel, nametags.
    /// 'eye' = where the camera drawing this sits (tracers face it).
    pub(crate) fn draw_world(&self, config: &GameConfig, maze: Option<&Maze>, eye: Vec3) {
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
                        config.render_distance,
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
            let cam_pos = self.position;
            let (site_x, site_y, site_z) = self.snd_bomb_site;
            let site_dist =
                ((cam_pos.x - site_x).powi(2) + (cam_pos.z - site_z).powi(2)).sqrt();

            // Bomb site marker — always visible (fade at very close range to avoid clutter)
            {
                let alpha = if site_dist <= 30.0 {
                    1.0f32
                } else {
                    1.0 - ((site_dist - 30.0) / 10.0).clamp(0.0, 1.0)
                };
                let pulse = ((get_time() as f32 * (if self.snd_bomb_planted { 4.0 } else { 1.5 })).sin() * 0.5 + 0.5) as f32;
                let site_color = if self.snd_bomb_planted {
                    Color::from_rgba(255, 50, 50, (220.0 * alpha) as u8)
                } else {
                    Color::from_rgba(255, 200, 50, ((160.0 + 60.0 * pulse) * alpha) as u8)
                };
                // Outer ring
                let r_outer = 2.0;
                let steps = 32;
                for i in 0..steps {
                    let a1 = (i as f32 / steps as f32) * std::f32::consts::TAU;
                    let a2 = ((i + 1) as f32 / steps as f32) * std::f32::consts::TAU;
                    draw_line_3d(
                        vec3(site_x + a1.cos() * r_outer, 0.02, site_z + a1.sin() * r_outer),
                        vec3(site_x + a2.cos() * r_outer, 0.02, site_z + a2.sin() * r_outer),
                        site_color,
                    );
                }
                // Inner ring
                let r_inner = 1.0;
                for i in 0..steps {
                    let a1 = (i as f32 / steps as f32) * std::f32::consts::TAU;
                    let a2 = ((i + 1) as f32 / steps as f32) * std::f32::consts::TAU;
                    draw_line_3d(
                        vec3(site_x + a1.cos() * r_inner, 0.02, site_z + a1.sin() * r_inner),
                        vec3(site_x + a2.cos() * r_inner, 0.02, site_z + a2.sin() * r_inner),
                        site_color,
                    );
                }
                // Cross lines through center
                draw_line_3d(
                    vec3(site_x - r_outer, 0.02, site_z),
                    vec3(site_x + r_outer, 0.02, site_z),
                    site_color,
                );
                draw_line_3d(
                    vec3(site_x, 0.02, site_z - r_outer),
                    vec3(site_x, 0.02, site_z + r_outer),
                    site_color,
                );
                // Vertical pillar so the site is visible over obstacles
                let pillar_alpha = (alpha * 180.0) as u8;
                let pillar_color = if self.snd_bomb_planted {
                    Color::from_rgba(255, 50, 50, pillar_alpha)
                } else {
                    Color::from_rgba(255, 200, 50, pillar_alpha)
                };
                for seg in 0..8u32 {
                    let y0 = site_y + seg as f32 * 0.4;
                    let y1 = site_y + (seg + 1) as f32 * 0.4;
                    draw_line_3d(vec3(site_x, y0, site_z), vec3(site_x, y1, site_z), pillar_color);
                }
            }

            // Dropped bomb — always visible to dead spectators, distance-gated for live players
            if let Some((bx, _by, bz)) = self.snd_bomb_dropped {
                let bomb_dist =
                    ((cam_pos.x - bx).powi(2) + (cam_pos.z - bz).powi(2)).sqrt();
                if self.is_dead || bomb_dist <= 10.0 {
                    let bomb_alpha = if self.is_dead || bomb_dist <= 6.0 {
                        1.0
                    } else {
                        1.0 - ((bomb_dist - 6.0) / 4.0).clamp(0.0, 1.0)
                    };
                    let bob = (get_time() as f32 * 2.0).sin() * 0.1;
                    draw_cube(
                        vec3(bx, 0.3 + bob, bz),
                        vec3(0.3, 0.3, 0.3),
                        None,
                        Color::from_rgba(255, 180, 50, (255.0 * bomb_alpha) as u8),
                    );
                    draw_cube_wires(
                        vec3(bx, 0.3 + bob, bz),
                        vec3(0.35, 0.35, 0.35),
                        Color::from_rgba(255, 100, 0, (200.0 * bomb_alpha) as u8),
                    );
                }
            }

            // Planted bomb at site — always visible to dead spectators
            if self.snd_bomb_planted && (self.is_dead || site_dist <= 10.0) {
                let bomb_alpha = if self.is_dead || site_dist <= 6.0 {
                    1.0
                } else {
                    1.0 - ((site_dist - 6.0) / 4.0).clamp(0.0, 1.0)
                };
                let pulse =
                    ((get_time() as f32 * 4.0).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                let r = (200.0 + 55.0 * pulse) as u8;
                draw_cube(
                    vec3(site_x, 0.15, site_z),
                    vec3(0.35, 0.25, 0.35),
                    None,
                    Color::from_rgba(r, 50, 50, (255.0 * bomb_alpha) as u8),
                );
                draw_cube_wires(
                    vec3(site_x, 0.15, site_z),
                    vec3(0.40, 0.30, 0.40),
                    Color::from_rgba(255, 0, 0, (180.0 * bomb_alpha) as u8),
                );
            }
        }

        // 3. things in the world
        if self.current_world == WorldId::Lobby {
            self.draw_target_dummy();
        }

        self.draw_other_players();
        self.draw_death_corpse();
        self.draw_player_hitboxes();
        self.draw_tracers(eye);
        self.draw_world_muzzle_flashes();
        self.draw_bullet_holes();
    }
}
