use crate::core::{GameConfig, GameState};
use crate::physics::{collision, movement::{MovementConfig, MovementState}};
use crate::protocols::*;
use crate::types::maze::Maze;
use crate::types::player::OtherPlayer;
use crate::ui::hud::draw_hud;
use crate::world::{draw_maze, lobby::draw_lobby};
use macroquad::prelude::*;
use std::net::UdpSocket;

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
    pub floor_texture: Option<Texture2D>,
    pub move_dir: Vec3,
    pub jump_requested: bool,
}

impl PlayingState {
    pub async fn new() -> Self {
        let floor_texture = load_texture("assets/rocks.png").await.ok();
        Self {
            position: vec3(0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            last_mouse_pos: (0.0, 0.0),
            other_players: Vec::new(),
            movement_state: MovementState::new(),
            movement_config: MovementConfig::new(),
            bob_timer: 0.0,
            floor_texture,
            move_dir: Vec3::ZERO,
            jump_requested: false,
        }
    }

    /// Process local input and return state transition if any
    pub fn process_local_input(&mut self, config: &GameConfig) -> Option<GameState> {
        if is_key_pressed(config.key_settings) {
            return Some(GameState::Paused);
        }

        input::process_mouse(
            &mut self.last_mouse_pos,
            &mut self.yaw,
            &mut self.pitch,
            config.raw_sensitivity(),
        );

        self.move_dir = input::process_movement_keys(self.yaw, config);
        self.movement_state.is_sprinting =
            input::process_sprint(config, &self.movement_state, self.move_dir);
        self.movement_state.is_crouching =
            input::process_crouch(config, self.movement_state.is_crouching);
        self.jump_requested = input::process_jump(config, self.movement_state.is_grounded);

        None
    }

    /// Run physics simulation
    pub fn simulate(&mut self, config: &GameConfig, maze: Option<&Maze>) {
        let delta = get_frame_time();

        self.update_stamina(delta);
        self.update_crouch_height(config, delta);
        self.apply_jump();
        self.apply_gravity(delta);

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

        if let Some(m) = maze {
            collision::resolve_maze(&mut self.position, &mut self.movement_state.velocity, m);
        }
    }

    /// Send position update to server
    pub fn send_network(&self, socket: &Option<UdpSocket>, server_addr: &str) {
        if let Some(ref s) = socket {
            let mut msg = vec![MSG_POSITION];
            msg.extend_from_slice(&self.position.x.to_be_bytes());
            msg.extend_from_slice(&self.position.y.to_be_bytes());
            msg.extend_from_slice(&self.position.z.to_be_bytes());
            msg.extend_from_slice(&self.yaw.to_be_bytes());
            let _ = s.send_to(&msg, server_addr);
        }
    }

    /// Render 3D world and HUD
    pub fn render(&mut self, config: &GameConfig, username: &str, player_id: u32, maze: Option<&Maze>) {
        self.update_head_bob(get_frame_time());
        let camera = self.build_camera(config);

        clear_background(BLACK);
        set_camera(&camera);

        if let Some(m) = maze {
            draw_maze(Some(m));
        }
        draw_lobby(self.floor_texture.as_ref());
        self.draw_other_players();

        set_default_camera();

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
        );
    }

    /// Clear movement intent (for pause)
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
            self.movement_state.stamina =
                (self.movement_state.stamina + self.movement_config.stamina_recover * delta)
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
        self.movement_state.velocity = self.movement_state.velocity.lerp(target_velocity, rate * delta);
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

        Camera3D {
            position: eye_pos,
            target: eye_pos + forward,
            up: vec3(0.0, 1.0, 0.0),
            fovy: config.raw_fov(),
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
}
