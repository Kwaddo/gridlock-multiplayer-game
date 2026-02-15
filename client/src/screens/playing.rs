mod camera;
mod movement;
mod rendering;
mod teleporter;

use crate::core::{GameConfig, GameState};
use crate::physics::{
    collision,
    movement::{MovementConfig, MovementState},
};
use crate::protocols::*;
use crate::screens::target_dummy::TargetDummy;
use crate::types::ai::simulated_player::SimulatedPlayer;
use crate::types::maze::Maze;
use crate::types::player::OtherPlayer;
use crate::types::world::WorldId;
use crate::ui::hud::draw_hud;
use crate::weapons::combat::{damage_falloff, CombatEntity, EntityKind, HitZone};
use crate::weapons::effects::{BulletTracer, DamageNumber};
use crate::weapons::recoil::{apply_shot_recoil, recover};
use crate::weapons::{recoil, SpreadContext, WeaponInventory};
use crate::world::{draw_maze, lobby::draw_lobby};

use macroquad::prelude::*;
use macroquad::rand::gen_range;
use std::net::UdpSocket;

use macroquad::models::Mesh;
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
    pub models: HashMap<String, Mesh>,
    pub move_dir: Vec3,
    pub jump_requested: bool,
    pub current_world: WorldId,
    pub pending_world_change: Option<WorldId>,
    pub pending_broken_walls: Vec<(u8, u8)>,
    pub pending_respawn: bool,

    //  --- Weapon system ---
    pub weapons: WeaponInventory,

    // --- NPC target dummy ---
    pub target_dummy: TargetDummy,
    pub hit_marker_timer: f32,     // >0 = show hit X on crosshair
    pub hit_marker_headshot: bool, // true = headshot color
    pub damage_numbers: Vec<DamageNumber>,
    pub tracers: Vec<BulletTracer>,

    pub hit_marker_pellet_mask: u16, // bitmasks of shotgun pellets hit, up to 16 | stored as

    // --- Dev mode ---
    pub dev_mode: DevModeState,

    // --- Practice bots ---
    pub practice_bots: Vec<SimulatedPlayer>,
    pub next_bot_id: u32,

    // --- Health tracking ---
    pub hp: f32,
    pub shield: f32,
    pub max_shield:f32,
    pub max_hp: f32,
    pub is_dead: bool,
    pub death_timer: f32, // time before showing respawn prompt
    pub invulnerability_timer: f32, // time after taking damage during which player can't be hurt again

    // --- Kill feed ---
    pub kill_feed: Vec<KillFeedEntry>,
}

pub struct KillFeedEntry {
    pub killer_id: u32,
    pub victim_id: u32,
    pub weapon: String, // weapon name
    pub timer: f32, // how long to show (counts down)
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
            models: HashMap::new(),
            move_dir: Vec3::ZERO,
            jump_requested: false,
            current_world: WorldId::Lobby,
            pending_world_change: None,
            pending_broken_walls: Vec::new(),
            pending_respawn: false,
            weapons: WeaponInventory::new(),
            target_dummy: TargetDummy::new(vec3(-3.0, 0.0, -3.0)),
            hit_marker_timer: 0.0,
            hit_marker_headshot: false,
            damage_numbers: Vec::new(),
            tracers: Vec::new(),
            hit_marker_pellet_mask: 0,
            dev_mode: DevModeState::new(),
            practice_bots: Vec::new(),
            next_bot_id: 9000,
            hp: 100.0,
            max_hp: 100.0,
            shield: 0.0,
            max_shield: 100.0,
            is_dead: false,
            death_timer: 0.0,
            invulnerability_timer: 0.0,
            kill_feed: Vec::new(),
        }
    }

    // process local input and return state transition if any
    pub fn process_local_input(
        &mut self,
        config: &GameConfig,
        mut maze: Option<&mut Maze>,
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
                    self.position.z,
                );
                let base_fwd = vec3(
                    self.yaw.cos() * self.pitch.cos(),
                    self.pitch.sin(),
                    self.yaw.sin() * self.pitch.cos(),
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

                    let rand = (stats.pattern_randomness + (idx as f32) * stats.randomness_growth)
                        .clamp(0.0, 1.0);

                    self.pitch += ptrn_pitch;
                    let ptrn_h = ptrn_yaw * stats.recoil_horizontal;
                    let rand_h = (rng - 0.5) * 2.0 * stats.recoil_horizontal;
                    self.yaw += ptrn_h * (1.0 - rand) + rand_h * rand;
                    self.pitch = self.pitch.clamp(-1.55, 1.55);
                }

                // hitscan: fire pellet_count rays, each with independent spread.
                // For most weapons pellet_count=1; shotgun fires 8 pellets.
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
                            fwd = (fwd
                                + right_vec * (angle.cos() * deviation)
                                + up_vec * (angle.sin() * deviation))
                                .normalize();
                        } else {
                            // random spread if no pattern specified (not recommended for competitive weapons since it's less consistent)
                            let angle = gen_range(0.0f32, std::f32::consts::TAU);
                            let radius = gen_range(0.0f32, spread);
                            fwd = (fwd
                                + right_vec * (angle.cos() * radius)
                                + up_vec * (angle.sin() * radius))
                                .normalize();
                        }
                    }

                    // Check if bullet hits any walls or props (maze world only)
                    let closest_blocking_dist = if self.current_world == WorldId::Maze {
                        let wall_hit = maze
                            .as_ref()
                            .and_then(|m| m.ray_test_walls(eye, fwd, range));
                        let prop_hit_dist = maze
                            .as_ref()
                            .and_then(|m| m.ray_test_props(eye, fwd, range));

                        // Determine closest hit (wall or prop)
                        match (wall_hit, prop_hit_dist) {
                            (Some(((wx, wy), wall_dist)), Some(prop_dist)) => {
                                if wall_dist < prop_dist {
                                    // Wall is closer - apply damage if breakable
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
                                // Only wall was hit
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

                    // bullet tracer — tiny offset so billboard quad is never perfectly edge-on
                    // If wall/prop was hit, end the tracer there instead of at max range
                    let tracer_start = eye + fwd * 1.5 + vec3(0.0, -0.025, 0.0);
                    let tracer_end = if let Some(dist) = closest_blocking_dist {
                        eye + fwd * dist
                    } else {
                        eye + fwd * range
                    };

                    self.tracers.push(BulletTracer {
                        start: tracer_start,
                        end: tracer_end,
                        lifetime: stats.tracer_lifetime,
                        speed: stats.bullet_speed,
                        progress: 0.0,
                        tracer_length: stats.tracer_length,
                        tracer_color: stats.tracer_color,
                        tracer_thickness: stats.tracer_thickness,
                    });

                    // --- Unified entity hit detection ---
                    // Build a list of references to all combat entities, then test them all in one loop
                    let mut closest: Option<(EntityKind, usize, HitZone, f32)> = None;

                    // test target dummy (lobby only)
                    if self.current_world == WorldId::Lobby {
                        if let Some((zone, dist)) = self.target_dummy.ray_test(eye, fwd, range) {
                            let blocked = closest_blocking_dist.map_or(false, |d| d < dist);
                            if !blocked {
                                closest = Some((EntityKind::Dummy, 0, zone, dist));
                            }
                        }
                    }

                    // test all other players (any world)
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

                    // test all practice bots
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

                    // apply damage to whichever entity was closest
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
                                self.other_players[idx].take_damage(dmg);
                            }
                            EntityKind::Bot => {
                                self.practice_bots[idx].take_damage(dmg);
                            }
                            _ => {} // MovingTarget — future entity types
                        }

                        total_dmg += dmg;
                        if zone == HitZone::Head {
                            any_headshot = true;
                        }
                        pellet_mask |= 1 << i;
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
            weapon.trigger_held = true;
        } else {
            weapon.trigger_held = false;
        }

        // check teleporter input
        self.check_teleporter(maze.as_ref().map(|m| &**m));

        None
    }

    // run physics simulation
    pub fn simulate(&mut self, config: &GameConfig, maze: Option<&Maze>) {
        // updated this function to handle different physics for lobby vs maze, and also added a pending_world_change field to PlayingState to handle world transitions smoothly
        let delta = get_frame_time();

        // Death timer countdown and respawn input
        if self.is_dead {
            if self.death_timer > 0.0 {
                self.death_timer -= delta;
            } else {
                // Death timer expired - allow respawn in Lobby with SPACE
                if self.current_world == WorldId::Lobby && is_key_pressed(KeyCode::Space) {
                    // Set a flag to send respawn request in send_network
                    self.pending_respawn = true;
                }
            }
            // Don't process input/movement while dead
            return;
        }

        // Decrement kill feed timers
        for entry in &mut self.kill_feed {
            entry.timer -= delta;
        }
        self.kill_feed.retain(|entry| entry.timer > 0.0);

        // weapon tick (cooldowns, reloads, recoil recovery, etc)
        self.weapons.update(delta);
        // recover() is called in process_local_input before firing
        // camera recoil is applied directly on fire in process_local_input

        // entity tick (dummy regen, player hit flash decay, etc.)
        self.target_dummy.update(delta);
        for p in &mut self.other_players {
            p.update(delta);
        }
        for bot in &mut self.practice_bots {
            bot.update(delta);
        }
        // respawn dead bots
        self.practice_bots.retain(|bot| !bot.is_dead());
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
                        m,
                    );
                }
            }
        }
    }

    pub fn send_network(&mut self, socket: &Option<UdpSocket>, server_addr: &str) {
        if let Some(ref s) = socket {
            // send respawn request if pending
            if self.pending_respawn {
                self.pending_respawn = false;
                self.request_respawn(s, server_addr);
            }

            // send world change request if pending
            if let Some(target) = self.pending_world_change.take() {
                let msg = vec![MSG_WORLD_CHANGE, target.as_u8()];
                let _ = s.send_to(&msg, server_addr);
            }

            while let Some((x, y)) = self.pending_broken_walls.pop() {
                let msg = vec![MSG_WALL_BROKEN, x, y];
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
                    draw_maze(Some(m), self.position, config.render_distance);
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

        // Dev mode: health bars for all entities
        self.draw_entity_health_bars(&camera);

        // has to be after set_default_camera since it's 2D text in a 3d world, also has to be after draw_other_players so that the prompt is on top of other players in case they are between the player and the teleporter
        self.draw_teleporter_prompt(maze);

        // first-person weapon sprite (drawn before HUD so crosshair/ammo sit on top)
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
        );

        // Draw death screen overlay if dead
        if self.is_dead {
            self.draw_death_screen();
        }
    }

    // clear movement intent (for pause) just means that we stop applying movement input, but we still want to simulate physics so that the player doesn't get stuck in walls or fall through the floor when they unpause
    pub fn clear_input(&mut self) {
        self.move_dir = Vec3::ZERO;
        self.movement_state.is_sprinting = false;
        self.jump_requested = false;
    }

    // Function for player (you) to take damage
    pub fn take_damage(&mut self, damage: f32, attacker_id: Option<u32>) { // attacker ID to check who killed u if the damage u took lead to your death

        // invinvibility checks
        if self.dev_mode.god_mode || self.is_dead || self.invulnerability_timer > 0.0 {
            return;
        }

        // apply damage to shields first if available
        if self.shield > 0.0 {
            let shield_damage = damage.min(self.shield); // .min so we dont go below zero, remaining damage will be applied to hp
            self.shield -= shield_damage;
            let remaining_damage = damage - shield_damage; // the new shield_damage will be the remainder of total damage - damage removed from shields

            if remaining_damage > 0.0 {
                self.hp -= remaining_damage;
            }
        } else {
            self.hp = (self.hp - damage).max(0.0);
        }

        // TODO
        // Screen shake, damage effects, different effects for shield vs hp damage, etc.


        if self.hp <= 0.0  {
            self.die(attacker_id)
        }
    }
    pub fn die(&mut self, killer_id: Option<u32>) {
        self.is_dead = true;
        self.hp = 0.0;
        self.death_timer = 3.0; // show respawn prompt after 3 seconds

        // HERE: Send MSG_DEATH to server
        // PLAY: Death or sound animation
    }

    pub fn draw_death_screen(&self) {
        use crate::ui::fonts::{measure_game_text, text_params};

        // semi-transparent black overlay
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::from_rgba(0, 0, 0, 180),
        );

        let cx = screen_width() / 2.0;
        let cy = screen_height() / 2.0;

        // "YOU DIED" text
        let death_text = "YOU DIED";
        let text_w = measure_game_text(death_text, 48);
        draw_text_ex(
            death_text,
            cx - text_w / 2.0,
            cy - 80.0,
            text_params(48, RED),
        );

        // show respawn options based on world
        if self.current_world == WorldId::Lobby {
            // Lobby: instant respawn
            if self.death_timer <= 0.0 {
                let respawn_text = "Press SPACE to Respawn";
                let tw = measure_game_text(respawn_text, 24);
                draw_text_ex(
                    respawn_text,
                    cx - tw / 2.0,
                    cy + 20.0,
                    text_params(24, WHITE),
                );
            } else {
                let timer_text = format!("Respawning in {:.1}s", self.death_timer);
                let tw = measure_game_text(&timer_text, 24);
                draw_text_ex(&timer_text, cx - tw / 2.0, cy + 20.0, text_params(24, GRAY));
            }
        } else if self.current_world == WorldId::Maze {
            // Maze: game mode specific respawn can be handled here later
            if self.death_timer <= 0.0 {
                let respawn_text = "Waiting to Respawn...";
                let tw = measure_game_text(respawn_text, 24);
                draw_text_ex(
                    respawn_text,
                    cx - tw / 2.0,
                    cy + 20.0,
                    text_params(24, GRAY),
                );

                let hint_text = "(Game mode will respawn you)";
                let hw = measure_game_text(hint_text, 16);
                draw_text_ex(
                    hint_text,
                    cx - hw / 2.0,
                    cy + 50.0,
                    text_params(16, DARKGRAY),
                );
            } else {
                let timer_text = format!("{:.1}s", self.death_timer);
                let tw = measure_game_text(&timer_text, 48);
                draw_text_ex(
                    &timer_text,
                    cx - tw / 2.0,
                    cy + 20.0,
                    text_params(48, Color::from_rgba(255, 100, 100, 255)),
                );
            }
        }
    }

    pub fn request_respawn(&self, socket: &std::net::UdpSocket, server_addr: &str) {
        use crate::protocols::MSG_RESPAWN_REQUEST;
        let msg = vec![MSG_RESPAWN_REQUEST];
        let _ = socket.send_to(&msg, server_addr);
    }

    pub fn respawn(&mut self, spawn_pos: Vec3) {
        self.is_dead = false;
        self.hp = self.max_hp;
        self.shield = 0.0; // Respawn without shield
        self.death_timer = 0.0;
        self.position = spawn_pos;
        self.movement_state.velocity = Vec3::ZERO;
    }
}
