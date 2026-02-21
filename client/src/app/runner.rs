use macroquad::prelude::*;
use macroquad::window::gl_set_drawcall_buffer_capacity;
use std::net::UdpSocket;
use std::path::Path;
use std::time::{Duration, Instant};

use client::core::{GameConfig, GameState};
use client::screens::dev_mode::{draw_dev_overlay, DevModeState};
use client::screens::{draw_pause_menu, LobbyInventoryScreen, LoginScreen, PlayingState};
use client::settings::SettingsScreen;
use client::types::maze::Maze;
use client::ui::fonts;
use client::weapons::crosshair_presets::CrosshairPresetManager;
use client::weapons::loadout::Loadout;
use client::weapons::WeaponInventory;

use super::asset_cache::load_cached_manifest;
use super::asset_loader::load_asset_entry;
use super::dev_ui::draw_bot_tab;
use super::world_sync::sync_world;

// asset quality selector used when resolving which version of a model to
// request from the server.  the value is passed down to
// `resolve_asset_path_for_quality`, which currently just uses it when
// choosing between `Compressed`/`Full` folders; defaults to `High`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModelQuality {
    Low,
    Medium,
    High,
}

impl Default for ModelQuality {
    fn default() -> Self {
        ModelQuality::High
    }
}

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
    let mut config = GameConfig::new();
    let mut preset_manager = CrosshairPresetManager::new("");
    let mut maze: Option<Maze> = None;
    let mut is_fullscreen = true;
    let mut loadout = Loadout::default_loadout();
    let mut lobby_inventory = LobbyInventoryScreen::new();

    // currently we always use high quality; eventually this could be
    // user-configurable via the settings screen.
    let mut model_quality = ModelQuality::default();

    // connection heartbeat / last seen from server
    let mut last_server_contact: Option<Instant> = None;
    const CLIENT_CONNECTION_LOST_TIMEOUT: Duration = Duration::from_secs(8);
    // keepalive: periodically send a lightweight position update while connected so the
    // server does not mark us inactive when we are idle/paused.
    let mut last_keepalive: Option<Instant> = None;

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
                    asset_cache_key = server_addr.clone();
                    // mark server contact as fresh on connect
                    last_server_contact = Some(Instant::now());

                    preset_manager = CrosshairPresetManager::new(&username);

                    {
                        let cached = load_cached_manifest(&asset_cache_key);
                        loading_assets = resolve_asset_entries_for_quality(&cached, model_quality);
                    }
                    loading_index = 0;
                    loading_current.clear();
                    loading_stable_frames = 0;
                    loading_started = !loading_assets.is_empty();
                    state = GameState::Loading;
                }
                login_screen.draw();
            }

            GameState::LobbyInventory => {
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
                    for (key, path) in resolve_asset_entries_for_quality(&entries, model_quality) {
                        load_asset_entry(&key, &path, &mut playing_state).await;
                    }
                }
                playing_state.simulate(&config, maze.as_ref());
                playing_state.render(&config, &username, player_id, maze.as_ref());

                draw_rectangle(
                    0.0,
                    0.0,
                    screen_width(),
                    screen_height(),
                    Color::from_rgba(0, 0, 0, 150),
                );
                lobby_inventory.draw(&loadout);
                if lobby_inventory.update(&mut loadout) {
                    playing_state.weapons = WeaponInventory::from_loadout(&loadout);
                    state = GameState::Paused;
                }
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
                    for (key, path) in resolve_asset_entries_for_quality(&entries, model_quality) {
                        load_asset_entry(&key, &path, &mut playing_state).await;
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
                    let (ref key, ref path) = loading_assets[loading_index];
                    loading_current = key.clone();
                    load_asset_entry(key, path, &mut playing_state).await;
                    loading_index += 1;
                }

                if loading_started && loading_index >= loading_assets.len() {
                    ensure_essential_viewmodel_assets(&mut playing_state).await;
                    playing_state.weapons = WeaponInventory::from_loadout(&loadout);
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
                        for (key, path) in
                            resolve_asset_entries_for_quality(&entries, model_quality)
                        {
                            load_asset_entry(&key, &path, &mut playing_state).await;
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
                    for (key, path) in resolve_asset_entries_for_quality(&entries, model_quality) {
                        load_asset_entry(&key, &path, &mut playing_state).await;
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
                    for (key, path) in resolve_asset_entries_for_quality(&entries, model_quality) {
                        load_asset_entry(&key, &path, &mut playing_state).await;
                    }
                }

                playing_state.simulate(&config, maze.as_ref());
                playing_state.render(&config, &username, player_id, maze.as_ref());

                if let Some(new_state) = draw_pause_menu(
                    &config,
                    &mut settings_screen,
                    &mut playing_state.dev_mode,
                    maze.is_some(),
                ) {
                    state = new_state;
                    if matches!(state, GameState::LobbyInventory) {
                        lobby_inventory = LobbyInventoryScreen::new();
                    }
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
                    for (key, path) in resolve_asset_entries_for_quality(&entries, model_quality) {
                        load_asset_entry(&key, &path, &mut playing_state).await;
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
                    for (key, path) in resolve_asset_entries_for_quality(&entries, model_quality) {
                        load_asset_entry(&key, &path, &mut playing_state).await;
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

        // Keepalive: if connected, periodically send a lightweight position update so the
        // server treats us as active even when idle/paused. Interval is less than server timeout.
        if socket.is_some() {
            if last_keepalive
                .map(|t| Instant::now().duration_since(t) > Duration::from_secs(4))
                .unwrap_or(true)
            {
                playing_state.send_network(&socket, &server_addr);
                last_keepalive = Some(Instant::now());
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
    use std::collections::HashMap;

    // build a quick lookup for the existing entries so we avoid an O(n²) scan
    let mut index_map: HashMap<String, usize> = HashMap::with_capacity(existing.len());
    for (i, (k, _)) in existing.iter().enumerate() {
        index_map.insert(k.clone(), i);
    }

    for (key, path) in incoming {
        if let Some(&idx) = index_map.get(&key) {
            existing[idx].1 = path;
        } else {
            index_map.insert(key.clone(), existing.len());
            existing.push((key, path));
        }
    }
}

fn resolve_asset_entries_for_quality(
    entries: &[(String, String)],
    model_quality: ModelQuality,
) -> Vec<(String, String)> {
    // the `model_quality` parameter is currently unused; kept so we can
    // support different heuristics later (low/medium/high).
    let _ = model_quality;
    entries
        .iter()
        .map(|(key, path)| {
            (
                key.clone(),
                resolve_asset_path_for_quality(path, model_quality),
            )
        })
        .collect()
}

fn resolve_asset_path_for_quality(path: &str, model_quality: ModelQuality) -> String {
    // avoid allocating a normalized copy unless we know it's a player GLB file
    if !path.ends_with(".glb") || !path.starts_with("assets/models/player/") {
        return path.to_string();
    }
    let normalized = if path.contains('\\') {
        path.replace('\\', "/")
    } else {
        path.to_string()
    };

    if Path::new(&normalized).exists() {
        return normalized;
    }

    let Some(file_name) = Path::new(&normalized).file_name().and_then(|s| s.to_str()) else {
        return path.to_string();
    };

    let compressed_path = format!("assets/models/player/Compressed/{}", file_name);
    if Path::new(&compressed_path).exists() {
        return compressed_path;
    }

    let full_path = format!("assets/models/player/Full/{}", file_name);
    if Path::new(&full_path).exists() {
        return full_path;
    }

    let flat_path = format!("assets/models/player/{}", file_name);
    if Path::new(&flat_path).exists() {
        return flat_path;
    }

    let compressed_idle = "assets/models/player/Compressed/Rifle_Idle.glb";
    if Path::new(compressed_idle).exists() {
        return compressed_idle.to_string();
    }

    let full_idle = "assets/models/player/Full/Rifle_Idle.glb";
    if Path::new(full_idle).exists() {
        return full_idle.to_string();
    }

    let idle = "assets/models/player/Rifle_Idle.glb";
    if Path::new(idle).exists() {
        return idle.to_string();
    }

    path.to_string()
}

async fn ensure_essential_viewmodel_assets(playing_state: &mut PlayingState) {
    let required = [
        ("fp_hands", "assets/models/hand/fps_animated_carbine.glb"),
        ("m1911_model", "assets/models/weapons/Pistols/m1911.glb"),
        ("deagle_model", "assets/models/weapons/Pistols/deagle.glb"),
        ("mp5_model", "assets/models/weapons/SMGs/mp5.glb"),
        ("spas_model", "assets/models/weapons/Shotguns/spas.glb"),
        ("m16_model", "assets/models/weapons/Assault Rifles/m16.glb"),
        (
            "bolt_action_model",
            "assets/models/weapons/Snipers/bolt_action.glb",
        ),
        ("minigun_model", "assets/models/weapons/Other/minigun.glb"),
    ];

    for (key, path) in required {
        if playing_state.models.contains_key(key) || playing_state.animated_models.contains_key(key)
        {
            continue;
        }
        load_asset_entry(key, path, playing_state).await;
    }
}
