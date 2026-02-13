mod movement;
mod camera;
mod rendering;
mod teleporter;

use crate::core::{ GameConfig, GameState };
use crate::physics::{ collision, movement::{ MovementConfig, MovementState } };
use crate::protocols::*;
use crate::screens::target_dummy::TargetDummy;
use crate::weapons::combat::{ CombatEntity, HitZone, damage_falloff };
use crate::types::maze::Maze;
use crate::types::player::OtherPlayer;
use crate::types::world::WorldId;
use crate::ui::hud::draw_hud;
use crate::weapons::effects::{ BulletTracer, DamageNumber };
use crate::weapons::recoil::{ apply_shot_recoil, recover };
use crate::weapons::{ recoil, SpreadContext, WeaponInventory };
use crate::world::{ draw_maze, lobby::draw_lobby };

use macroquad::prelude::scene::camera_pos;
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use std::net::UdpSocket;

use std::collections::HashMap;

use super::dev_mode::DevModeState;
use super::input;

pub struct PlayingState {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub last_mouse_pos: (f32, f32),
    pub other_players: Vec<OtherPlayer>,
    pub movement_state: MovementState,
    pub movement_config: MovementConfig,
    pub bob_timer: f32,
    pub assets: HashMap<String, Texture2D>,
    pub move_dir: Vec3,
    pub jump_requested: bool,
    pub current_world: WorldId,
    pub pending_world_change: Option<WorldId>,

    //  --- Weapon system ---
    pub weapons: WeaponInventory,

    // --- NPC target dummy ---
    pub target_dummy: TargetDummy,
    pub hit_marker_timer: f32, // >0 = show hit X on crosshair
    pub hit_marker_headshot: bool, // true = headshot color
    pub damage_numbers: Vec<DamageNumber>,
    pub tracers: Vec<BulletTracer>,

    pub hit_marker_pellet_mask: u16, // bitmasks of shotgun pellets hit, up to 16 | stored as 

    // --- Dev mode ---
    pub dev_mode: DevModeState,
}

impl PlayingState {
    pub async fn new() -> Self {
        Self {
            position: vec3(0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            last_mouse_pos: (0.0, 0.0),
            other_players: Vec::new(),
            movement_state: MovementState::new(),
            movement_config: MovementConfig::new(),
            bob_timer: 0.0,
            assets: HashMap::new(),
            move_dir: Vec3::ZERO,
            jump_requested: false,
            current_world: WorldId::Lobby,
            pending_world_change: None,
            weapons: WeaponInventory::new(),
            target_dummy: TargetDummy::new(vec3(-3.0, 0.0, -3.0)),
            hit_marker_timer: 0.0,
            hit_marker_headshot: false,
            damage_numbers: Vec::new(),
            tracers: Vec::new(),
            hit_marker_pellet_mask: 0,
            dev_mode: DevModeState::new(),
        }
    }

    // process local input and return state transition if any
    pub fn process_local_input(
        &mut self,
        config: &GameConfig,
        maze: Option<&Maze>
    ) -> Option<GameState> {
        if config.bind_settings.is_pressed() {
            return Some(GameState::Paused);
        }

        // ADS sensitivity: apply per-weapon multiplier AND FOV compensation
        // FOV compensation makes sensitivity feel natural at any zoom level
        // by scaling based on tan(ads_fov/2) / tan(base_fov/2)
        let weapon = self.weapons.active();
        let stats = weapon.stats();
        let ads_mult = stats.ads_sensitivity.unwrap_or(1.0);
        let base_fov_rad = config.raw_fov().to_radians();
        let ads_fov_rad = stats.ads_zoom_fov.to_radians();
        let fov_scale = (ads_fov_rad / 2.0).tan() / (base_fov_rad / 2.0).tan();
        // lerp from 1.0 (hip) to fov_scale * ads_mult (full ADS)
        let ads_factor = 1.0 + (fov_scale * ads_mult - 1.0) * weapon.ads_progress;
        let sens = config.raw_sensitivity() * ads_factor;

        input::process_mouse(&mut self.last_mouse_pos, &mut self.yaw, &mut self.pitch, sens);

        self.move_dir = input::process_movement_keys(self.yaw, config);
        self.movement_state.is_sprinting = input::process_sprint(
            config,
            &self.movement_state,
            self.move_dir
        );
        self.movement_state.is_crouching = input::process_crouch(
            config,
            self.movement_state.is_crouching
        );
        self.jump_requested = input::process_jump(
            config,
            self.movement_state.is_grounded,
            &mut self.weapons
        );

        // weapon input
        input::process_weapon_input(&mut self.weapons, config);

        // recover recoil/bloom from previous frame BEFORE firing
        // so new bloom from this frame's shot survives to render
        let delta = get_frame_time();
        let vel = self.movement_state.velocity;
        let is_moving = vec2(vel.x, vel.z).length() > 0.5;
        recover(self.weapons.active_mut(), delta, is_moving);

        // Compute effective spread for crosshair sizing and shot deviation.
        // Base = lerp(hip_spread, ads_spread, ads_progress) + bloom from firing.
        // Then multiplied by movement context: moving ~1.2-1.4x, sprinting ~1.6-3x,
        // crouching ~0.6-0.9x (tighter), airborne ~1.5x. Stacks multiplicatively.
        let vel = self.movement_state.velocity;
        let ctx = SpreadContext {
            is_moving: vec2(vel.x, vel.z).length() > 0.5,
            is_sprinting: self.movement_state.is_sprinting,
            is_crouching: self.movement_state.is_crouching,
            is_airborne: !self.movement_state.is_grounded,
        };
        let spread = recoil::current_spread(self.weapons.active(), &ctx);
        self.weapons.active_mut().computed_spread = spread;

        // firing
        let weapon = self.weapons.active_mut();
        if config.bind_fire.is_down() {
            if weapon.fire() {
                // dev mode: infinite ammo — refill after each shot
                if self.dev_mode.infinite_ammo {
                    weapon.ammo = weapon.stats_data.magazine_size;
                    weapon.is_reloading = false;
                    weapon.reload_timer = 0.0;
                }

                // recoil
                let rng = gen_range(0.0f32, 1.0);
                let fsa_shot = if self.dev_mode.no_recoil {
                    // skip bloom/recoil accumulation entirely
                    false
                } else {
                    apply_shot_recoil(weapon, rng)
                };

                let stats = weapon.stats();

                // compute shot direction BEFORE camera kick so the bullet
                // goes where the crosshair is, not where recoil moves it
                let eye = vec3(
                    self.position.x,
                    self.position.y + self.movement_state.height,
                    self.position.z
                );
                let base_fwd = vec3(
                    self.yaw.cos() * self.pitch.cos(),
                    self.pitch.sin(),
                    self.yaw.sin() * self.pitch.cos()
                );

                // NOW apply camera kick (visual recoil for next frame)
                // skip if FSA was active or dev no_recoil is on
                if !fsa_shot && !self.dev_mode.no_recoil {
                    let pattern = stats.recoil_pattern;
                    let idx = weapon.shot_index.saturating_sub(1) as usize; // -1 because we just incremented, saturating is used to avoid going below zero
                    let (ptrn_pitch, ptrn_yaw) = if pattern.is_empty() {
                        (stats.recoil_vertical, 0.0)
                    } else {
                        pattern[idx.min(pattern.len() - 1)]
                    };

                    let rand = (
                        stats.pattern_randomness +
                        (idx as f32) * stats.randomness_growth
                    ).clamp(0.0, 1.0);

                    self.pitch += ptrn_pitch;
                    let ptrn_h = ptrn_yaw * stats.recoil_horizontal;
                    let rand_h = (rng - 0.5) * 2.0 * stats.recoil_horizontal;
                    self.yaw += ptrn_h * (1.0 - rand) + rand_h * rand;
                    self.pitch = self.pitch.clamp(-1.55, 1.55);
                }

                // hitscan: fire pellet_count rays, each with independent spread.
                // For most weapons pellet_count=1; shotgun fires 8 pellets.
                if self.current_world == WorldId::Lobby {
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

                    for i in 0..stats.pellet_count as usize {
                        let mut fwd = base_fwd;
                        // follow fixed spread pattern
                        if spread > 0.0001 {
                            if let Some(ptrn) = stats.pellet_spread {
                                let (angle, radius) = ptrn.get(i).copied().unwrap_or((0.0, 0.0));
                                let deviation = spread * radius;
                                fwd = (fwd + right_vec * (angle.cos() * deviation) + up_vec * (angle.sin() * deviation)).normalize();
                            } else {
                                // random spread if no pattern specified (not recommended for competitive weapons since it's less consistent)
                                let angle = gen_range(0.0f32, std::f32::consts::TAU);
                                let radius = gen_range(0.0f32, spread);
                                fwd = (fwd + right_vec * (angle.cos() * radius) + up_vec * (angle.sin() * radius)).normalize();
                            }
                        }
                        

                        // bullet tracer — tiny offset so billboard quad is never perfectly edge-on
                        let tracer_start = eye + fwd * 1.5 + vec3(0.0, -0.05, 0.0);
                        self.tracers.push(BulletTracer {
                            start: tracer_start,
                            end: eye + fwd * range,
                            lifetime: stats.tracer_lifetime,
                            speed: stats.bullet_speed,
                            progress: 0.0,
                            tracer_length: stats.tracer_length,
                            tracer_color: stats.tracer_color,
                            tracer_thickness: stats.tracer_thickness,
                        });

                        let mut closest: Option<(usize, HitZone, f32)> = None;

                        // check hit on target dummy
                        if let Some((hit_zone, dist)) = self.target_dummy.ray_test(eye, fwd, range) {
                            closest = Some((0, hit_zone, dist));
                        }

                        // check for hit on other players
                        for (i, p) in self.other_players.iter().enumerate() {
                            if let Some((hit_zone, dist)) = p.ray_test(eye, fwd, range) {
                                // check if this hit is closer than the best hit we have so far, if so it becomes the new best
                                let is_closer = closest.map_or(true, |(_, _, d)| dist < d);
                                if is_closer {
                                    closest = Some((i + 1, hit_zone, dist));
                                }
                            }
                        }

                        // apply damage to appropriate entity
                        if let Some((target_idx, zone, dist)) = closest {
                            let mult = if zone == HitZone::Head { stats.headshot_mult } else { zone.damage_mult() };
                            let falloff = damage_falloff(dist, stats);
                            let dmg = damage * mult * falloff;
                            if target_idx == 0 {
                                if !self.dev_mode.dummy_invincible {
                                    self.target_dummy.take_damage(dmg);
                                }
                            } else {
                                self.other_players[target_idx - 1].take_damage(dmg);
                            }
                            total_dmg += dmg;
                            if zone == HitZone::Head {
                                any_headshot = true;
                            }
                            pellet_mask |= 1 << i; // if this pellet hit, you set the bit mark to one, << is left shift operator, so it sets it left by i, bit 1 is at i zero so it is the first bit if it hits, looks like 0 0 0 0 0 0 0 1 (if 8 pellets) and so on
                        }
                    }

                    // show hit marker + damage number for the combined total
                    if total_dmg > 0.0 {
                        self.hit_marker_timer = 0.35;
                        self.hit_marker_headshot = any_headshot;

                        // pellet shit
                        self.hit_marker_pellet_mask = pellet_mask;
                        self.damage_numbers.push(DamageNumber {
                            amount: total_dmg,
                            is_headshot: any_headshot,
                            screen_x: cx + gen_range(-20.0f32, 20.0),
                            screen_y: cy - 30.0,
                            lifetime: 1.0,
                        });
                    }
                }
            }
            weapon.trigger_held = true;
        } else {
            weapon.trigger_held = false;
        }

        // check teleporter input
        self.check_teleporter(maze);

        None
    }

    // run physics simulation
    pub fn simulate(&mut self, config: &GameConfig, maze: Option<&Maze>) {
        // updated this function to handle different physics for lobby vs maze, and also added a pending_world_change field to PlayingState to handle world transitions smoothly
        let delta = get_frame_time();

        // weapon tick (cooldowns, reloads, recoil recovery, etc)
        self.weapons.update(delta);
        // recover() is called in process_local_input before firing
        // camera recoil is applied directly on fire in process_local_input

        // NPC + hit marker tick
        self.target_dummy.update(delta);
        if self.hit_marker_timer > 0.0 {
            self.hit_marker_timer = (self.hit_marker_timer - delta).max(0.0);
        }

        // damage numbers: drift upward and fade
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
        self.apply_jump();
        self.apply_gravity(delta);

        match self.current_world {
            WorldId::Lobby => {
                collision::resolve_vertical(
                    &mut self.position,
                    &mut self.movement_state.velocity,
                    self.movement_state.height,
                    &mut self.movement_state.is_grounded
                );
                self.apply_movement(delta);
                collision::resolve_horizontal(
                    &mut self.position,
                    &mut self.movement_state.velocity,
                    self.movement_state.height
                );
            }
            WorldId::Maze => {
                // Simple ground check (no lobby cubes in maze)
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
                        m
                    );
                }
            }
        }
    }

    pub fn send_network(&mut self, socket: &Option<UdpSocket>, server_addr: &str) {
        if let Some(ref s) = socket {
            // send world change request if pending
            if let Some(target) = self.pending_world_change.take() {
                let msg = vec![MSG_WORLD_CHANGE, target.as_u8()];
                let _ = s.send_to(&msg, server_addr);
            }

            let mut msg = vec![MSG_POSITION];
            msg.extend_from_slice(&self.position.x.to_be_bytes());
            msg.extend_from_slice(&self.position.y.to_be_bytes());
            msg.extend_from_slice(&self.position.z.to_be_bytes());
            msg.extend_from_slice(&self.yaw.to_be_bytes());
            let _ = s.send_to(&msg, server_addr);
        }
    }

    pub fn render(
        // updated this function to take current world and render accordingly, also added a pending_world_change field to PlayingState to handle world transitions smoothly
        &mut self,
        config: &GameConfig,
        username: &str,
        player_id: u32,
        maze: Option<&Maze>
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
                    draw_maze(Some(m));
                }
            }
        }

        // teleporter
        self.draw_teleporter(maze);

        // NPC target dummy (only in lobby)
        if self.current_world == WorldId::Lobby {
            self.draw_target_dummy();
        }

        self.draw_other_players();

        // bullet tracer
        self.draw_tracers(camera.position);

        set_default_camera();

        // NPC health bar (2D overlay, projected from 3D)
        if self.current_world == WorldId::Lobby {
            self.draw_target_dummy_hud(&camera);
        }

        // has to be after set_default_camera since it's 2D text in a 3d world, also has to be after draw_other_players so that the prompt is on top of other players in case they are between the player and the teleporter
        self.draw_teleporter_prompt(maze);

        let forward = vec3(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos()
        );
        let right = vec3(-self.yaw.sin(), 0.0, self.yaw.cos());

        draw_hud(
            username,
            player_id,
            self.position,
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
            self.hit_marker_pellet_mask
        );
    }

    // clear movement intent (for pause) just means that we stop applying movement input, but we still want to simulate physics so that the player doesn't get stuck in walls or fall through the floor when they unpause
    pub fn clear_input(&mut self) {
        self.move_dir = Vec3::ZERO;
        self.movement_state.is_sprinting = false;
        self.jump_requested = false;
    }
}
