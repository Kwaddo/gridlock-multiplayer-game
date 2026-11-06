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
        // Team Deathmatch in progress: the score bar takes the top of the screen
        let tdm_bar = self.team_score.filter(|_| {
            self.match_mode == Some(crate::net::protocols::MODE_TEAM_DEATHMATCH)
                && self.current_world == WorldId::Maze
        });

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

        if self.current_world == WorldId::Lobby {
            self.draw_target_dummy();
        }

        self.draw_other_players();
        self.draw_death_corpse();
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

        self.draw_player_overhead(&camera, maze, player_id);
        self.draw_lobby_console_prompt();
        self.draw_shooting_range_prompt();

        // Lobby console (Start Game) overlay
        self.draw_lobby_console(config, player_id);

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
                &self.kill_feed,
                // S&D times rounds, and the TDM score bar has the clock in it
                if self.match_mode == Some(crate::net::protocols::MODE_SEARCH_AND_DESTROY)
                    || tdm_bar.is_some()
                {
                    0.0
                } else {
                    self.match_time_left
                },
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

        if let Some(score) = tdm_bar {
            let local_team = self
                .connected_players
                .iter()
                .find(|(id, ..)| *id == player_id)
                .map(|(_, _, _, _, _, team, _, _)| *team)
                .unwrap_or(0);
            let clock = (!config.hide_match_timer && self.match_time_left > 0.0)
                .then_some(self.match_time_left);
            crate::ui::hud::draw_team_score_bar(score, local_team, clock);
        }

        // Show player list / scoreboard while holding Tab
        if is_key_down(KeyCode::Tab) && tdm_bar.is_some() {
            crate::ui::hud::draw_team_scoreboard(
                &self.connected_players,
                player_id,
                &self.match_player_ids,
                self.team_score,
            );
        } else if is_key_down(KeyCode::Tab) {
            crate::ui::hud::draw_player_list(
                &self.connected_players,
                player_id,
                self.local_kills,
                self.local_deaths,
                (self.weapons.active_slot + 1) as u8,
                self.current_world == WorldId::Maze,
                self.match_mode,
            );

            // FPS counter — averaged over 0.5s window so the number is stable
            let dt = get_frame_time();
            self.fps_frame_count += 1;
            self.fps_time_acc += dt;
            if self.fps_time_acc >= 0.5 {
                self.fps_display = (self.fps_frame_count as f32 / self.fps_time_acc).round() as i32;
                self.fps_frame_count = 0;
                self.fps_time_acc = 0.0;
            }
            let fps = self.fps_display;
            let fps_text = format!("{} FPS", fps);
            let text_w = crate::ui::fonts::measure_game_text(&fps_text, 14);
            let pad = 10.0;
            let pill_w = text_w + pad * 2.0;
            let pill_h = 24.0;
            let pill_x = screen_width() - pill_w - 12.0;
            let pill_y = 12.0;
            draw_rectangle(pill_x, pill_y, pill_w, pill_h, Color::from_rgba(8, 10, 18, 200));
            draw_rectangle_lines(pill_x, pill_y, pill_w, pill_h, 1.0, Color::from_rgba(255, 255, 255, 30));
            let fps_color = if fps >= 60 {
                Color::from_rgba(100, 220, 100, 255)
            } else if fps >= 30 {
                Color::from_rgba(220, 180, 60, 255)
            } else {
                Color::from_rgba(220, 80, 80, 255)
            };
            draw_text_ex(
                &fps_text,
                pill_x + pad,
                pill_y + 17.0,
                crate::ui::fonts::text_params(14, fps_color),
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

        // Kill confirmation effect — brief white vignette flash when you get a kill
        if self.kill_confirm_timer > 0.0 {
            let t = self.kill_confirm_timer / 0.3; // 0..1 normalized
            let alpha = (t * 60.0) as u8; // subtle, max 60 alpha
            let sw = screen_width();
            let sh = screen_height();
            // top edge
            draw_rectangle(0.0, 0.0, sw, sh * 0.08, Color::from_rgba(255, 255, 255, alpha));
            // bottom edge
            draw_rectangle(0.0, sh * 0.92, sw, sh * 0.08, Color::from_rgba(255, 255, 255, alpha));
            // left edge
            draw_rectangle(0.0, 0.0, sw * 0.06, sh, Color::from_rgba(255, 255, 255, alpha));
            // right edge
            draw_rectangle(sw * 0.94, 0.0, sw * 0.06, sh, Color::from_rgba(255, 255, 255, alpha));
        }

        let is_snd = self.match_mode == Some(crate::net::protocols::MODE_SEARCH_AND_DESTROY)
            && self.current_world == WorldId::Maze
            && self.snd_round > 0;
        // Death camera: show gradual darkening during Watching phase + "killed by" text
        {
            use crate::screens::playing::state::DeathCameraPhase;
            match self.death_camera_phase {
                DeathCameraPhase::ZoomOut { timer, .. } => {
                    // Slight vignette that builds during zoom-out
                    let t = (timer / 0.5).clamp(0.0, 1.0);
                    let alpha = (t * 40.0) as u8;
                    draw_rectangle(
                        0.0, 0.0, screen_width(), screen_height(),
                        Color::from_rgba(0, 0, 0, alpha),
                    );
                }
                DeathCameraPhase::Watching { timer, .. } => {
                    // Gradual darkening over the 2s watching period
                    let t = (timer / 2.0).clamp(0.0, 1.0);
                    let alpha = (40.0 + t * 160.0) as u8;
                    draw_rectangle(
                        0.0, 0.0, screen_width(), screen_height(),
                        Color::from_rgba(0, 0, 0, alpha),
                    );
                    // "Killed by" text during watching phase
                    if let Some(killer) = &self.death_killer_name {
                        let txt = if let Some(weapon) = &self.death_weapon_name {
                            format!("Killed by {} with {}", killer, weapon)
                        } else {
                            format!("Killed by {}", killer)
                        };
                        let tw = crate::ui::fonts::measure_game_text(&txt, 22);
                        let text_alpha = ((timer / 0.5).clamp(0.0, 1.0) * 255.0) as u8;
                        draw_text_ex(
                            &txt,
                            (screen_width() - tw) / 2.0,
                            screen_height() * 0.15,
                            crate::ui::fonts::text_params(
                                22,
                                Color::from_rgba(255, 160, 160, text_alpha),
                            ),
                        );
                    }
                }
                _ => {}
            }
        }

        // Only show the full death screen overlay once the death camera is done
        let death_cam_done = matches!(
            self.death_camera_phase,
            crate::screens::playing::state::DeathCameraPhase::Done
                | crate::screens::playing::state::DeathCameraPhase::None
        );
        if self.is_dead && self.match_end_countdown_timer <= 0.0 && !is_snd && death_cam_done {
            self.draw_death_screen();
        }

        // Draw kill feed on top of death screen so it stays visible when dead
        if self.is_dead {
            let local_team = self
                .connected_players
                .iter()
                .find(|(id, ..)| *id == player_id)
                .map(|(_, _, _, _, _, team, _, _)| *team)
                .unwrap_or(0u8);
            crate::ui::hud::draw_kill_feed(
                &self.kill_feed,
                &self.assets,
                player_id,
                local_team,
            );
        }

        // S&D HUD overlay
        if is_snd {
            self.draw_snd_hud(player_id);
        }

        self.chat.draw();
    }
}
