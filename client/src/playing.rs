use macroquad::prelude::*;
use std::net::UdpSocket;
use crate::config::GameConfig;
use crate::game_state::GameState;
use crate::hud::draw_hud;
use crate::lobby::{draw_lobby, LANDMARK_CUBES, PARKOUR_CUBES};
use crate::movement::{MovementState, MovementConfig};
use crate::protocols::*;
use crate::ui::OtherPlayer;

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

    /*
        Below are the main phases of the game loop when in the Playing state. Each phase is separated into its own method for clarity, and they are called in order from the main loop in main.rs.
        Phase 1 handles local input and updates movement intent. 
        Phase 2 receives and processes network packets from the server. 
        Phase 3 runs physics and updates player position based on movement intent and collisions. 
        Phase 4 sends the updated position to the server. 
        Phase 5 renders the world and HUD.
        The reason for this separation is to ensure that even when the game is paused, we can still receive network updates and render the world, while ignoring local input and halting movement simulation. 
        This allows for a responsive pause menu without freezing the entire game state.
        This also makes it easy to add additional features later, such as spectator mode, where you can fly around and look at the world without affecting your player position or sending updates to the server.
        This also will account for when we add deaths and respawns etc..
     */

    // ── Phase 1: Local Input ─────────────────────────────────────────
    // Reads keyboard/mouse and produces movement intent.
    // Only called when actively playing (skipped when paused).
    pub fn process_local_input(&mut self, config: &GameConfig) -> Option<GameState> {
        if is_key_pressed(config.key_settings) {
            return Some(GameState::Paused);
        }

        let mouse_pos = mouse_position();
        let mouse_delta = (
            mouse_pos.0 - self.last_mouse_pos.0,
            mouse_pos.1 - self.last_mouse_pos.1,
        );
        self.last_mouse_pos = mouse_pos;

        self.yaw += mouse_delta.0 * config.raw_sensitivity();
        self.pitch -= mouse_delta.1 * config.raw_sensitivity();
        self.pitch = self.pitch.clamp(-1.55, 1.55);

        let right = vec3(-self.yaw.sin(), 0.0, self.yaw.cos());
        let move_forward = vec3(self.yaw.cos(), 0.0, self.yaw.sin());

        self.move_dir = Vec3::ZERO;
        if is_key_down(config.key_forward) { self.move_dir += move_forward; }
        if is_key_down(config.key_backward) { self.move_dir -= move_forward; }
        if is_key_down(config.key_left) { self.move_dir -= right; }
        if is_key_down(config.key_right) { self.move_dir += right; }
        if self.move_dir != Vec3::ZERO {
            self.move_dir = self.move_dir.normalize();
        }

        // sprinting (only on the ground)
        self.movement_state.is_sprinting =
            is_key_down(config.key_sprint)
            && self.movement_state.is_grounded
            && self.move_dir != Vec3::ZERO
            && self.movement_state.stamina > 0.0;

        // crouching
        if config.crouch_toggle {
            if is_key_pressed(config.key_crouch) {
                self.movement_state.is_crouching = !self.movement_state.is_crouching;
            }
        } else {
            self.movement_state.is_crouching = is_key_down(config.key_crouch);
        }

        /* jump request, 
        its a request because we only set a flag here and the actual jump impulse is applied in the simulate() method, 
        which allows us to have consistent jump behavior even if the player presses jump slightly before landing. 
        */
        self.jump_requested = is_key_down(config.key_jump) && self.movement_state.is_grounded;

        None
    }

    // ── Phase 2: Network Receive ─────────────────────────────────────
    // Drains all pending server packets and updates world state.
    // Called in both playing and paused states.
    pub fn receive_network(&mut self, socket: &Option<UdpSocket>, player_id: u32) {
        if let Some(ref s) = socket {
            let mut recv_buf: [u8; 512] = [0; 512];
            while let Ok((size, _)) = s.recv_from(&mut recv_buf) {
                if size > 0 && recv_buf[0] == MSG_STATE {
                    let player_count = u32::from_be_bytes([
                        recv_buf[1], recv_buf[2], recv_buf[3], recv_buf[4],
                    ]) as usize;
                    let mut offset = 5;
                    let mut new_players = Vec::new();
                    for _ in 0..player_count {
                        let id = u32::from_be_bytes([
                            recv_buf[offset], recv_buf[offset + 1],
                            recv_buf[offset + 2], recv_buf[offset + 3],
                        ]);
                        offset += 4;
                        let x = f32::from_be_bytes([
                            recv_buf[offset], recv_buf[offset + 1],
                            recv_buf[offset + 2], recv_buf[offset + 3],
                        ]);
                        offset += 4;
                        let y = f32::from_be_bytes([
                            recv_buf[offset], recv_buf[offset + 1],
                            recv_buf[offset + 2], recv_buf[offset + 3],
                        ]);
                        offset += 4;
                        let z = f32::from_be_bytes([
                            recv_buf[offset], recv_buf[offset + 1],
                            recv_buf[offset + 2], recv_buf[offset + 3],
                        ]);
                        offset += 4;
                        let remote_yaw = f32::from_be_bytes([
                            recv_buf[offset], recv_buf[offset + 1],
                            recv_buf[offset + 2], recv_buf[offset + 3],
                        ]);
                        offset += 4;
                        let name_len = recv_buf[offset] as usize;
                        offset += 1;
                        let name = String::from_utf8_lossy(
                            &recv_buf[offset..offset + name_len],
                        ).to_string();
                        offset += name_len;

                        if id != player_id {
                            let r = ((id * 73 + 29) % 256) as u8;
                            let g = ((id * 97 + 53) % 256) as u8;
                            let b = ((id * 61 + 19) % 256) as u8;
                            new_players.push(OtherPlayer {
                                id, x, y, z,
                                yaw: remote_yaw,
                                username: name,
                                skin: None,
                                color: Color::from_rgba(r, g, b, 255),
                            });
                        }
                    }
                    self.other_players = new_players;
                }
            }
        }
    }

    // ── Phase 3: Simulate ────────────────────────────────────────────
    // Runs physics, stamina, collisions. Uses whatever move_dir / sprint /
    // crouch / jump_requested was set by input (or left zeroed when paused).
    pub fn simulate(&mut self, config: &GameConfig) {
        let delta_time = get_frame_time();

        // Stamina
        if self.movement_state.is_sprinting {
            self.movement_state.stamina -= self.movement_config.stamina_drain * delta_time;
        } else {
            self.movement_state.stamina = (
                self.movement_state.stamina + self.movement_config.stamina_recover * delta_time
            ).min(100.0);
        }

        // Crouch height lerp
        let target_height = if self.movement_state.is_crouching {
            config.crouch_height
        } else {
            config.standing_height
        };
        self.movement_state.height =
            self.movement_state.height
            + (target_height - self.movement_state.height) * 10.0 * delta_time;

        // Jump
        if self.jump_requested {
            self.movement_state.velocity.y = self.movement_config.jump_strength;
            self.movement_state.is_grounded = false;
            self.jump_requested = false;
        }

        // Gravity
        self.movement_state.velocity.y -= self.movement_config.gravity * delta_time;
        self.position.y += self.movement_state.velocity.y * delta_time;

        // Vertical collision (ground + cube tops + head bump)
        self.resolve_vertical_collision();

        // Horizontal movement
        let speed = if self.movement_state.is_sprinting {
            self.movement_config.sprint_speed
        } else if self.movement_state.is_crouching {
            self.movement_config.crouch_speed
        } else {
            self.movement_config.walk_speed
        };
        let target_velocity = self.move_dir * speed;

        // Air vs ground control: much less acceleration/friction in the air
        let rate = if !self.movement_state.is_grounded {
            2.0 // very low air control
        } else if self.move_dir != Vec3::ZERO {
            self.movement_config.acceleration
        } else {
            self.movement_config.friction
        };

        // Preserve vertical velocity when lerping horizontal
        let saved_vy = self.movement_state.velocity.y;
        self.movement_state.velocity = self.movement_state.velocity.lerp(
            target_velocity,
            rate * delta_time,
        );
        self.movement_state.velocity.y = saved_vy;

        self.position.x += self.movement_state.velocity.x * delta_time;
        self.position.z += self.movement_state.velocity.z * delta_time;

        // Horizontal collision (side walls of cubes)
        self.resolve_horizontal_collision();
    }

    // ── Phase 4: Network Send ────────────────────────────────────────
    // Sends local player position to the server.
    // Only called when actively playing (skipped when paused).
    pub fn send_network(&self, socket: &Option<UdpSocket>, server_addr: &str) {
        if let Some(ref s) = socket {
            let mut pos_msg = vec![MSG_POSITION];
            pos_msg.extend_from_slice(&self.position.x.to_be_bytes());
            pos_msg.extend_from_slice(&self.position.y.to_be_bytes());
            pos_msg.extend_from_slice(&self.position.z.to_be_bytes());
            pos_msg.extend_from_slice(&self.yaw.to_be_bytes());
            let _ = s.send_to(&pos_msg, server_addr);
        }
    }

    // ── Phase 5: Render ──────────────────────────────────────────────
    // Draws the 3D world, other players, and HUD.
    // Called in both playing and paused states.
    pub fn render(&mut self, config: &GameConfig, username: &str, player_id: u32) {
        let delta_time = get_frame_time();

        // Head bobbing when sprinting on the ground
        if self.movement_state.is_sprinting && self.movement_state.is_grounded {
            self.bob_timer += delta_time * 12.0;
        } else {
            self.bob_timer = 0.0;
        }
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
        let right = vec3(-self.yaw.sin(), 0.0, self.yaw.cos());

        // Camera — eye position = foot position + eye height + bob
        let eye_pos = vec3(
            self.position.x,
            self.position.y + self.movement_state.height + bob_offset,
            self.position.z,
        );

        let camera = Camera3D {
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
        };

        clear_background(BLACK);
        set_camera(&camera);

        draw_lobby(self.floor_texture.as_ref());

        // Draw other players
        for p in &self.other_players {
            let player_color = if p.skin.is_some() { WHITE } else { p.color };
            draw_cube(vec3(p.x, p.y + 0.6, p.z), vec3(0.6, 1.2, 0.4), None, player_color);
            draw_cube(vec3(p.x, p.y + 1.4, p.z), vec3(0.4, 0.4, 0.4), None, player_color);
        }

        set_default_camera();

        // HUD
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

    /// Clears movement intent so simulation runs passively (friction/gravity only).
    /// Called when transitioning to paused state.
    pub fn clear_input(&mut self) {
        self.move_dir = Vec3::ZERO;
        self.movement_state.is_sprinting = false;
        self.jump_requested = false;
    }

    // iterator over all collidable cubes (landmarks + parkour)
    fn all_cubes() -> impl Iterator<Item = &'static (Vec3, Vec3)> {
        LANDMARK_CUBES.iter().chain(PARKOUR_CUBES.iter())
    }

    // resolve vertical collision: land on cube tops, bump head on cube bottoms, land on ground.
    fn resolve_vertical_collision(&mut self) {
        let px = self.position.x;
        let py = self.position.y; // foot Y
        let pz = self.position.z;
        let player_radius = 0.25;
        let player_height = self.movement_state.height; // body height above feet
        let head_y = py + player_height;

        // start with flat ground
        let mut best_ground = 0.0_f32;

        for (center, size) in Self::all_cubes() {
            let half = *size * 0.5;
            let cube_min_x = center.x - half.x;
            let cube_max_x = center.x + half.x;
            let cube_min_y = center.y - half.y;
            let cube_max_y = center.y + half.y;
            let cube_min_z = center.z - half.z;
            let cube_max_z = center.z + half.z;

            // check XZ overlap (player circle vs cube AABB)
            let closest_x = px.clamp(cube_min_x, cube_max_x);
            let closest_z = pz.clamp(cube_min_z, cube_max_z);
            let dx = px - closest_x;
            let dz = pz - closest_z;
            if dx * dx + dz * dz > player_radius * player_radius {
                continue; // no horizontal overlap
            }

            // landing on top: feet at or just above the cube top, and we were falling
            if py >= cube_max_y - 0.1 && self.movement_state.velocity.y <= 0.0 {
                if cube_max_y > best_ground {
                    best_ground = cube_max_y;
                }
            }

            // head bump: head penetrates cube bottom while rising
            if self.movement_state.velocity.y > 0.0
                && head_y >= cube_min_y && py < cube_min_y
            {
                self.position.y = cube_min_y - player_height;
                self.movement_state.velocity.y = 0.0;
            }
        }

        // apply ground landing
        if self.position.y <= best_ground {
            self.position.y = best_ground;
            self.movement_state.is_grounded = true;
            self.movement_state.velocity.y = 0.0;
        }
    }

    // resolve horizontal collision: push player out of cube side walls.
    fn resolve_horizontal_collision(&mut self) {
        let player_radius = 0.25;
        let py = self.position.y;
        let player_height = self.movement_state.height;
        let head_y = py + player_height;

        for (center, size) in Self::all_cubes() {
            let half = *size * 0.5;
            let cube_min_x = center.x - half.x;
            let cube_max_x = center.x + half.x;
            let cube_min_y = center.y - half.y;
            let cube_max_y = center.y + half.y;
            let cube_min_z = center.z - half.z;
            let cube_max_z = center.z + half.z;

            // check vertical overlap
            // only collide with sides if there's vertical overlap AND player isn't on top
            if head_y <= cube_min_y || py >= cube_max_y {
                continue; // no vertical overlap, skip
            }

            // find closest point on cube XZ to player
            let closest_x = self.position.x.clamp(cube_min_x, cube_max_x);
            let closest_z = self.position.z.clamp(cube_min_z, cube_max_z);
            let dx = self.position.x - closest_x;
            let dz = self.position.z - closest_z;
            let dist_sq = dx * dx + dz * dz;

            if dist_sq < player_radius * player_radius && dist_sq > 0.0001 {
                // push player out
                let dist = dist_sq.sqrt();
                let overlap = player_radius - dist;
                let nx = dx / dist;
                let nz = dz / dist;
                self.position.x += nx * overlap;
                self.position.z += nz * overlap;

                // kill velocity into the wall
                let dot = self.movement_state.velocity.x * nx + self.movement_state.velocity.z * nz;
                if dot < 0.0 {
                    self.movement_state.velocity.x -= dot * nx;
                    self.movement_state.velocity.z -= dot * nz;
                }
            } else if dist_sq < 0.0001 {
                // player center is inside the cube — push out on shortest axis, just in case this somehow happens
                let push_left = self.position.x - cube_min_x;
                let push_right = cube_max_x - self.position.x;
                let push_back = self.position.z - cube_min_z;
                let push_front = cube_max_z - self.position.z;
                let min_push = push_left.min(push_right).min(push_back).min(push_front);
                if min_push == push_left {
                    self.position.x = cube_min_x - player_radius;
                    self.movement_state.velocity.x = self.movement_state.velocity.x.min(0.0);
                } else if min_push == push_right {
                    self.position.x = cube_max_x + player_radius;
                    self.movement_state.velocity.x = self.movement_state.velocity.x.max(0.0);
                } else if min_push == push_back {
                    self.position.z = cube_min_z - player_radius;
                    self.movement_state.velocity.z = self.movement_state.velocity.z.min(0.0);
                } else {
                    self.position.z = cube_max_z + player_radius;
                    self.movement_state.velocity.z = self.movement_state.velocity.z.max(0.0);
                }
            }
        }
    }
}
