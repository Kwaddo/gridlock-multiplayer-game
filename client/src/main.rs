use macroquad::prelude::*;
use std::net::UdpSocket;
use client::game_state::*;
use client::login::LoginScreen;
use client::config::GameConfig;
use client::settings::SettingsScreen;
use client::playing::PlayingState;
use client::pause_menu::draw_pause_menu;
use client::fonts;

#[macroquad::main("Multiplayer FPS")]
async fn main() {
    fonts::init_font().await;

    let mut state = GameState::Login;
    let mut login_screen = LoginScreen::new();
    let mut settings_screen = SettingsScreen::new();
    let mut playing_state = PlayingState::new().await;

    let mut socket: Option<UdpSocket> = None;
    let mut player_id: u32 = 0;
    let mut username = String::new();
    let mut server_addr = String::new();

    let mut config = GameConfig::new();

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
                    playing_state.last_mouse_pos = mouse_position();
                }
                login_screen.draw();
            }

            GameState::Playing => {
                if let Some(new_state) = playing_state.process_local_input(&config) { // phase 1: input processing
                    state = new_state;
                    if matches!(state, GameState::Paused) {
                        set_cursor_grab(false);
                        show_mouse(true);
                        playing_state.clear_input();
                    }
                    continue;
                }
                playing_state.receive_network(&socket, player_id); // phase 2: network receive
                playing_state.simulate(&config); // phase 3: simulation
                playing_state.send_network(&socket, &server_addr); // phase 4: network send
                playing_state.render(&config, &username, player_id); // phase 5: rendering
            }

            GameState::Paused => {
                playing_state.receive_network(&socket, player_id); // when we pause we still run phase 2 to receive updates from the server, this allows us to have an up to date view of the game world when we unpause, otherwise we might unpause and immediately get teleported to a different position because the server has already processed several movement updates while we were paused. We also run phase 3 to update our local movement state based on the latest input and physics calculations, this allows us to have a smooth transition when we unpause instead of suddenly snapping to a different position or velocity.
                playing_state.simulate(&config); // phase 3: simulation
                playing_state.render(&config, &username, player_id); // phase 5: rendering

                if let Some(new_state) = draw_pause_menu(&config, &mut settings_screen) {
                    state = new_state;
                    if matches!(state, GameState::Playing) {
                        set_cursor_grab(true);
                        show_mouse(false);
                        playing_state.last_mouse_pos = mouse_position();
                    }
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
