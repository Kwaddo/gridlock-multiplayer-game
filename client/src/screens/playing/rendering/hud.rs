use macroquad::prelude::*;

use crate::types::maze::Maze;

use super::super::PlayingState;

impl PlayingState {
    pub(crate) fn draw_target_dummy_hud(&self, camera: &Camera3D) {
        let d = &self.target_dummy;
        let world_pos = d.position + vec3(0.0, 2.0, 0.0);
        let rel = world_pos - camera.position;
        let forward = (camera.target - camera.position).normalize();
        let right = forward.cross(camera.up).normalize();
        let up = right.cross(forward).normalize();

        let dot = rel.dot(forward);
        if dot < 0.1 {
            return;
        }

        let fov_rad = camera.fovy;
        let fov_scale = screen_height() / (2.0 * (fov_rad / 2.0).tan());

        let sx = screen_width() / 2.0 + (rel.dot(right) / dot) * fov_scale;
        let sy = screen_height() / 2.0 - (rel.dot(up) / dot) * fov_scale;

        let bar_w = 60.0;
        let bar_h = 6.0;
        let hp_frac = d.hp / d.max_hp;

        draw_rectangle(sx - bar_w / 2.0, sy, bar_w, bar_h, Color::from_rgba(40, 40, 40, 200));
        let hp_color = if hp_frac > 0.5 {
            Color::from_rgba(80, 200, 80, 255)
        } else if hp_frac > 0.25 {
            Color::from_rgba(220, 180, 40, 255)
        } else {
            RED
        };
        draw_rectangle(sx - bar_w / 2.0, sy, bar_w * hp_frac, bar_h, hp_color);
        draw_rectangle_lines(sx - bar_w / 2.0, sy, bar_w, bar_h, 1.0, WHITE);

        let label = "TARGET DUMMY";
        let measure_w = self.target_dummy_label_dims.0;
        draw_text(label, sx - measure_w / 2.0, sy - 4.0, 14.0, WHITE);
    }

    /// Draws nametags (always) and health bars (dev mode) above other players.
    /// Uses proper camera-based 3D-to-screen projection. Hides tags when a
    /// maze wall is between the camera and the player.
    pub(crate) fn draw_player_overhead(&self, camera: &Camera3D, maze: Option<&Maze>, player_id: u32) {
        let forward = (camera.target - camera.position).normalize();
        let right = forward.cross(camera.up).normalize();
        let up = right.cross(forward).normalize();
        let fov_rad = camera.fovy;
        let fov_scale = screen_height() / (2.0 * (fov_rad / 2.0).tan());

        let show_hp = self.dev_mode.show_health_bars;

        // Resolve local team for teammate/enemy distinction in team modes.
        let is_team_mode = matches!(
            self.match_mode,
            Some(m) if m == crate::net::protocols::MODE_TEAM_DEATHMATCH
                    || m == crate::net::protocols::MODE_SEARCH_AND_DESTROY
        );
        let local_team: u8 = self
            .connected_players
            .iter()
            .find(|(id, ..)| *id == player_id)
            .map(|(_, _, _, _, _, team, _, _)| *team)
            .unwrap_or(0);

        // helper: project world-space point to screen coordinates
        let project = |world_pos: Vec3| -> Option<(f32, f32)> {
            let rel = world_pos - camera.position;
            let dot = rel.dot(forward);
            if dot < 0.1 {
                return None;
            }
            let sx = screen_width() / 2.0 + (rel.dot(right) / dot) * fov_scale;
            let sy = screen_height() / 2.0 - (rel.dot(up) / dot) * fov_scale;
            Some((sx, sy))
        };

        // helper: check if a wall blocks line-of-sight to a target position
        let is_occluded = |target_pos: Vec3| -> bool {
            if let Some(m) = maze {
                let dir = target_pos - camera.position;
                let dist = dir.length();
                if dist < 0.01 {
                    return false;
                }
                let dir_norm = dir / dist;
                // check if any wall is between camera and target
                if let Some((_, wall_dist)) = m.ray_test_walls(camera.position, dir_norm, dist) {
                    return wall_dist < dist - 0.1;
                }
            }
            false
        };

        // Target dummy (lobby only)
        if show_hp && self.current_world == crate::types::world::WorldId::Lobby {
            let world_pos = self.target_dummy.position + vec3(0.0, 1.7, 0.0);
            if let Some((sx, sy)) = project(world_pos) {
                Self::draw_health_bar(
                    sx,
                    sy,
                    self.target_dummy.hp,
                    self.target_dummy.max_hp,
                    "Target Dummy"
                );
            }
        }

        // Other players
        for p in &self.other_players {
            let display_name = if p.is_bot {
                format!("[BOT] {}", p.username)
            } else {
                p.username.clone()
            };

            let player_pos = p.render_position();
            let tag_pos = player_pos + vec3(0.0, 1.7, 0.0);

            // In team modes distinguish teammates from enemies:
            //   - teammates: always visible (no wall occlusion), shown in green
            //   - enemies:   occluded by walls, shown in orange/red
            // In non-team modes keep existing per-player color with wall occlusion.
            let is_teammate = is_team_mode && local_team != 0 && p.team == local_team;

            if !is_teammate && is_occluded(player_pos + vec3(0.0, 0.8, 0.0)) {
                continue;
            }

            if let Some((sx, sy)) = project(tag_pos) {
                if show_hp {
                    Self::draw_health_bar(sx, sy, p.hp, p.max_hp, &display_name);
                } else {
                    let tag_color = if is_team_mode {
                        if is_teammate {
                            Color::from_rgba(80, 220, 120, 255) // green for teammates
                        } else {
                            Color::from_rgba(255, 120, 60, 255) // orange-red for enemies
                        }
                    } else {
                        p.color
                    };
                    Self::draw_nametag(sx, sy, &display_name, tag_color);
                }
            }
        }

        // Practice bots
        for bot in &self.practice_bots {
            let p = &bot.player;
            let bot_pos = vec3(p.x, p.y, p.z);
            let tag_pos = bot_pos + vec3(0.0, 1.7, 0.0);

            if is_occluded(bot_pos + vec3(0.0, 0.8, 0.0)) {
                continue;
            }

            if let Some((sx, sy)) = project(tag_pos) {
                let display_name = if p.is_bot {
                    format!("[BOT] {}", p.username)
                } else {
                    p.username.clone()
                };
                if show_hp {
                    Self::draw_health_bar(sx, sy, p.hp, p.max_hp, &display_name);
                } else {
                    Self::draw_nametag(sx, sy, &display_name, p.color);
                }
            }
        }
    }

    fn draw_nametag(sx: f32, sy: f32, label: &str, color: Color) {
        use crate::ui::fonts::{ measure_game_text, text_params };
        let label_w = measure_game_text(label, 14);
        draw_text_ex(label, sx - label_w / 2.0, sy, text_params(14, color));
    }

    fn draw_health_bar(sx: f32, sy: f32, hp: f32, max_hp: f32, label: &str) {
        use crate::ui::fonts::{ measure_game_text, text_params };

        let bar_w = 60.0;
        let bar_h = 6.0;
        let hp_frac = (hp / max_hp).clamp(0.0, 1.0);

        draw_rectangle(sx - bar_w / 2.0, sy, bar_w, bar_h, Color::from_rgba(40, 40, 40, 200));

        let hp_color = if hp_frac > 0.5 {
            Color::from_rgba(80, 200, 80, 255)
        } else if hp_frac > 0.25 {
            Color::from_rgba(220, 180, 40, 255)
        } else {
            RED
        };
        draw_rectangle(sx - bar_w / 2.0, sy, bar_w * hp_frac, bar_h, hp_color);

        draw_rectangle_lines(sx - bar_w / 2.0, sy, bar_w, bar_h, 1.0, WHITE);

        let label_w = measure_game_text(label, 12);
        draw_text_ex(label, sx - label_w / 2.0, sy - 4.0, text_params(12, WHITE));
    }
}
