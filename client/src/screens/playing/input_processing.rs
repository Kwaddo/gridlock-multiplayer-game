use crate::core::{GameConfig, GameState};
use crate::screens::input;
use crate::types::maze::Maze;
use crate::types::world::WorldId;
use crate::weapons::combat::{damage_falloff, CombatEntity, EntityKind, HitZone};
use crate::weapons::effects::DamageNumber;
use crate::weapons::recoil::{apply_shot_recoil, recover};
use crate::weapons::{recoil, SpreadContext};

use macroquad::prelude::*;
use macroquad::rand::gen_range;
use rodio::{Decoder, Sink};
use std::collections::HashMap;
use std::io::Cursor;

use super::PlayingState;

impl PlayingState {
    pub fn process_local_input(
        &mut self,
        config: &GameConfig,
        mut maze: Option<&mut Maze>,
    ) -> Option<GameState> {
        let raw_delta = get_frame_time();
        if raw_delta > 0.20 {
            self.clear_input();
            self.move_dir = Vec3::ZERO;
            self.jump_requested = false;
            self.movement_state.velocity = Vec3::ZERO;
            self.last_mouse_pos = mouse_position();
            return None;
        }

        if config.bind_settings.is_pressed() {
            return Some(GameState::Paused);
        }

        if self.is_dead {
            self.clear_input();
            return None;
        }

        let weapon = self.weapons.active();
        let stats = weapon.stats();
        let ads_mult = stats.ads_sensitivity.unwrap_or(1.0);
        let base_fov_rad = config.raw_fov().to_radians();
        let ads_fov_rad = stats.ads_zoom_fov.to_radians();
        let fov_scale = (ads_fov_rad / 2.0).tan() / (base_fov_rad / 2.0).tan();
        let ads_factor = 1.0 + (fov_scale * ads_mult - 1.0) * weapon.ads_progress;
        let sens = config.raw_sensitivity() * ads_factor;

        input::process_mouse(
            &mut self.last_mouse_pos,
            &mut self.yaw,
            &mut self.pitch,
            sens,
        );

        self.move_dir = input::process_movement_keys(self.yaw, config);
        self.movement_state.is_sprinting =
            input::process_sprint(config, &self.movement_state, self.move_dir);
        self.movement_state.is_crouching =
            input::process_crouch(config, self.movement_state.is_crouching);
        self.jump_requested =
            input::process_jump(config, self.movement_state.is_grounded, &mut self.weapons);

        input::process_weapon_input(&mut self.weapons, config);

        let delta = raw_delta.min(0.05);
        let vel = self.movement_state.velocity;
        let is_moving = vec2(vel.x, vel.z).length() > 0.5;
        recover(self.weapons.active_mut(), delta, is_moving);

        let vel = self.movement_state.velocity;
        let ctx = SpreadContext {
            is_moving: vec2(vel.x, vel.z).length() > 0.5,
            is_sprinting: self.movement_state.is_sprinting,
            is_crouching: self.movement_state.is_crouching,
            is_airborne: !self.movement_state.is_grounded,
        };
        let spread = recoil::current_spread(self.weapons.active(), &ctx);
        self.weapons.active_mut().computed_spread = spread;

        let mut fire_sound_key: Option<&'static str> = None;
        let weapon = self.weapons.active_mut();
        if config.bind_fire.is_down() {
            if weapon.fire() {
                fire_sound_key = Some(weapon.stats().fire_sound_asset_key);

                if self.dev_mode.infinite_ammo {
                    weapon.ammo = weapon.stats_data.magazine_size;
                    weapon.is_reloading = false;
                    weapon.reload_timer = 0.0;
                }

                let rng = gen_range(0.0f32, 1.0);
                let fsa_shot = if self.dev_mode.no_recoil {
                    false
                } else {
                    apply_shot_recoil(weapon, rng)
                };

                let stats = weapon.stats();
                let eye = vec3(
                    self.position.x,
                    self.position.y + self.movement_state.height,
                    self.position.z,
                );
                let base_fwd = vec3(
                    self.yaw.cos() * self.pitch.cos(),
                    self.pitch.sin(),
                    self.yaw.sin() * self.pitch.cos(),
                );

                if !fsa_shot && !self.dev_mode.no_recoil {
                    let pattern = stats.recoil_pattern;
                    let idx = weapon.shot_index.saturating_sub(1) as usize;
                    let (ptrn_pitch, ptrn_yaw) = if pattern.is_empty() {
                        (stats.recoil_vertical, 0.0)
                    } else {
                        pattern[idx.min(pattern.len() - 1)]
                    };

                    let rand = (stats.pattern_randomness + (idx as f32) * stats.randomness_growth)
                        .clamp(0.0, 1.0);

                    self.pitch += ptrn_pitch;
                    let ptrn_h = ptrn_yaw * stats.recoil_horizontal;
                    let rand_h = (rng - 0.5) * 2.0 * stats.recoil_horizontal;
                    self.yaw += ptrn_h * (1.0 - rand) + rand_h * rand;
                    self.pitch = self.pitch.clamp(-1.55, 1.55);
                }

                let right_vec = vec3(-self.yaw.sin(), 0.0, self.yaw.cos());
                let up_vec = right_vec.cross(base_fwd).normalize();

                let damage = stats.damage;
                let range = stats.bullet_range;
                let spread = weapon.computed_spread;
                let cx = screen_width() / 2.0;
                let cy = screen_height() / 2.0;

                let mut total_dmg = 0.0f32;
                let mut any_headshot = false;
                let mut pellet_mask: u16 = 0;
                let mut player_damage_by_id: HashMap<u32, f32> = HashMap::new();

                for i in 0..stats.pellet_count as usize {
                    let mut fwd = base_fwd;
                    if spread > 0.0001 {
                        if let Some(ptrn) = stats.pellet_spread {
                            let (angle, radius) = ptrn.get(i).copied().unwrap_or((0.0, 0.0));
                            let deviation = spread * radius;
                            fwd = (fwd
                                + right_vec * (angle.cos() * deviation)
                                + up_vec * (angle.sin() * deviation))
                                .normalize();
                        } else {
                            let angle = gen_range(0.0f32, std::f32::consts::TAU);
                            let radius = gen_range(0.0f32, spread);
                            fwd = (fwd
                                + right_vec * (angle.cos() * radius)
                                + up_vec * (angle.sin() * radius))
                                .normalize();
                        }
                    }

                    let closest_blocking_dist = if self.current_world == WorldId::Maze {
                        let wall_hit = maze
                            .as_ref()
                            .and_then(|m| m.ray_test_walls(eye, fwd, range));
                        let prop_hit_dist = maze
                            .as_ref()
                            .and_then(|m| m.ray_test_props(eye, fwd, range));

                        match (wall_hit, prop_hit_dist) {
                            (Some(((wx, wy), wall_dist)), Some(prop_dist)) => {
                                if wall_dist < prop_dist {
                                    if let Some(m) = maze.as_mut() {
                                        if m.damage_wall(wx, wy, stats.damage) {
                                            if !self.pending_broken_walls.contains(&(wx, wy)) {
                                                self.pending_broken_walls.push((wx, wy));
                                            }
                                        }
                                    }
                                    Some(wall_dist)
                                } else {
                                    Some(prop_dist)
                                }
                            }
                            (Some(((wx, wy), wall_dist)), None) => {
                                if let Some(m) = maze.as_mut() {
                                    if m.damage_wall(wx, wy, stats.damage) {
                                        if !self.pending_broken_walls.contains(&(wx, wy)) {
                                            self.pending_broken_walls.push((wx, wy));
                                        }
                                    }
                                }
                                Some(wall_dist)
                            }
                            (None, Some(prop_dist)) => Some(prop_dist),
                            (None, None) => None,
                        }
                    } else {
                        None
                    };

                    let tracer_start = eye + fwd * 1.5 + vec3(0.0, -0.025, 0.0);
                    let tracer_end = if let Some(dist) = closest_blocking_dist {
                        eye + fwd * dist
                    } else {
                        eye + fwd * range
                    };

                    self.tracers.push(crate::weapons::effects::BulletTracer {
                        start: tracer_start,
                        end: tracer_end,
                        lifetime: stats.tracer_lifetime,
                        speed: stats.bullet_speed,
                        progress: 0.0,
                        tracer_length: stats.tracer_length,
                        tracer_color: stats.tracer_color,
                        tracer_thickness: stats.tracer_thickness,
                    });

                    let mut closest: Option<(EntityKind, usize, HitZone, f32)> = None;

                    if self.current_world == WorldId::Lobby {
                        if let Some((zone, dist)) = self.target_dummy.ray_test(eye, fwd, range) {
                            let blocked = closest_blocking_dist.map_or(false, |d| d < dist);
                            if !blocked {
                                closest = Some((EntityKind::Dummy, 0, zone, dist));
                            }
                        }
                    }

                    for (idx, p) in self.other_players.iter().enumerate() {
                        if let Some((zone, dist)) = p.ray_test(eye, fwd, range) {
                            let blocked = closest_blocking_dist.map_or(false, |d| d < dist);
                            if !blocked {
                                let is_closer = closest.map_or(true, |(_, _, _, d)| dist < d);
                                if is_closer {
                                    closest = Some((EntityKind::Player, idx, zone, dist));
                                }
                            }
                        }
                    }

                    for (idx, bot) in self.practice_bots.iter().enumerate() {
                        if let Some((zone, dist)) = bot.ray_test(eye, fwd, range) {
                            let blocked = closest_blocking_dist.map_or(false, |d| d < dist);
                            if !blocked {
                                let is_closer = closest.map_or(true, |(_, _, _, d)| dist < d);
                                if is_closer {
                                    closest = Some((EntityKind::Bot, idx, zone, dist));
                                }
                            }
                        }
                    }

                    if let Some((kind, idx, zone, dist)) = closest {
                        let mult = if zone == HitZone::Head {
                            stats.headshot_mult
                        } else {
                            zone.damage_mult()
                        };
                        let falloff = damage_falloff(dist, stats);
                        let dmg = damage * mult * falloff;

                        match kind {
                            EntityKind::Dummy => {
                                self.target_dummy.take_damage(dmg);
                            }
                            EntityKind::Player => {
                                if let Some(victim_id) = self.other_players[idx].entity_id() {
                                    *player_damage_by_id.entry(victim_id).or_insert(0.0) += dmg;
                                }
                            }
                            EntityKind::Bot => {
                                self.practice_bots[idx].take_damage(dmg);
                            }
                            _ => {}
                        }

                        total_dmg += dmg;
                        if zone == HitZone::Head {
                            any_headshot = true;
                        }
                        pellet_mask |= 1 << i;
                    }
                }

                if total_dmg > 0.0 {
                    self.hit_marker_timer = 0.35;
                    self.hit_marker_headshot = any_headshot;
                    self.hit_marker_pellet_mask = pellet_mask;
                    self.damage_numbers.push(DamageNumber {
                        amount: total_dmg,
                        is_headshot: any_headshot,
                        screen_x: cx + gen_range(-20.0f32, 20.0),
                        screen_y: cy - 30.0,
                        lifetime: 1.0,
                    });
                }

                for (victim_id, damage) in player_damage_by_id {
                    if damage > 0.0 {
                        self.pending_hits.push((victim_id, damage));
                    }
                }
            }
            weapon.trigger_held = true;
        } else {
            weapon.trigger_held = false;
        }

        if let Some(sound_key) = fire_sound_key {
            self.play_weapon_shot_sound(sound_key, config);
        }

        self.check_teleporter(maze.as_ref().map(|m| &**m));

        None
    }

    fn play_weapon_shot_sound(&self, sound_key: &str, config: &GameConfig) {
        if config.mute_audio {
            return;
        }
        let Some(sample_bytes) = self.sound_samples.get(sound_key) else {
            return;
        };
        let Some(audio_handle) = self.audio_stream_handle.as_ref() else {
            return;
        };

        let pitch = gen_range(0.94f32, 1.06f32);
        let volume = gen_range(0.70f32, 0.82f32);
        let cursor = Cursor::new(sample_bytes.clone());
        let Ok(source) = Decoder::new(cursor) else {
            return;
        };
        let Ok(sink) = Sink::try_new(audio_handle) else {
            return;
        };
        sink.set_speed(pitch);
        sink.set_volume(volume);
        sink.append(source);
        sink.detach();
    }
}
