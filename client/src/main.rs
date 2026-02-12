use macroquad::prelude::*;
use std::net::UdpSocket;

use client::core::{ GameConfig, GameState };
use client::net::poll_updates;
use client::screens::{ draw_pause_menu, LoginScreen, PlayingState };
use client::screens::dev_mode::{ draw_dev_overlay, DevModeState };
use client::settings::SettingsScreen;
use client::types::maze::Maze;
use client::ui::fonts;

fn window_conf() -> Conf {
    Conf {
        window_title: "Multiplayer FPS".to_owned(),
        fullscreen: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // Set Nearest filtering globally so all textures (sprites, crosshairs) stay crisp
    set_default_filter_mode(FilterMode::Nearest);

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
    let mut is_fullscreen = true;

    // loading state tracking
    let mut loading_assets: Vec<(String, String)> = Vec::new();
    let mut loading_index: usize = 0;
    let mut loading_current: String = String::new();
    let mut loading_stable_frames: u8 = 0;
    let mut loading_started: bool = false;

    loop {
        if is_key_pressed(KeyCode::F11) {
            is_fullscreen = !is_fullscreen;
            set_fullscreen(is_fullscreen);
        }

        match state {
            GameState::Login => {
                if let Some((s, name, id)) = login_screen.update() {
                    socket = Some(s);
                    username = name;
                    player_id = id;
                    server_addr = login_screen.ip_input.clone();

                    // reset loading state and transition to Loading
                    loading_assets.clear();
                    loading_index = 0;
                    loading_current.clear();
                    loading_stable_frames = 0;
                    loading_started = false;
                    state = GameState::Loading;
                }
                login_screen.draw();
            }

            GameState::Loading => {
                login_screen.update_background();

                // poll for network data (maze, assets)
                let assets = sync_world(&socket, player_id, &mut playing_state, &mut maze);
                if let Some(entries) = assets {
                    if !entries.is_empty() {
                        loading_assets.extend(entries);
                        loading_stable_frames = 0;
                    }
                }

                if !loading_started {
                    loading_stable_frames += 1;
                    // start once we've collected asset packets and they've stabilised
                    if !loading_assets.is_empty() && loading_stable_frames >= 3 {
                        loading_started = true;
                    }
                    // timeout: server might have no assets at all
                    if loading_stable_frames >= 30 {
                        loading_started = true;
                    }
                }

                // load one texture per frame to keep the progress bar updating
                if loading_started && loading_index < loading_assets.len() {
                    let (key, path) = loading_assets[loading_index].clone();
                    loading_current = key.clone();
                    match load_texture(&path).await {
                        Ok(tex) => {
                            tex.set_filter(FilterMode::Nearest);
                            playing_state.assets.insert(key, tex);
                        }
                        Err(e) => {
                            println!("Failed to load {}: {:?}", path, e);
                        }
                    }
                    loading_index += 1;
                }

                // transition to Playing when done
                if loading_started && loading_index >= loading_assets.len() {
                    state = GameState::Playing;
                    set_cursor_grab(true);
                    show_mouse(false);
                    playing_state.last_mouse_pos = mouse_position();
                }

                // draw loading screen
                login_screen.draw_loading(
                    loading_assets.len(),
                    loading_index,
                    &loading_current,
                    loading_started,
                );
            }

            GameState::Playing => {
                // F3 → open dev mode (code prompt if not yet verified)
                if is_key_pressed(KeyCode::F3) {
                    playing_state.dev_mode.request_open();
                    if playing_state.dev_mode.is_unlocked() {
                        state = GameState::DevMode;
                        set_cursor_grab(false);
                        show_mouse(true);
                        playing_state.clear_input();
                        next_frame().await;
                        continue;
                    }
                }



                // handle code prompt while playing (if F3 was pressed but code not yet verified)
                if playing_state.dev_mode.show_code_prompt {
                    set_cursor_grab(false); // show cursor to know ur in prompt mode
                    show_mouse(true);
                    let assets = sync_world(&socket, player_id, &mut playing_state, &mut maze);
                    if let Some(entries) = assets {
                        for (key, path) in entries {
                            match load_texture(&path).await {
                                Ok(tex) => { tex.set_filter(FilterMode::Nearest); playing_state.assets.insert(key, tex); }
                                Err(_) => {}
                            }
                        }
                    }
                    playing_state.simulate(&config, maze.as_ref());
                    playing_state.render(&config, &username, player_id, maze.as_ref());
                    if DevModeState::update_code_prompt(&mut playing_state.dev_mode) {
                        state = GameState::DevMode;
                        set_cursor_grab(false);
                        show_mouse(true);
                        playing_state.clear_input();
                    }
                    DevModeState::draw_code_prompt(&playing_state.dev_mode);

                    // if prompt was closed (ESC) without entering dev mode, re-grab cursor for gameplay
                    if !playing_state.dev_mode.show_code_prompt && !matches!(state, GameState::DevMode) {
                        set_cursor_grab(true);
                        show_mouse(false);
                        playing_state.last_mouse_pos = mouse_position();
                    }

                    next_frame().await;
                    continue;
                }

                let assets = sync_world(&socket, player_id, &mut playing_state, &mut maze);

                if let Some(entries) = assets {
                    for (key, path) in entries {
                        println!("Loading asset: {} -> {}", key, path);
                        match load_texture(&path).await {
                            Ok(tex) => {
                                println!("  Loaded successfully");
                                tex.set_filter(FilterMode::Nearest);
                                playing_state.assets.insert(key, tex);
                            }
                            Err(e) => {
                                println!("  FAILED to load: {:?}", e);
                            }
                        }
                    }
                }

                if let Some(new_state) = playing_state.process_local_input(&config, maze.as_ref()) {
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
                // keep world up to date even while paused.
                let assets = sync_world(&socket, player_id, &mut playing_state, &mut maze);

                if let Some(entries) = assets {
                    for (key, path) in entries {
                        println!("Loading asset (paused): {} -> {}", key, path);
                        match load_texture(&path).await {
                            Ok(tex) => {
                                println!("  Loaded successfully");
                                tex.set_filter(FilterMode::Nearest);
                                playing_state.assets.insert(key, tex);
                            }
                            Err(e) => {
                                println!("  FAILED to load: {:?}", e);
                            }
                        }
                    }
                }

                playing_state.simulate(&config, maze.as_ref());
                playing_state.render(&config, &username, player_id, maze.as_ref());

                if let Some(new_state) = draw_pause_menu(&config, &mut settings_screen, &mut playing_state.dev_mode) {
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
                settings_screen.draw(&config, &playing_state.weapons, &playing_state.assets);
                if settings_screen.update(&mut config, &mut playing_state.weapons) {
                    state = GameState::Paused;
                }
            }

            GameState::DevMode => {
                // F3 or ESC to close dev mode
                if is_key_pressed(KeyCode::F3) {
                    state = GameState::Playing;
                    set_cursor_grab(true);
                    show_mouse(false);
                    playing_state.last_mouse_pos = mouse_position();
                    next_frame().await;
                    continue;
                }
                if config.bind_settings.is_pressed() {
                    state = GameState::Paused;
                    next_frame().await;
                    continue;
                }

                // keep world in sync + simulate + render behind the overlay
                let assets = sync_world(&socket, player_id, &mut playing_state, &mut maze);
                if let Some(entries) = assets {
                    for (key, path) in entries {
                        match load_texture(&path).await {
                            Ok(tex) => { tex.set_filter(FilterMode::Nearest); playing_state.assets.insert(key, tex); }
                            Err(_) => {}
                        }
                    }
                }
                playing_state.simulate(&config, maze.as_ref());
                playing_state.render(&config, &username, player_id, maze.as_ref());

                // draw dev overlay on top
                draw_dev_overlay(
                    &mut playing_state.dev_mode,
                    &mut playing_state.weapons,
                    &mut playing_state.movement_config,
                    &mut config,
                );
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
    maze: &mut Option<Maze>
) -> Option<Vec<(String, String)> /* assets to load */> {
    if let Some(ref sock) = socket {
        let updates = poll_updates(sock, player_id);
        if let Some((new_maze, spawn)) = updates.maze {
            *maze = Some(new_maze);
            // playing_state.position = spawn; // removed because player starts in lobby now
        }
        if let Some(players) = updates.players {
            playing_state.other_players = players;
        }

        if let Some((world_id, spawn)) = updates.world_change {
            playing_state.current_world = world_id;
            playing_state.position = spawn;
            playing_state.movement_state.velocity = Vec3::ZERO;
        }
        return updates.assets;
    }
    None
}
