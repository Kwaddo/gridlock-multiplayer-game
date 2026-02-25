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
        config: &mut GameConfig,
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
                        &self.destroyed_lights,
                    );
                }
            }
        }

        // in-world teleporter removed

        if self.current_world == WorldId::Lobby {
            self.draw_target_dummy();
        }

        self.draw_other_players();
        self.draw_player_hitboxes();
        self.draw_tracers(camera.position);
        self.draw_world_muzzle_flashes();
        self.draw_bullet_holes();
        if self.dev_mode.ruler_active {
            if let (Some(mut start), Some(mut end)) = (self.dev_mode.ruler_start, self.dev_mode.ruler_end) {
                start.y = 0.02;
                end.y = 0.02;
                draw_line_3d(start, end, GREEN);

                let delta = end - start;
                let dist = delta.length();
                if dist > 0.001 {
                    let dir = delta / dist;
                    let tick_count = dist.floor() as i32;
                    for i in 0..=tick_count {
                        let p = start + dir * (i as f32);
                        let color = if i % 5 == 0 { YELLOW } else { ORANGE };
                        draw_line_3d(p, p + vec3(0.0, 0.20, 0.0), color);
                    }
                }
            }
        }

        set_default_camera();

        if self.current_world == WorldId::Lobby {
            self.draw_target_dummy_hud(&camera);
        }

        if self.dev_mode.ruler_active {
            if let (Some(start), Some(end)) = (self.dev_mode.ruler_start, self.dev_mode.ruler_end) {
                let dist = (end - start).length();
                let txt = format!("Ruler Distance: {:.2}", dist);
                draw_text_ex(
                    &txt,
                    16.0,
                    24.0,
                    crate::ui::fonts::text_params(18, Color::from_rgba(160, 240, 160, 255)),
                );
            }
        }

        self.draw_player_overhead(&camera, maze);
        self.draw_lobby_console_prompt();
        self.draw_shooting_range_prompt();

        // Lobby console (Start Game) overlay
        self.draw_lobby_console(config);

        // Don't draw viewmodel while dead or when lobby console is open
        if !self.is_dead && !self.lobby_console_open {
            self.draw_viewmodel();
        }

        // When lobby console is open, suppress regular HUD (crosshair, minimap, status panels, etc.)
        if !self.lobby_console_open && !self.is_dead {
            // compute local team from connected_players (default spectator 0)
            let local_team = self
                .connected_players
                .iter()
                .find(|(id, ..)| *id == player_id)
                .map(|(_, _name, _kills, _deaths, _gl, team, _wep, _is_bot)| *team)
                .unwrap_or(0u8);

            draw_hud(
                username,
                player_id,
                self.position,
                self.yaw,
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
                self.match_time_left,
                self.damage_flash_timer,
                self.last_damage_attacker.as_deref(),
                maze,
                self.current_world,
                local_team,
                config,
            );

            let dt = get_frame_time();
            if self.movement_state.is_stamina_depleted {
                self.stamina_depleted_vignette_alpha =
                    (self.stamina_depleted_vignette_alpha + dt * 2.0).min(1.0);
            } else {
                self.stamina_depleted_vignette_alpha =
                    (self.stamina_depleted_vignette_alpha - dt * 2.0).max(0.0);
            }

            if self.stamina_depleted_vignette_alpha > 0.0 {
                let pulse = ((get_time() as f32 * 4.0).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                let alpha =
                    (self.stamina_depleted_vignette_alpha * (0.10 + 0.10 * pulse) * 255.0) as u8;
                let w = screen_width();
                let h = screen_height();
                let edge = (w.min(h) * 0.08).max(28.0);
                let col = Color::from_rgba(120, 20, 20, alpha);

                draw_rectangle(0.0, 0.0, w, edge, col);
                draw_rectangle(0.0, h - edge, w, edge, col);
                draw_rectangle(0.0, 0.0, edge, h, col);
                draw_rectangle(w - edge, 0.0, edge, h, col);
            }
        }

        // Show player list / scoreboard while holding Tab
        if is_key_down(KeyCode::Tab) {
            crate::ui::hud::draw_player_list(
                &self.connected_players,
                player_id,
                self.local_kills,
                self.local_deaths,
                (self.weapons.active_slot + 1) as u8,
                self.current_world == WorldId::Maze,
                self.match_mode,
            );
        }

        // Spawn feedback toast (e.g. teleporter fallback)
        if let Some(msg) = &self.spawn_feedback_message {
            let alpha = (self.spawn_feedback_timer / 3.0).clamp(0.0, 1.0);
            let color = Color::from_rgba(255, 200, 100, (200.0 * alpha) as u8);
            let w = crate::ui::fonts::measure_game_text(msg, 18);
            draw_text_ex(
                msg,
                (screen_width() - w) / 2.0,
                80.0,
                crate::ui::fonts::text_params(18, color),
            );
        }

        // Match-start countdown (large center number)
        if self.match_start_countdown_timer > 0.0 {
            let secs = self.match_start_countdown_timer.ceil() as i32;
            if secs != self.match_start_cached_secs {
                self.match_start_cached_secs = secs;
                self.match_start_cached_text = secs.to_string();
            }
            let txt = &self.match_start_cached_text;
            let w = crate::ui::fonts::measure_game_text(txt, 96);
            // when shown in the Lobby (before teleport), nudge the countdown slightly upward
            let y_center = if self.current_world == WorldId::Lobby {
                screen_height() / 2.0 - 40.0
            } else {
                screen_height() / 2.0
            };
            draw_text_ex(
                txt,
                (screen_width() - w) / 2.0,
                y_center,
                crate::ui::fonts::text_params(96, Color::from_rgba(255, 220, 120, 255)),
            );
            let sub = "Match starts in";
            let sw = crate::ui::fonts::measure_game_text(sub, 20);
            draw_text_ex(
                sub,
                (screen_width() - sw) / 2.0,
                y_center - 80.0,
                crate::ui::fonts::text_params(20, Color::from_rgba(200, 200, 200, 200)),
            );
        }

        // Post-teleport freeze indicator (players cannot move for a short time after being moved into the Maze)
        if self.post_teleport_freeze_timer > 0.0 && self.current_world == WorldId::Maze {
            let secs = self.post_teleport_freeze_timer.ceil() as i32;
            if secs != self.post_freeze_cached_secs {
                self.post_freeze_cached_secs = secs;
                self.post_freeze_cached_text = format!("Movement & firing locked for {}s", secs);
            }
            let txt = &self.post_freeze_cached_text;
            let sw = crate::ui::fonts::measure_game_text(txt, 20);
            draw_text_ex(
                txt,
                (screen_width() - sw) / 2.0,
                screen_height() / 2.0 + 60.0,
                crate::ui::fonts::text_params(20, Color::from_rgba(200, 200, 200, 200)),
            );
        }

        // Victory screen overlay (match end)
        if self.match_end_countdown_timer > 0.0 {
            if let Some(winner) = &self.match_end_winner {
                self.draw_victory_screen(winner, player_id);
            }
        }

        // Bottom-left announcements (recent events like "PlayerX joined the match")
        if self.match_end_countdown_timer <= 0.0 {
            let mut y = screen_height() - 24.0;
            for (txt, timer) in &self.announcements {
                let alpha: f32 = (timer / 4.0).clamp(0.0, 1.0);
                let col = Color::from_rgba(200, 200, 200, (220.0 * alpha) as u8);
                draw_text_ex(txt, 12.0, y, crate::ui::fonts::text_params(14, col));
                y -= 18.0;
            }
        }

        if self.is_dead && self.match_end_countdown_timer <= 0.0 {
            self.draw_death_screen();
        }
    }
}
