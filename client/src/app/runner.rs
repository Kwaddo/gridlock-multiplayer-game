use macroquad::prelude::*;
use macroquad::window::gl_set_drawcall_buffer_capacity;
use std::net::UdpSocket;
use std::path::Path;
use std::time::{Duration, Instant};

use client::core::{GameConfig, GameState};
use client::screens::dev_mode::{draw_dev_overlay, DevModeState};
use client::screens::{draw_pause_menu, LoginScreen, ModelQuality, PlayingState};
use client::settings::SettingsScreen;
use client::types::maze::Maze;
use client::ui::fonts;
use client::weapons::crosshair_presets::CrosshairPresetManager;

use super::asset_cache::{load_cached_manifest, save_cached_manifest};
use super::asset_loader::load_asset_entry;
use super::dev_ui::draw_bot_tab;
use super::world_sync::sync_world;

pub async fn run() {
    set_default_filter_mode(FilterMode::Nearest);
    gl_set_drawcall_buffer_capacity(200_000, 300_000);

    fonts::init_font().await;

    let mut state = GameState::Login;
    let mut login_screen = LoginScreen::new();
    let mut settings_screen = SettingsScreen::new();
    let mut playing_state = PlayingState::new().await;

    let mut socket: Option<UdpSocket> = None;
    let mut player_id: u32 = 0;
    let mut username = String::new();
    let mut server_addr = String::new();
    let mut asset_cache_key = String::new();
    let mut model_quality = ModelQuality::Compressed;
    let mut config = GameConfig::new();
    let mut preset_manager = CrosshairPresetManager::new("");
    let mut maze: Option<Maze> = None;
    let mut is_fullscreen = true;

    // connection heartbeat / last seen from server
    let mut last_server_contact: Option<Instant> = None;
    const CLIENT_CONNECTION_LOST_TIMEOUT: Duration = Duration::from_secs(8);

    let mut loading_assets: Vec<(String, String)> = Vec::new();
    let mut loading_index: usize = 0;
    let mut loading_current: String = String::new();
    let mut loading_stable_frames: u8 = 0;
    let mut loading_started: bool = false;

    loop {
        if config.bind_fullscreen.is_pressed() {
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
                    model_quality = login_screen.model_quality;
                    asset_cache_key = format!(
                        "{}|model_quality={}",
                        server_addr,
                        model_quality.label().to_lowercase()
                    );
                    // mark server contact as fresh on connect
                    last_server_contact = Some(Instant::now());

                    preset_manager = CrosshairPresetManager::new(&username);

                    loading_assets = resolve_asset_entries_for_quality(
                        load_cached_manifest(&asset_cache_key),
                        model_quality,
                    );
                    loading_index = 0;
                    loading_current.clear();
                    loading_stable_frames = 0;
                    loading_started = !loading_assets.is_empty();
                    state = GameState::Loading;
                }
                login_screen.draw();
            }

            GameState::Loading => {
                login_screen.update_background();

                let sync = sync_world(&socket, player_id, &mut playing_state, &mut maze);
                if sync.removed_by_server {
                    // server removed our player (timed out) — go back to connection page
                    socket = None;
                    login_screen.error_msg = "Timed out / disconnected by server.".to_string();
                    state = GameState::Login;
                    set_cursor_grab(false);
                    show_mouse(true);
                    playing_state.clear_input();
                    last_server_contact = None;
                    continue;
                }
                if sync.server_seen {
                    last_server_contact = Some(Instant::now());
                }

                if let Some(entries) = sync.assets {
                    if !entries.is_empty() {
                        merge_manifest_entries(
                            &mut loading_assets,
                            resolve_asset_entries_for_quality(entries, model_quality),
                        );
                        save_cached_manifest(&asset_cache_key, &loading_assets);
                        loading_stable_frames = 0;
                    }
                }

                if !loading_started {
                    loading_stable_frames += 1;
                    if !loading_assets.is_empty() && loading_stable_frames >= 3 {
                        loading_started = true;
                    }
                    if loading_stable_frames >= 30 {
                        loading_started = true;
                    }
                }

                if loading_started && loading_index < loading_assets.len() {
                    let (key, path) = loading_assets[loading_index].clone();
                    loading_current = key.clone();
                    load_asset_entry(key, path, &mut playing_state).await;
                    loading_index += 1;
                }

                if loading_started && loading_index >= loading_assets.len() {
                    state = GameState::Playing;
                    set_cursor_grab(true);
                    show_mouse(false);
                    playing_state.last_mouse_pos = mouse_position();
                }

                login_screen.draw_loading(
                    loading_assets.len(),
                    loading_index,
                    &loading_current,
                    loading_started,
                );
            }

            GameState::Playing => {
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

                if playing_state.dev_mode.show_code_prompt {
                    set_cursor_grab(false);
                    show_mouse(true);
                    let sync = sync_world(&socket, player_id, &mut playing_state, &mut maze);
                    if sync.removed_by_server {
                        socket = None;
                        login_screen.error_msg = "Timed out / disconnected by server.".to_string();
                        state = GameState::Login;
                        set_cursor_grab(false);
                        show_mouse(true);
                        playing_state.clear_input();
                        last_server_contact = None;
                        continue;
                    }
                    if sync.server_seen {
                        last_server_contact = Some(Instant::now());
                    }
                    if let Some(entries) = sync.assets {
                        for (key, path) in resolve_asset_entries_for_quality(entries, model_quality)
                        {
                            load_asset_entry(key, path, &mut playing_state).await;
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

                    if !playing_state.dev_mode.show_code_prompt
                        && !matches!(state, GameState::DevMode)
                    {
                        set_cursor_grab(true);
                        show_mouse(false);
                        playing_state.last_mouse_pos = mouse_position();
                    }

                    next_frame().await;
                    continue;
                }

                let sync = sync_world(&socket, player_id, &mut playing_state, &mut maze);

                if sync.removed_by_server {
                    socket = None;
                    login_screen.error_msg = "Timed out / disconnected by server.".to_string();
                    state = GameState::Login;
                    set_cursor_grab(false);
                    show_mouse(true);
                    playing_state.clear_input();
                    last_server_contact = None;
                    continue;
                }
                if sync.server_seen {
                    last_server_contact = Some(Instant::now());
                }

                if let Some(entries) = sync.assets {
                    for (key, path) in resolve_asset_entries_for_quality(entries, model_quality) {
                        load_asset_entry(key, path, &mut playing_state).await;
                    }
                }

                if let Some(new_state) = playing_state.process_local_input(&config, maze.as_mut()) {
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
                let sync = sync_world(&socket, player_id, &mut playing_state, &mut maze);

                if sync.removed_by_server {
                    socket = None;
                    login_screen.error_msg = "Timed out / disconnected by server.".to_string();
                    state = GameState::Login;
                    set_cursor_grab(false);
                    show_mouse(true);
                    playing_state.clear_input();
                    last_server_contact = None;
                    continue;
                }
                if sync.server_seen {
                    last_server_contact = Some(Instant::now());
                }

                if let Some(entries) = sync.assets {
                    for (key, path) in resolve_asset_entries_for_quality(entries, model_quality) {
                        load_asset_entry(key, path, &mut playing_state).await;
                    }
                }

                playing_state.simulate(&config, maze.as_ref());
                playing_state.render(&config, &username, player_id, maze.as_ref());

                if let Some(new_state) =
                    draw_pause_menu(&config, &mut settings_screen, &mut playing_state.dev_mode)
                {
                    state = new_state;
                    if matches!(state, GameState::Playing) {
                        set_cursor_grab(true);
                        show_mouse(false);
                        playing_state.last_mouse_pos = mouse_position();
                    }
                }
            }

            GameState::Settings => {
                let sync = sync_world(&socket, player_id, &mut playing_state, &mut maze);
                if sync.removed_by_server {
                    socket = None;
                    login_screen.error_msg = "Timed out / disconnected by server.".to_string();
                    state = GameState::Login;
                    set_cursor_grab(false);
                    show_mouse(true);
                    playing_state.clear_input();
                    last_server_contact = None;
                    continue;
                }
                if sync.server_seen {
                    last_server_contact = Some(Instant::now());
                }
                if let Some(entries) = sync.assets {
                    for (key, path) in resolve_asset_entries_for_quality(entries, model_quality) {
                        load_asset_entry(key, path, &mut playing_state).await;
                    }
                }
                playing_state.simulate(&config, maze.as_ref());
                playing_state.render(&config, &username, player_id, maze.as_ref());

                draw_rectangle(
                    0.0,
                    0.0,
                    screen_width(),
                    screen_height(),
                    Color::from_rgba(0, 0, 0, 180),
                );

                settings_screen.draw(
                    &config,
                    &playing_state.weapons,
                    &preset_manager,
                    &playing_state.assets,
                );
                if settings_screen.update(
                    &mut config,
                    &mut playing_state.weapons,
                    &mut preset_manager,
                ) {
                    state = GameState::Paused;
                }
            }

            GameState::DevMode => {
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

                let sync = sync_world(&socket, player_id, &mut playing_state, &mut maze);
                if sync.removed_by_server {
                    socket = None;
                    login_screen.error_msg = "Timed out / disconnected by server.".to_string();
                    state = GameState::Login;
                    set_cursor_grab(false);
                    show_mouse(true);
                    playing_state.clear_input();
                    last_server_contact = None;
                    continue;
                }
                if sync.server_seen {
                    last_server_contact = Some(Instant::now());
                }
                if let Some(entries) = sync.assets {
                    for (key, path) in resolve_asset_entries_for_quality(entries, model_quality) {
                        load_asset_entry(key, path, &mut playing_state).await;
                    }
                }
                playing_state.simulate(&config, maze.as_ref());
                playing_state.render(&config, &username, player_id, maze.as_ref());

                draw_dev_overlay(
                    &mut playing_state.dev_mode,
                    &mut playing_state.weapons,
                    &mut playing_state.movement_config,
                    &mut config,
                    |x, y, w, mouse, clicked, _held| {
                        draw_bot_tab(
                            &mut playing_state.practice_bots,
                            &mut playing_state.next_bot_id,
                            playing_state.current_world,
                            x,
                            y,
                            w,
                            mouse,
                            clicked,
                        );
                    },
                );
            }

            GameState::Exit => {
                if let Some(ref s) = socket {
                    let disconnect_msg = vec![client::net::protocols::MSG_DISCONNECT];
                    let _ = s.send_to(&disconnect_msg, &server_addr);
                }
                std::process::exit(0);
            }
        }

        // Global: if we haven't heard from the server for a while, treat as disconnected
        if socket.is_some() {
            if let Some(last) = last_server_contact {
                if Instant::now().duration_since(last) > CLIENT_CONNECTION_LOST_TIMEOUT {
                    socket = None;
                    login_screen.error_msg = "Connection to server lost.".to_string();
                    state = GameState::Login;
                    set_cursor_grab(false);
                    show_mouse(true);
                    playing_state.clear_input();
                    last_server_contact = None;
                }
            }
        }

        next_frame().await;
    }
}

fn merge_manifest_entries(existing: &mut Vec<(String, String)>, incoming: Vec<(String, String)>) {
    for (key, path) in incoming {
        if let Some((_, existing_path)) = existing.iter_mut().find(|(k, _)| *k == key) {
            *existing_path = path;
        } else {
            existing.push((key, path));
        }
    }
}

fn resolve_asset_entries_for_quality(
    entries: Vec<(String, String)>,
    model_quality: ModelQuality,
) -> Vec<(String, String)> {
    entries
        .into_iter()
        .map(|(key, path)| (key, resolve_asset_path_for_quality(&path, model_quality)))
        .collect()
}

fn resolve_asset_path_for_quality(path: &str, model_quality: ModelQuality) -> String {
    let normalized = path.replace('\\', "/");
    if !normalized.ends_with(".glb") || !normalized.starts_with("assets/models/player/") {
        return path.to_string();
    }

    let Some(file_name) = Path::new(&normalized)
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
    else {
        return path.to_string();
    };

    let preferred_dir = match model_quality {
        ModelQuality::Compressed => "assets/models/player/Compressed",
        ModelQuality::Full => "assets/models/player/Full",
    };
    let preferred_path = format!("{}/{}", preferred_dir, file_name);
    if Path::new(&preferred_path).exists() {
        return preferred_path;
    }

    // Requested behavior: if compressed is missing but full exists, use Rifle_Idle fallback.
    if matches!(model_quality, ModelQuality::Compressed) {
        let full_path = format!("assets/models/player/Full/{}", file_name);
        if Path::new(&full_path).exists() {
            let compressed_idle = "assets/models/player/Compressed/Rifle_Idle.glb";
            if Path::new(compressed_idle).exists() {
                return compressed_idle.to_string();
            }
            let full_idle = "assets/models/player/Full/Rifle_Idle.glb";
            if Path::new(full_idle).exists() {
                return full_idle.to_string();
            }
        }
    }

    let preferred_idle = match model_quality {
        ModelQuality::Compressed => "assets/models/player/Compressed/Rifle_Idle.glb",
        ModelQuality::Full => "assets/models/player/Full/Rifle_Idle.glb",
    };
    if Path::new(preferred_idle).exists() {
        return preferred_idle.to_string();
    }
    let any_idle = "assets/models/player/Full/Rifle_Idle.glb";
    if Path::new(any_idle).exists() {
        return any_idle.to_string();
    }

    path.to_string()
}
