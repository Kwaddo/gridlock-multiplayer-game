use macroquad::prelude::*;
use std::net::UdpSocket;
use client::game_state::*;
use client::login::LoginScreen;

#[macroquad::main("Multiplayer FPS")]
async fn main() {
    let mut state = GameState::Login;
    let mut login_screen = LoginScreen::new();

    let mut socket: Option<UdpSocket> = None;
    let mut player_id: u32 = 0;
    let mut username = String::new();

    let mut position = vec3(0.0, 1.0, 0.0);
    let mut yaw: f32 = 0.0;
    let mut pitch: f32 = 0.0;
    let mut last_mouse_pos = mouse_position();

    loop {
        match state {
            GameState::Login => {
                if let Some((s, name, id)) = login_screen.update() {
                    socket = Some(s);
                    username = name;
                    player_id = id;
                    state = GameState::Playing;
                    set_cursor_grab(true);
                    show_mouse(false);
                    last_mouse_pos = mouse_position();
                }
                login_screen.draw();
            }

            GameState::Playing => {
                let delta_time = get_frame_time();

                let mouse_pos = mouse_position();
                let mouse_delta = (
                    mouse_pos.0 - last_mouse_pos.0,
                    mouse_pos.1 - last_mouse_pos.1,
                );
                last_mouse_pos = mouse_pos;

                yaw += mouse_delta.0 * 0.002; // switched this because it was inverted (-= -> +=)
                pitch -= mouse_delta.1 * 0.002;
                pitch = pitch.clamp(-std::f32::consts::PI / 2.0, std::f32::consts::PI / 2.0);

                let forward = vec3(
                    yaw.cos() * pitch.cos(),
                    pitch.sin(),
                    yaw.sin() * pitch.cos(),
                );
                let right = vec3(-yaw.sin(), 0.0, yaw.cos());

                let mut move_dir = vec3(0.0, 0.0, 0.0);
                if is_key_down(KeyCode::W) { move_dir += forward; }
                if is_key_down(KeyCode::S) { move_dir -= forward; }
                if is_key_down(KeyCode::A) { move_dir -= right; }
                if is_key_down(KeyCode::D) { move_dir += right; }
                if move_dir != vec3(0.0, 0.0, 0.0) {
                    move_dir = move_dir.normalize();
                    position += move_dir * 5.0 * delta_time;
                }

                let camera = Camera3D {
                    position,
                    target: position + forward,
                    up: vec3(0.0, 1.0, 0.0),
                    fovy: 45.0,
                    aspect: None,
                    z_near: 0.1,
                    z_far: 100.0,
                    projection: Projection::Perspective,
                    render_target: None,
                    viewport: None,
                };

                clear_background(BLACK);
                set_camera(&camera);

                draw_cube(vec3(0.0, -1.0, 0.0), vec3(20.0, 0.1, 20.0), None, GRAY);
                draw_cube(vec3(0.0, 0.0, -5.0), vec3(1.0, 1.0, 1.0), None, RED);
                draw_cube(vec3(3.0, 0.0, -5.0), vec3(1.0, 1.0, 1.0), None, GREEN);

                set_default_camera();
                draw_text("Multiplayer FPS", 20.0, 20.0, 30.0, WHITE);
                draw_text(
                    &format!("Player: {} (ID: {})", username, player_id),
                    20.0, 50.0, 20.0, WHITE,
                );
                draw_text(
                    &format!("Pos: {:.2}, {:.2}, {:.2}", position.x, position.y, position.z),
                    20.0, 70.0, 20.0, WHITE,
                );
            }
        }

        next_frame().await
    }
}
