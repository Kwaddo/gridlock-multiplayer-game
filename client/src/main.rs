use macroquad::prelude::*;
use std::net::UdpSocket;

use crate::game_state::GameState;
use crate::login::LoginScreen;

mod game_state;
mod login;
mod net;
mod network;
mod types;
mod world;

use net::{poll_updates, protocols::*, send_position};
use types::maze::Maze;
use types::orientation::Orientation;
use types::player::OtherPlayer;
use world::{draw_maze, draw_nametags, draw_players, draw_ui, make_camera};

const MOVE_SPEED: f32 = 5.0;
const PLAYER_RADIUS: f32 = 0.35;
const GROUND_Y: f32 = 0.0;
const EYE_HEIGHT: f32 = 1.6;
const MOUSE_SENSITIVITY: f32 = 0.002;

struct App {
    state: GameState,
    login_screen: LoginScreen,
    socket: Option<UdpSocket>,
    player_id: u32,
    username: String,
    position: Vec3,
    orientation: Orientation,
    other_players: Vec<OtherPlayer>,
    server_addr: String,
    maze: Option<Maze>,
}

impl App {
    fn new() -> Self {
        Self {
            state: GameState::Login,
            login_screen: LoginScreen::new(),
            socket: None,
            player_id: 0,
            username: String::new(),
            position: vec3(0.0, GROUND_Y, 0.0),
            orientation: Orientation::new(),
            other_players: Vec::new(),
            server_addr: String::new(),
            maze: None,
        }
    }

    fn handle_login(&mut self) {
        if let Some((socket, name, id)) = self.login_screen.update() {
            self.socket = Some(socket);
            self.username = name;
            self.player_id = id;
            self.server_addr = self.login_screen.ip_input.clone();
            self.state = GameState::Playing;
            set_cursor_grab(true);
            show_mouse(false);
            self.orientation.last_mouse_pos = mouse_position();
        }
        self.login_screen.draw();
    }

    fn handle_play(&mut self, delta_time: f32) {
        if self.handle_system_input() {
            return;
        }

        self.orientation.update(MOUSE_SENSITIVITY);
        self.apply_movement(delta_time);
        self.send_position();
        self.receive_updates();
        self.render_scene();
    }

    fn handle_system_input(&mut self) -> bool {
        if is_key_pressed(KeyCode::Escape) {
            if let Some(ref socket) = self.socket {
                let _ = socket.send_to(&[MSG_DISCONNECT], &self.server_addr);
            }
            std::process::exit(0);
        }

        if is_key_pressed(KeyCode::Q) {
            set_cursor_grab(false);
            show_mouse(true);
        }

        if is_mouse_button_pressed(MouseButton::Left) {
            set_cursor_grab(true);
            show_mouse(false);
            self.orientation.last_mouse_pos = mouse_position();
        }

        false
    }

    fn apply_movement(&mut self, delta_time: f32) {
        let mut move_dir = vec3(0.0, 0.0, 0.0);
        if is_key_down(KeyCode::W) {
            move_dir += self.orientation.forward_flat;
        }
        if is_key_down(KeyCode::S) {
            move_dir -= self.orientation.forward_flat;
        }
        if is_key_down(KeyCode::A) {
            move_dir -= self.orientation.right;
        }
        if is_key_down(KeyCode::D) {
            move_dir += self.orientation.right;
        }

        if move_dir != vec3(0.0, 0.0, 0.0) {
            move_dir = move_dir.normalize();
            let mut desired = self.position + move_dir * MOVE_SPEED * delta_time;
            if let Some(ref maze) = self.maze {
                desired = maze.clamp_move(self.position, desired, PLAYER_RADIUS);
            }
            desired.y = GROUND_Y;
            self.position = desired;
        }
    }

    fn send_position(&self) {
        if let Some(ref socket) = self.socket {
            send_position(
                socket,
                &self.server_addr,
                self.position,
                self.orientation.yaw,
            );
        }
    }

    fn receive_updates(&mut self) {
        if let Some(ref socket) = self.socket {
            let updates = poll_updates(socket, self.player_id);
            if let Some((maze, spawn)) = updates.maze {
                self.maze = Some(maze);
                self.position = spawn;
            }
            if let Some(players) = updates.players {
                self.other_players = players;
            }
        }
    }

    fn render_scene(&self) {
        let camera_pos = vec3(
            self.position.x,
            self.position.y + EYE_HEIGHT,
            self.position.z,
        );
        let camera = make_camera(camera_pos, self.orientation.forward);

        clear_background(BLACK);
        set_camera(&camera);

        draw_maze(self.maze.as_ref());
        draw_players(&self.other_players);

        set_default_camera();
        draw_nametags(
            &self.other_players,
            self.position,
            self.orientation.forward,
            self.orientation.right,
            self.orientation.pitch,
        );
        draw_ui(
            &self.username,
            self.player_id,
            self.position,
            &self.other_players,
        );
    }
}

#[macroquad::main("Multiplayer FPS")]
async fn main() {
    let mut app = App::new();

    loop {
        match app.state {
            GameState::Login => app.handle_login(),
            GameState::Playing => app.handle_play(get_frame_time()),
        }

        next_frame().await;
    }
}
