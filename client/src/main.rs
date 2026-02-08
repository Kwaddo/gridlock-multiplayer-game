use macroquad::prelude::*;
use std::net::UdpSocket;
use client::game_state::*;
use client::login::LoginScreen;
use client::protocols::*;
use client::movement::{ MovementState, MovementConfig };
use client::lobby::*;
use client::config::GameConfig;
use client::settings::SettingsScreen;

struct OtherPlayer {
    id: u32,
    x: f32,
    y: f32,
    z: f32,
    yaw: f32,
    username: String,
    skin: Option<Texture2D>,
    color: Color, // random color till skins
}

// helper function to see which button is pressed based on config keybinds, we will use this in the settings menu to show which button is bound to each action
fn is_button_clicked(x: f32, y: f32, w: f32, h: f32, label:  &str, color: Color, hover_color: Color) -> bool {
    let mouse = mouse_position();
    let hover = mouse.0 >= x && mouse.0 <= x + w && mouse.1 >= y && mouse.1 <= y + h;
    draw_rectangle(x, y, w, h, if hover { hover_color } else { color });
    draw_rectangle_lines(x, y, w, h, 1.0, WHITE);
    let tw = measure_text(label, None, 20, 1.0).width;
    draw_text(label, x + (w - tw) / 2.0, y + 26.0, 20.0, WHITE);
    hover && is_mouse_button_pressed(MouseButton::Left)
}

#[macroquad::main("Multiplayer FPS")]
async fn main() {
    let mut state = GameState::Login;
    let mut login_screen = LoginScreen::new();
    let mut settings_screen = SettingsScreen::new();

    let mut socket: Option<UdpSocket> = None;
    let mut player_id: u32 = 0;
    let mut username = String::new();

    let mut position = vec3(0.0, 1.0, 0.0);
    let mut yaw: f32 = 0.0;
    let mut pitch: f32 = 0.0;
    let mut last_mouse_pos = mouse_position();

    let mut other_players: Vec<OtherPlayer> = Vec::new();
    let mut server_addr = String::new();

    // movement implementation
    let mut movement_state = MovementState::new();
    let movement_config = MovementConfig::new();

    // game config
    let mut config = GameConfig::new();

    // Load textures once before the game loop
    let floor_texture = load_texture("assets/rocks.png").await.ok();

    loop {
        match state {
            GameState::Login => {
                if let Some((s, name, id)) = login_screen.update() {
                    socket = Some(s);
                    username = name;
                    player_id = id;
                    server_addr = login_screen.ip_input.clone();
                    state = GameState::Playing;
                    set_cursor_grab(true);
                    show_mouse(false);
                    last_mouse_pos = mouse_position();
                }
                login_screen.draw();
            }

            GameState::Playing => {
                let delta_time = get_frame_time();

                // Press ESC to open settings
                if is_key_pressed(config.key_settings) {
                    state = GameState::Paused;
                    set_cursor_grab(false);
                    show_mouse(true);
                    continue;
                }

                let mouse_pos = mouse_position();
                let mouse_delta = (mouse_pos.0 - last_mouse_pos.0, mouse_pos.1 - last_mouse_pos.1);
                last_mouse_pos = mouse_pos;

                yaw += mouse_delta.0 * config.raw_sensitivity();
                pitch -= mouse_delta.1 * config.raw_sensitivity();
                pitch = pitch.clamp(-std::f32::consts::PI / 2.0, std::f32::consts::PI / 2.0);

                let forward = vec3(yaw.cos() * pitch.cos(), pitch.sin(), yaw.sin() * pitch.cos());
                let right = vec3(-yaw.sin(), 0.0, yaw.cos());

                let mut move_dir = vec3(0.0, 0.0, 0.0);
                if is_key_down(config.key_forward) {
                    move_dir += forward;
                }
                if is_key_down(config.key_backward) {
                    move_dir -= forward;
                }
                if is_key_down(config.key_left) {
                    move_dir -= right;
                }
                if is_key_down(config.key_right) {
                    move_dir += right;
                }
                if move_dir != vec3(0.0, 0.0, 0.0) {
                    move_dir = move_dir.normalize();
                }

                // calculate velocity
                let target_velocity = move_dir * movement_config.walk_speed;

                // accelerate towards target or toward zero
                let rate = if move_dir != vec3(0.0, 0.0, 0.0) {
                    // if vec3 is not all zeroes, means u are not moving towards any of the axis, so we should accelerate towards target velocity
                    movement_config.acceleration
                } else {
                    movement_config.friction
                };
                movement_state.velocity = movement_state.velocity.lerp(
                    target_velocity,
                    rate * delta_time
                );

                position.x += movement_state.velocity.x * delta_time;
                // position.y += movement_state.velocity.y * delta_time; // we don't want to move vertically based on input, only based on gravity and jumping, so we comment this out for now. We will add vertical movement later when we implement jumping and gravity
                position.z += movement_state.velocity.z * delta_time;

                // Send position update to server
                if let Some(ref s) = socket {
                    let mut pos_msg = vec![MSG_POSITION];
                    pos_msg.extend_from_slice(&position.x.to_be_bytes());
                    pos_msg.extend_from_slice(&position.y.to_be_bytes());
                    pos_msg.extend_from_slice(&position.z.to_be_bytes());
                    pos_msg.extend_from_slice(&yaw.to_be_bytes());
                    let _ = s.send_to(&pos_msg, &server_addr);
                }

                // Receive updates from server
                if let Some(ref s) = socket {
                    let mut recv_buf: [u8; 512] = [0; 512];
                    while let Ok((size, _)) = s.recv_from(&mut recv_buf) {
                        if size > 0 && recv_buf[0] == MSG_STATE {
                            /*
                            Server state message format:
                            recv_buf[1..5] contains the player count as a 4 byte integer
                            Then for each player:
                            - 4 bytes for player id
                            - 4 bytes for x
                            - 4 bytes for y
                            - 4 bytes for z
                            - 4 bytes for yaw
                            - 1 byte for name length
                            that is why we start from offset 5 then add 4 + 4 + 4 + 4 + 4 + 1 = 21 bytes for each player, then add the name length to get to the next player
                            */
                            let player_count = u32::from_be_bytes([
                                recv_buf[1],
                                recv_buf[2],
                                recv_buf[3],
                                recv_buf[4],
                            ]) as usize;
                            let mut offset = 5;
                            let mut new_players = Vec::new();
                            for _ in 0..player_count {
                                let id = u32::from_be_bytes([
                                    recv_buf[offset],
                                    recv_buf[offset + 1],
                                    recv_buf[offset + 2],
                                    recv_buf[offset + 3],
                                ]);
                                offset += 4;
                                let x = f32::from_be_bytes([
                                    recv_buf[offset],
                                    recv_buf[offset + 1],
                                    recv_buf[offset + 2],
                                    recv_buf[offset + 3],
                                ]);
                                offset += 4;
                                let y = f32::from_be_bytes([
                                    recv_buf[offset],
                                    recv_buf[offset + 1],
                                    recv_buf[offset + 2],
                                    recv_buf[offset + 3],
                                ]);
                                offset += 4;
                                let z = f32::from_be_bytes([
                                    recv_buf[offset],
                                    recv_buf[offset + 1],
                                    recv_buf[offset + 2],
                                    recv_buf[offset + 3],
                                ]);
                                offset += 4;
                                let yaw = f32::from_be_bytes([
                                    recv_buf[offset],
                                    recv_buf[offset + 1],
                                    recv_buf[offset + 2],
                                    recv_buf[offset + 3],
                                ]);
                                offset += 4;
                                let name_len = recv_buf[offset] as usize;
                                offset += 1;
                                let name = String::from_utf8_lossy(
                                    &recv_buf[offset..offset + name_len]
                                ).to_string();
                                offset += name_len;

                                if id != player_id {
                                    let r = ((id * 73 + 29) % 256) as u8;
                                    let g = ((id * 97 + 53) % 256) as u8;
                                    let b = ((id * 61 + 19) % 256) as u8;
                                    let other_player = OtherPlayer {
                                        id,
                                        x,
                                        y,
                                        z,
                                        yaw,
                                        username: name,
                                        skin: None,
                                        color: Color::from_rgba(r, g, b, 255),
                                    };
                                    new_players.push(other_player);
                                }
                            }
                            other_players = new_players;
                        }
                    }
                }

                let camera = Camera3D {
                    position,
                    target: position + forward,
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

                draw_lobby(floor_texture.as_ref());

                // Draw other players
                for p in &other_players {
                    let player_color = if p.skin.is_some() {
                        WHITE // if we had skins, we'd draw the skin texture here instead of a cube, and we wouldn't need the color field. For now we just use a random color for each player
                    } else {
                        p.color
                    };
                    // Body (tall and narrow)
                    draw_cube(vec3(p.x, p.y + 0.6, p.z), vec3(0.6, 1.2, 0.4), None, player_color);
                    // Head (small cube on top)
                    draw_cube(vec3(p.x, p.y + 1.4, p.z), vec3(0.4, 0.4, 0.4), None, player_color);
                }

                set_default_camera();
                draw_text("Multiplayer FPS", 20.0, 20.0, 30.0, WHITE);
                draw_text(
                    &format!("Player: {} (ID: {})", username, player_id),
                    20.0,
                    50.0,
                    20.0,
                    WHITE
                );
                draw_text(
                    &format!("Pos: {:.2}, {:.2}, {:.2}", position.x, position.y, position.z),
                    20.0,
                    70.0,
                    20.0,
                    WHITE
                );

                // Draw nametags above other players
                for p in &other_players {
                    let head_pos = vec3(p.x, p.y + 1.8, p.z);
                    let rel = head_pos - position;
                    let dot = rel.dot(forward);
                    if dot > 0.1 {
                        // only draw if player is in front of us
                        let right_dot = rel.dot(right);
                        let up_dot = rel.y - position.y + pitch.sin() * dot;
                        let fov_scale =
                            screen_height() / (2.0 * ((45.0_f32).to_radians() / 2.0).tan());
                        let screen_x = screen_width() / 2.0 + (right_dot / dot) * fov_scale;
                        let screen_y = screen_height() / 2.0 - (up_dot / dot) * fov_scale;
                        let name_w = measure_text(&p.username, None, 16, 1.0).width;
                        draw_text(&p.username, screen_x - name_w / 2.0, screen_y, 16.0, p.color);
                    }
                }

                // Player list
                draw_text(
                    &format!("Players online: {}", other_players.len() + 1),
                    20.0,
                    100.0,
                    20.0,
                    YELLOW
                );
                for (i, p) in other_players.iter().enumerate() {
                    draw_text(
                        &format!("   {} (ID: {})", p.username, p.id),
                        20.0,
                        120.0 + (i as f32) * 20.0,
                        18.0,
                        p.color
                    );
                }
            }

            GameState::Paused => {
                // Press ESC to go back to playing
                // if is_key_pressed(config.key_settings) {
                //     state = GameState::Playing;
                //     set_cursor_grab(true);
                //     show_mouse(false);
                //     last_mouse_pos = mouse_position();
                // }

                clear_background(Color::from_rgba(0, 0, 0, 200));

                // Settings panel
                    let panel_w = 300.0;
                    let panel_h = 250.0;
                    let panel_x = (screen_width() - panel_w) / 2.0;
                    let panel_y = (screen_height() - panel_h) / 2.0;
                    draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::from_rgba(30, 30, 30, 240));
                    draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 2.0, WHITE);

                    // Title
                    let title = "PAUSED";
                    let title_w = measure_text(title, None, 30, 1.0).width;
                    draw_text(title, (screen_width() - title_w) / 2.0, panel_y + 40.0, 30.0, WHITE);

                    // Buttons
                    let btn_w = 200.0;
                    let btn_h = 40.0;
                    let btn_x = (screen_width() - btn_w) / 2.0;
                    let btn_color = Color::from_rgba(50,50,50,255);
                    let btn_hover = DARKGRAY;

                    if is_button_clicked(btn_x, panel_y + 80.0, btn_w, btn_h, "Resume", btn_color, btn_hover) {
                        state = GameState::Playing;
                        set_cursor_grab(true);
                        show_mouse(false);
                        last_mouse_pos = mouse_position();
                    }

                    if is_button_clicked(btn_x, panel_y + 130.0, btn_w, btn_h, "Settings", btn_color, btn_hover) {
                        state = GameState::Settings;
                    }

                    if is_button_clicked(btn_x, panel_y + 190.0, btn_w, btn_h, "Exit", Color::from_rgba(80, 20, 20, 255), MAROON) {
                        std::process::exit(0);
                    }
            }
            GameState::Settings => {
                clear_background(Color::from_rgba(0, 0, 0, 255));
                settings_screen.draw(&config);
                if settings_screen.update(&mut config) {
                    state = GameState::Paused;
                }
            }
        }

        next_frame().await;
    }
}
