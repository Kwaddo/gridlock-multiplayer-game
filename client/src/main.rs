use macroquad::prelude::*;
use std::net::UdpSocket;

use client::core::{GameConfig, GameState};
use client::net::poll_updates;
use client::screens::{draw_pause_menu, LoginScreen, PlayingState};
use client::settings::SettingsScreen;
use client::types::maze::Maze;
use client::ui::fonts;

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
    let mut maze: Option<Maze> = None;

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
                sync_world(&socket, player_id, &mut playing_state, &mut maze);

                if let Some(new_state) = playing_state.process_local_input(&config) {
                    state = new_state;
                    if matches!(state, GameState::Paused) {
                        set_cursor_grab(false);
                        show_mouse(true);
                        playing_state.clear_input();
                    }
                    next_frame().await;
                    continue;
                }

                playing_state.simulate(&config, maze.as_ref());
                playing_state.send_network(&socket, &server_addr);
                playing_state.render(&config, &username, player_id, maze.as_ref());
            }

            GameState::Paused => {
                // Keep world up to date even while paused.
                sync_world(&socket, player_id, &mut playing_state, &mut maze);

                playing_state.simulate(&config, maze.as_ref());
                playing_state.render(&config, &username, player_id, maze.as_ref());

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

            GameState::Exit => {
                // Send disconnect message to server
                if let Some(ref s) = socket {
                    let disconnect_msg = vec![client::net::protocols::MSG_DISCONNECT];
                    let _ = s.send_to(&disconnect_msg, &server_addr);
                }
                std::process::exit(0);
            }
        }

        next_frame().await;
    }
}

fn sync_world(
    socket: &Option<UdpSocket>,
    player_id: u32,
    playing_state: &mut PlayingState,
    maze: &mut Option<Maze>,
) {
    if let Some(ref sock) = socket {
        let updates = poll_updates(sock, player_id);
        if let Some((new_maze, spawn)) = updates.maze {
            *maze = Some(new_maze);
            playing_state.position = spawn;
        }
        if let Some(players) = updates.players {
            playing_state.other_players = players;
        }
    }
}
