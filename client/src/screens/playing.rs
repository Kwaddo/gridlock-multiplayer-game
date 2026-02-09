use crate::core::{GameConfig, GameState};
use crate::net::networking::MAZE_CELL_SIZE;
use crate::physics::{
    collision,
    movement::{MovementConfig, MovementState},
};
use crate::protocols::*;
use crate::screens::input::process_weapon_input;
use crate::types::maze::Maze;
use crate::types::player::OtherPlayer;
use crate::types::world::WorldId;
use crate::ui::hud::draw_hud;
use crate::weapons::recoil::{apply_shot_recoil, recover};
use crate::weapons::{recoil, SpreadContext, WeaponInventory};
use crate::world::{draw_maze, lobby::draw_lobby};

use macroquad::prelude::*;
use macroquad::rand::{gen_range, rand};
use std::net::UdpSocket;

use std::collections::HashMap;

use super::input;

// Floating Damage Numbers
pub struct DamageNumber {
    pub amount: f32,
    pub is_headshot: bool,
    pub screen_x: f32,
    pub screen_y: f32,
    pub lifetime: f32, // starts at 1.0, counts down
}

// bullet tracer

pub struct BulletTracer {
    pub start: Vec3,
    pub end: Vec3,
    pub lifetime: f32,
}

// Hit zones: split player into head, body, and limbs for multipliers
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HitZone {
    Head,
    Torso,
    Limbs,
}

impl HitZone {
    pub fn damage_mult(&self) -> f32 {
        match self {
            HitZone::Head => 2.0,
            HitZone::Torso => 1.0,
            HitZone::Limbs => 0.75,
        }
    }
}

pub struct Hitbox {
    pub center_offset: Vec3, // offset from player's position to center of this hitbox
    pub half_extents: Vec3,  // size of the hitbox (x=width/2, y=height/2, z=depth/2)
    pub zone: HitZone,       // which hitzone this box represents (head, torso, limbs)
}

// NPC Target Dummy
pub struct TargetDummy {
    pub position: Vec3, // feet position
    pub hp: f32,
    pub max_hp: f32,
    pub hit_timer: f32,        // >0 = flash red (seconds remaining)
    pub hitboxes: Vec<Hitbox>, // for more complex targets with multiple hit zones, but for simplicity we'll just do one head box and one body box
}

impl TargetDummy {
    pub fn new(pos: Vec3) -> Self {
        Self {
            position: pos,
            hp: 100.0,
            max_hp: 100.0,
            hit_timer: 0.0,
            hitboxes: vec![
                // Head
                Hitbox {
                    center_offset: vec3(0.0, 1.5, 0.0),
                    half_extents: vec3(0.15, 0.15, 0.15),
                    zone: HitZone::Head,
                },
                // Torso
                Hitbox {
                    center_offset: vec3(0.0, 0.9, 0.0),
                    half_extents: vec3(0.25, 0.35, 0.15),
                    zone: HitZone::Torso,
                },
                // Left arm
                Hitbox {
                    center_offset: vec3(-0.4, 0.9, 0.0),
                    half_extents: vec3(0.1, 0.3, 0.1),
                    zone: HitZone::Limbs,
                },
                // Right arm
                Hitbox {
                    center_offset: vec3(0.4, 0.9, 0.0),
                    half_extents: vec3(0.1, 0.3, 0.1),
                    zone: HitZone::Limbs,
                },
                // Left leg
                Hitbox {
                    center_offset: vec3(-0.12, 0.25, 0.0),
                    half_extents: vec3(0.1, 0.25, 0.1),
                    zone: HitZone::Limbs,
                },
                // Right leg
                Hitbox {
                    center_offset: vec3(0.12, 0.25, 0.0),
                    half_extents: vec3(0.1, 0.25, 0.1),
                    zone: HitZone::Limbs,
                },
            ],
        }
    }

    pub fn body_center(&self) -> Vec3 {
        self.position + vec3(0.0, 0.6, 0.0) // body center at waist height
    }

    pub fn head_center(&self) -> Vec3 {
        self.position + vec3(0.0, 1.4, 0.0) // head center
    }

    pub fn take_damage(&mut self, dmg: f32) {
        self.hp = (self.hp - dmg).max(1.0); // never dies, minimum 1 HP
        self.hit_timer = 0.3; // flash red for 300ms
    }

    pub fn update(&mut self, delta: f32) {
        if self.hit_timer > 0.0 {
            self.hit_timer = (self.hit_timer - delta).max(0.0);
        }
        // regen: heal back to full over 3 seconds when not being hit
        if self.hit_timer <= 0.0 && self.hp < self.max_hp {
            self.hp = (self.hp + 20.0 * delta).min(self.max_hp);
        }
    }

    pub fn ray_test(&self, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<HitZone> {
        // check all hitboxes against a ray and return first hit zone found
        for hit_box in &self.hitboxes {
            let center = self.position + hit_box.center_offset;
            if ray_hit_aabb(origin, dir, center, hit_box.half_extents, max_dist) {
                return Some(hit_box.zone);
            }
        }
        None
    }
}

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
    pub hit_marker_timer: f32,     // >0 = show hit X on crosshair
    pub hit_marker_headshot: bool, // true = headshot color
    pub damage_numbers: Vec<DamageNumber>,
    pub tracers: Vec<BulletTracer>,

    pub hit_marker_pellets_hit: u32, // for shotguns: how many pellets hit, to show different hit marker icons (1-2 = small, 3-5 = medium, 6-8 = large)
    pub hit_marker_pellets_total: u32, // for shotguns: total pellets fired, to know when to reset hit marker state on next shot
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
            hit_marker_pellets_hit: 0,
            hit_marker_pellets_total: 0,
        }
    }

    // process local input and return state transition if any
    pub fn process_local_input(
        &mut self,
        config: &GameConfig,
        maze: Option<&Maze>,
    ) -> Option<GameState> {
        if is_key_pressed(config.key_settings) {
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
        self.jump_requested = input::process_jump(config, self.movement_state.is_grounded);

        // weapon input
        input::process_weapon_input(&mut self.weapons, config);

        // recover recoil/bloom from previous frame BEFORE firing
        // so new bloom from this frame's shot survives to render
        let delta = get_frame_time();
        recover(self.weapons.active_mut(), delta);

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
                // recoil
                let rng = gen_range(0.0f32, 1.0);
                apply_shot_recoil(weapon, rng);

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
                self.pitch += stats.recoil_vertical;
                let h_kick = (rng - 0.5) * 2.0 * stats.recoil_horizontal;
                self.yaw += h_kick;
                self.pitch = self.pitch.clamp(-1.55, 1.55);

                // hitscan: fire pellet_count rays, each with independent spread.
                // For most weapons pellet_count=1; shotgun fires 8 pellets.
                if self.current_world == WorldId::Lobby {
                    let right_vec = vec3(-self.yaw.sin(), 0.0, self.yaw.cos());
                    let up_vec = right_vec.cross(base_fwd).normalize();

                    let damage = stats.damage;
                    let hs_mult = stats.headshot_mult;
                    let range = stats.bullet_range;
                    let spread = weapon.computed_spread;
                    let cx = screen_width() / 2.0;
                    let cy = screen_height() / 2.0;

                    let mut total_dmg = 0.0f32;
                    let mut any_headshot = false;

                    let mut pellets_hit = 0u32;

                    for _ in 0..stats.pellet_count {
                        let mut fwd = base_fwd;
                        // each pellet gets its own random spread
                        if spread > 0.0001 {
                            let angle = gen_range(0.0f32, std::f32::consts::TAU);
                            let deviation = gen_range(0.0f32, spread);
                            fwd = (fwd
                                + right_vec * (angle.cos() * deviation)
                                + up_vec * (angle.sin() * deviation))
                                .normalize();
                        }

                        // bullet tracer
                        self.tracers.push(BulletTracer {
                            start: eye,
                            end: eye + fwd * range,
                            lifetime: if stats.pellet_count > 1 { 0.1 } else { 0.2 },
                        });

                        if let Some(zone) = self.target_dummy.ray_test(eye, fwd, range) {
                            let mult = zone.damage_mult();
                            let dmg = damage * mult;
                            self.target_dummy.take_damage(dmg);
                            total_dmg += dmg;
                            if zone == HitZone::Head {
                                any_headshot = true;
                            }
                            pellets_hit += 1;
                        }
                    }

                    // show hit marker + damage number for the combined total
                    if total_dmg > 0.0 {
                        self.hit_marker_timer = 0.35;
                        self.hit_marker_headshot = any_headshot;

                        // pellet shit
                        self.hit_marker_pellets_hit = pellets_hit;
                        self.hit_marker_pellets_total = stats.pellet_count;

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
            t.lifetime -= delta;
        }
        self.tracers.retain(|t| t.lifetime > 0.0);

        self.update_stamina(delta);
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
        maze: Option<&Maze>,
    ) {
        self.update_head_bob(get_frame_time());
        let camera = self.build_camera(config);

        clear_background(BLACK);
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
        self.draw_tracers();

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
            self.hit_marker_pellets_hit,
            self.hit_marker_pellets_total,
        );
    }

    // clear movement intent (for pause) just means that we stop applying movement input, but we still want to simulate physics so that the player doesn't get stuck in walls or fall through the floor when they unpause
    pub fn clear_input(&mut self) {
        self.move_dir = Vec3::ZERO;
        self.movement_state.is_sprinting = false;
        self.jump_requested = false;
    }

    // ── Private methods ──────────────────────────────────────────────

    fn update_stamina(&mut self, delta: f32) {
        if self.movement_state.is_sprinting {
            self.movement_state.stamina -= self.movement_config.stamina_drain * delta;
        } else {
            self.movement_state.stamina = (self.movement_state.stamina
                + self.movement_config.stamina_recover * delta)
                .min(100.0);
        }
    }

    fn update_crouch_height(&mut self, config: &GameConfig, delta: f32) {
        let target = if self.movement_state.is_crouching {
            config.crouch_height
        } else {
            config.standing_height
        };
        self.movement_state.height += (target - self.movement_state.height) * 10.0 * delta;
    }

    fn apply_jump(&mut self) {
        if self.jump_requested {
            self.movement_state.velocity.y = self.movement_config.jump_strength;
            self.movement_state.is_grounded = false;
            self.jump_requested = false;
        }
    }

    fn apply_gravity(&mut self, delta: f32) {
        self.movement_state.velocity.y -= self.movement_config.gravity * delta;
        self.position.y += self.movement_state.velocity.y * delta;
    }

    fn apply_movement(&mut self, delta: f32) {
        let speed = if self.movement_state.is_sprinting {
            self.movement_config.sprint_speed
        } else if self.movement_state.is_crouching {
            self.movement_config.crouch_speed
        } else {
            self.movement_config.walk_speed
        };

        let target_velocity = self.move_dir * speed;
        let rate = if !self.movement_state.is_grounded {
            2.0 // Low air control
        } else if self.move_dir != Vec3::ZERO {
            self.movement_config.acceleration
        } else {
            self.movement_config.friction
        };

        let saved_vy = self.movement_state.velocity.y;
        self.movement_state.velocity = self
            .movement_state
            .velocity
            .lerp(target_velocity, rate * delta);
        self.movement_state.velocity.y = saved_vy;

        self.position.x += self.movement_state.velocity.x * delta;
        self.position.z += self.movement_state.velocity.z * delta;
    }

    fn update_head_bob(&mut self, delta: f32) {
        if self.movement_state.is_sprinting && self.movement_state.is_grounded {
            self.bob_timer += delta * 12.0;
        } else {
            self.bob_timer = 0.0;
        }
    }

    fn build_camera(&self, config: &GameConfig) -> Camera3D {
        let bob_offset = if self.movement_state.is_sprinting && self.movement_state.is_grounded {
            self.bob_timer.sin() * 0.05
        } else {
            0.0
        };

        let forward = vec3(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        );

        let eye_pos = vec3(
            self.position.x,
            self.position.y + self.movement_state.height + bob_offset,
            self.position.z,
        );

        // ADS zoom: interpolate FOV based on ads_progress
        let weapon = self.weapons.active();
        let base_fov = config.raw_fov();
        let ads_fov = weapon.stats().ads_zoom_fov;
        let fov = base_fov + (ads_fov - base_fov) * weapon.ads_progress;

        Camera3D {
            position: eye_pos,
            target: eye_pos + forward,
            up: vec3(0.0, 1.0, 0.0),
            fovy: fov.to_radians(),
            aspect: None,
            z_near: 0.1,
            z_far: 100.0,
            projection: Projection::Perspective,
            render_target: None,
            viewport: None,
        }
    }

    fn draw_other_players(&self) {
        for p in &self.other_players {
            let color = if p.skin.is_some() { WHITE } else { p.color };
            draw_cube(vec3(p.x, p.y + 0.6, p.z), vec3(0.6, 1.2, 0.4), None, color);
            draw_cube(vec3(p.x, p.y + 1.4, p.z), vec3(0.4, 0.4, 0.4), None, color);
        }
    }
    /*
        Most unnecessary function ever but I wanted to have some fun while separating
        lobby and maze, maybe we can still use this in the future
        (at least x amount of players to be able to teleport and start game)

    */
    fn maze_teleporter_pos(&self, maze: Option<&Maze>) -> Vec3 {
        if let Some(m) = maze {
            m.teleporter_world()
        } else {
            vec3(1.5 * MAZE_CELL_SIZE, 0.0, 1.5 * MAZE_CELL_SIZE)
        }
    }

    fn draw_teleporter(&self, maze: Option<&Maze>) {
        let pos = match self.current_world {
            WorldId::Lobby => vec3(0.0, 0.0, -10.0),
            WorldId::Maze => self.maze_teleporter_pos(maze),
        };

        let t = get_time() as f32;
        let alpha = (t * 2.0).sin() * 0.3 + 0.7;
        let color = Color::new(0.2, 0.8, 1.0, alpha);

        draw_cube(pos + vec3(0.0, 1.5, 0.0), vec3(2.0, 3.0, 2.0), None, color);
        draw_cube_wires(pos + vec3(0.0, 1.5, 0.0), vec3(2.0, 3.0, 2.0), WHITE);
    }
    fn draw_teleporter_prompt(&self, maze: Option<&Maze>) {
        let (pos, label) = match self.current_world {
            WorldId::Lobby => (vec3(0.0, 0.0, -10.0), "ENTER MAZE"),
            WorldId::Maze => (self.maze_teleporter_pos(maze), "RETURN TO LOBBY"),
        };

        let dist = vec3(self.position.x - pos.x, 0.0, self.position.z - pos.z).length();
        if dist < 2.5 {
            let text = format!("[E] {}", label);
            let measure = measure_text(&text, None, 24, 1.0);
            draw_text(
                &text,
                screen_width() / 2.0 - measure.width / 2.0,
                screen_height() / 2.0 + 60.0,
                24.0,
                WHITE,
            );
        }
    }
    fn check_teleporter(&mut self, maze: Option<&Maze>) {
        let tp_pos = match self.current_world {
            WorldId::Lobby => vec3(0.0, 0.0, -10.0),
            WorldId::Maze => self.maze_teleporter_pos(maze),
        };

        let dist = vec3(self.position.x - tp_pos.x, 0.0, self.position.z - tp_pos.z).length();
        if dist < 2.0 && is_key_pressed(KeyCode::E) {
            self.pending_world_change = Some(match self.current_world {
                WorldId::Lobby => WorldId::Maze,
                WorldId::Maze => WorldId::Lobby,
            });
        }
    }

    fn draw_target_dummy(&self) {
        let d = &self.target_dummy;
        let hit_flash = d.hit_timer > 0.0;

        for hb in &d.hitboxes {
            let center = d.position + hb.center_offset;
            let size = hb.half_extents * 2.0;

            // Each zone gets a distinct color
            let base_color = match hb.zone {
                HitZone::Head => Color::from_rgba(255, 80, 80, 200), // red
                HitZone::Torso => Color::from_rgba(80, 180, 255, 200), // blue
                HitZone::Limbs => Color::from_rgba(80, 255, 80, 200), // green
            };

            let color = if hit_flash { WHITE } else { base_color };

            // Wire color matches zone but brighter
            let wire_color = match hb.zone {
                HitZone::Head => Color::from_rgba(255, 120, 120, 255),
                HitZone::Torso => Color::from_rgba(120, 200, 255, 255),
                HitZone::Limbs => Color::from_rgba(120, 255, 120, 255),
            };

            draw_cube(center, size, None, color);
            draw_cube_wires(center, size, wire_color);
        }
    }

    fn draw_target_dummy_hud(&self, camera: &Camera3D) {
        let d = &self.target_dummy;
        // project 3D position above head to screen space
        let world_pos = d.position + vec3(0.0, 2.0, 0.0);
        let rel = world_pos - camera.position;
        let forward = (camera.target - camera.position).normalize();
        let right = forward.cross(camera.up).normalize();
        let up = right.cross(forward).normalize();

        let dot = rel.dot(forward);
        if dot < 0.1 {
            return;
        } // behind camera

        let fov_rad = camera.fovy; // already in radians
        let fov_scale = screen_height() / (2.0 * (fov_rad / 2.0).tan());

        let sx = screen_width() / 2.0 + (rel.dot(right) / dot) * fov_scale;
        let sy = screen_height() / 2.0 - (rel.dot(up) / dot) * fov_scale;

        // health bar
        let bar_w = 60.0;
        let bar_h = 6.0;
        let hp_frac = d.hp / d.max_hp;

        draw_rectangle(
            sx - bar_w / 2.0,
            sy,
            bar_w,
            bar_h,
            Color::from_rgba(40, 40, 40, 200),
        );
        let hp_color = if hp_frac > 0.5 {
            Color::from_rgba(80, 200, 80, 255)
        } else if hp_frac > 0.25 {
            Color::from_rgba(220, 180, 40, 255)
        } else {
            RED
        };
        draw_rectangle(sx - bar_w / 2.0, sy, bar_w * hp_frac, bar_h, hp_color);
        draw_rectangle_lines(sx - bar_w / 2.0, sy, bar_w, bar_h, 1.0, WHITE);

        // label
        let label = "TARGET DUMMY";
        let measure = measure_text(label, None, 14, 1.0);
        draw_text(label, sx - measure.width / 2.0, sy - 4.0, 14.0, WHITE);
    }
    fn draw_tracers(&self) {
        for t in &self.tracers {
            let alpha = (t.lifetime / 0.2 * 255.0).min(255.0) as u8;
            let color = Color::from_rgba(255, 200, 80, alpha);
            draw_line_3d(t.start, t.end, color);
        }
    }
}

// ── Ray vs AABB intersection ─────────────────────────────────────────
// Returns true if a ray from `origin` in `dir` hits an axis-aligned box
// centered at `center` with half-extents `half`, within `max_dist`.
fn ray_hit_aabb(origin: Vec3, dir: Vec3, center: Vec3, half: Vec3, max_dist: f32) -> bool {
    let min = center - half;
    let max = center + half;

    let mut tmin = 0.0_f32;
    let mut tmax = max_dist;

    for i in 0..3 {
        let o = match i {
            0 => origin.x,
            1 => origin.y,
            _ => origin.z,
        };
        let d = match i {
            0 => dir.x,
            1 => dir.y,
            _ => dir.z,
        };
        let bmin = match i {
            0 => min.x,
            1 => min.y,
            _ => min.z,
        };
        let bmax = match i {
            0 => max.x,
            1 => max.y,
            _ => max.z,
        };

        if d.abs() < 1e-8 {
            // ray parallel to slab — miss if origin outside
            if o < bmin || o > bmax {
                return false;
            }
        } else {
            let inv_d = 1.0 / d;
            let mut t1 = (bmin - o) * inv_d;
            let mut t2 = (bmax - o) * inv_d;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            tmin = tmin.max(t1);
            tmax = tmax.min(t2);
            if tmin > tmax {
                return false;
            }
        }
    }
    true
}
