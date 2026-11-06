use macroquad::prelude::*;
use macroquad::window::gl_set_drawcall_buffer_capacity;
use client::net::Session;
use transport::Link;
use std::path::Path;
use platform::{Duration, Instant};

use client::core::{ GameConfig, GameState };
use client::screens::dev_mode::{ draw_dev_overlay, DevModeState };
use client::screens::{ LobbyInventoryScreen, LoginScreen, PauseAction, PauseMenu, PlayingState };
use client::settings::SettingsScreen;
use client::types::maze::Maze;
use client::ui::fonts;
use client::weapons::crosshair_presets::CrosshairPresetManager;
use client::weapons::loadout::Loadout;
use client::weapons::WeaponInventory;

use super::asset_loader::{
    asset_loaded,
    download_progress,
    load_asset_entry,
    load_manifest,
    preload,
    preload_status,
    Prefetcher,
    PREFETCH_AHEAD,
};
use super::dev_ui::draw_bot_tab;
use super::world_sync::sync_world;

// asset quality selector used when resolving which version of a model to
// request from the server.  the value is passed down to
// `resolve_asset_path_for_quality`, which currently just uses it when
// choosing between `Compressed`/`Full` folders; defaults to `High`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
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

fn set_cwd_to_assets_root() {
    // The browser build fetches assets relative to the page instead.
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(|p| p.to_path_buf());
        while let Some(d) = dir {
            if d.join("assets").is_dir() {
                let _ = std::env::set_current_dir(&d);
                return;
            }
            dir = d.parent().map(|p| p.to_path_buf());
        }
    }
}

pub async fn run() {
    set_cwd_to_assets_root();
    set_default_filter_mode(FilterMode::Nearest);
    gl_set_drawcall_buffer_capacity(200_000, 300_000);

    fonts::init_font().await;

    let mut state = GameState::Login;
    let mut login_screen = LoginScreen::new();
    let mut settings_screen = SettingsScreen::new();
    let mut pause_menu = PauseMenu::default();
    let mut playing_state = PlayingState::new().await;

    let mut session: Option<Session> = None;
    let mut player_id: u32 = 0;
    let mut username = String::new();
    let mut config = GameConfig::load();
    let mut preset_manager = CrosshairPresetManager::new("");
    let mut maze: Option<Maze> = None;
    let mut is_fullscreen = true;
    let mut loadout = Loadout::default_loadout();
    let mut lobby_inventory = LobbyInventoryScreen::new();

    // currently we always use high quality; eventually this could be
    // user-configurable via the settings screen.
    let model_quality = ModelQuality::default();

    // connection heartbeat / last seen from server
    let mut last_server_contact: Option<Instant> = None;
    const CLIENT_CONNECTION_LOST_TIMEOUT: Duration = Duration::from_secs(8);
    // keepalive: periodically send a lightweight position update while connected so the
    // server does not mark us inactive when we are idle/paused.
    let mut last_keepalive: Option<Instant> = None;

    let mut loading_assets: Vec<(String, String)> = Vec::new();
    let mut loading_index: usize = 0;
    let mut loading_current: String = String::new();
    let mut prefetcher = Prefetcher::default();

    // Everything a game loads: the asset manifest (one list for every game,
    // kept with the assets) plus the first-person viewmodels. Downloading
    // starts now, while the player is still on the title screen, so joining
    // only reads it from the browser's cache.
    let game_assets = {
        let mut entries = load_manifest().await;
        entries.extend(missing_viewmodel_assets(&playing_state));
        resolve_asset_entries_for_quality(&entries, model_quality)
    };
    for (_, path) in &game_assets {
        preload(path);
    }

    let mut was_on_login = true;

    loop {
        // Keys pressed while playing (WASD...) also queue up as typed text;
        // drop them when the title screen comes back, or they would be typed
        // into its fields.
        let on_login = matches!(state, GameState::Login);
        if on_login && !was_on_login {
            while get_char_pressed().is_some() {}
        }
        was_on_login = on_login;

        // In a hidden browser tab these frames come from a background timer
        // (web/gridlock_web.js) and only keep the game going: step a game we
        // host, take in what the server sent, send keepalives. Nothing is
        // simulated or drawn. (Asset lists only arrive while loading, which
        // still runs in full.)
        if platform::page_hidden() && session.is_some() && !matches!(state, GameState::Loading) {
            let sync = sync_world(link_of(&mut session), player_id, &username, &mut playing_state, &mut maze);
            if sync.removed_by_server {
                session = None;
                login_screen.returned_with("Timed out / disconnected by server.");
                state = GameState::Login;
                last_server_contact = None;
                continue;
            }
            if sync.server_seen {
                last_server_contact = Some(Instant::now());
            }
            // A match that ended while hidden still hands back the lobby loadout.
            if sync.returned_to_lobby {
                playing_state.weapons = WeaponInventory::from_loadout(&loadout);
            }
            if let Some(host) = session.as_mut().and_then(|s| s.host.as_mut()) {
                host.update();
            }
            if last_keepalive.map_or(true, |t| t.elapsed() > Duration::from_secs(4)) {
                playing_state.send_network(link_of(&mut session), &config);
                last_keepalive = Some(Instant::now());
            }
            next_frame().await;
            continue;
        }

        if config.bind_fullscreen.is_pressed() {
            is_fullscreen = !is_fullscreen;
            set_fullscreen(is_fullscreen);
        }

        match state {
            GameState::Login => {
                if macroquad::input::is_key_pressed(macroquad::input::KeyCode::Escape) {
                    state = GameState::Exit;
                }
                if let Some(joined) = login_screen.update() {
                    playing_state.chat.clear();
                    pause_menu.reset();
                    username = joined.username.clone();
                    player_id = joined.player_id;
                    session = Some(joined);
                    // mark server contact as fresh on connect
                    last_server_contact = Some(Instant::now());

                    preset_manager = CrosshairPresetManager::new(&username);

                    loading_assets = game_assets.clone();
                    loading_index = 0;
                    loading_current.clear();
                    prefetcher.clear();
                    state = GameState::Loading;
                }
                login_screen.draw();
                if let Some((done, total)) = preload_status().filter(|(done, total)| done < total) {
                    login_screen.draw_preload(done, total);
                }
            }

            GameState::LobbyInventory => {
                let sync = sync_world(link_of(&mut session), player_id, &username, &mut playing_state, &mut maze);
                if sync.removed_by_server {
                    session = None;
                    login_screen.returned_with("Timed out / disconnected by server.");
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
                playing_state.simulate(&config, maze.as_ref());
                playing_state.render(&mut config, &username, player_id, maze.as_ref());

                draw_rectangle(
                    0.0,
                    0.0,
                    screen_width(),
                    screen_height(),
                    Color::from_rgba(0, 0, 0, 150)
                );
                lobby_inventory.draw(&loadout);
                if lobby_inventory.update(&mut loadout) {
                    playing_state.weapons = WeaponInventory::from_loadout(&loadout);
                    state = GameState::Paused;
                }
            }

            GameState::Loading => {
                login_screen.update_background();

                let sync = sync_world(link_of(&mut session), player_id, &username, &mut playing_state, &mut maze);
                if sync.removed_by_server {
                    // server removed our player (timed out) — go back to connection page
                    session = None;
                    login_screen.returned_with("Timed out / disconnected by server.");
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


                if loading_index < loading_assets.len() {
                    // Download a few ahead in the background; load the next
                    // one only once its bytes are here, so no frame waits on
                    // the network.
                    // Assets already loaded (e.g. joining a second game) need no download.
                    for (key, path) in loading_assets.iter().skip(loading_index).take(PREFETCH_AHEAD) {
                        if !asset_loaded(&playing_state, key, path) {
                            prefetcher.start(path);
                        }
                    }
                    let (key, path) = loading_assets[loading_index].clone();
                    loading_current = key.clone();
                    if asset_loaded(&playing_state, &key, &path) || prefetcher.ready(&path) {
                        load_asset_entry(&key, &path, &mut playing_state).await;
                        loading_index += 1;
                    }
                }

                if loading_index >= loading_assets.len() {
                    playing_state.weapons = WeaponInventory::from_loadout(&loadout);
                    state = GameState::Playing;
                    // Loading paused this loop, so time spent there is not silence from the server.
                    last_server_contact = Some(Instant::now());
                    set_cursor_grab(true);
                    show_mouse(false);
                    playing_state.last_mouse_pos = mouse_position();
                }

                let download = loading_assets
                    .get(loading_index)
                    .and_then(|(_, path)| download_progress(path));
                login_screen.draw_loading(
                    loading_assets.len(),
                    loading_index,
                    &loading_current,
                    download
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
                    let sync = sync_world(
                        link_of(&mut session),
                        player_id,
                        &username,
                        &mut playing_state,
                        &mut maze
                    );
                    if sync.removed_by_server {
                        session = None;
                        login_screen.returned_with("Timed out / disconnected by server.");
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

                    playing_state.simulate(&config, maze.as_ref());
                    playing_state.render(&mut config, &username, player_id, maze.as_ref());
                    if DevModeState::update_code_prompt(&mut playing_state.dev_mode) {
                        state = GameState::DevMode;
                        set_cursor_grab(false);
                        show_mouse(true);
                        playing_state.clear_input();
                    }
                    DevModeState::draw_code_prompt(&playing_state.dev_mode);

                    if
                        !playing_state.dev_mode.show_code_prompt &&
                        !matches!(state, GameState::DevMode)
                    {
                        set_cursor_grab(true);
                        show_mouse(false);
                        playing_state.last_mouse_pos = mouse_position();
                    }

                    next_frame().await;
                    continue;
                }

                let sync = sync_world(link_of(&mut session), player_id, &username, &mut playing_state, &mut maze);

                if sync.removed_by_server {
                    session = None;
                    login_screen.returned_with("Timed out / disconnected by server.");
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


                if sync.returned_to_lobby {
                    playing_state.weapons = WeaponInventory::from_loadout(&loadout);
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
                playing_state.send_network(link_of(&mut session), &config);
                playing_state.render(&mut config, &username, player_id, maze.as_ref());
            }

            GameState::Paused => {
                let sync = sync_world(link_of(&mut session), player_id, &username, &mut playing_state, &mut maze);

                if sync.removed_by_server {
                    session = None;
                    login_screen.returned_with("Timed out / disconnected by server.");
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


                if sync.returned_to_lobby {
                    playing_state.weapons = WeaponInventory::from_loadout(&loadout);
                }

                playing_state.simulate(&config, maze.as_ref());
                playing_state.render(&mut config, &username, player_id, maze.as_ref());

                let in_match = playing_state.current_world == client::types::world::WorldId::Maze;
                let is_host = playing_state.is_host(player_id);
                if
                    let Some(action) = pause_menu.draw(
                        &config,
                        &mut settings_screen,
                        &mut playing_state.dev_mode,
                        in_match,
                        is_host
                    )
                {
                    // Ending or leaving the match keeps the connection: the
                    // server moves us back to the lobby and play goes on there.
                    state = match action {
                        PauseAction::Go(next) => next,
                        PauseAction::EndMatch => {
                            playing_state.pending_end_match = true;
                            GameState::Playing
                        }
                        PauseAction::LeaveMatch => {
                            playing_state.pending_world_change = Some(
                                client::types::world::WorldId::Lobby
                            );
                            GameState::Playing
                        }
                    };
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
                let sync = sync_world(link_of(&mut session), player_id, &username, &mut playing_state, &mut maze);
                if sync.removed_by_server {
                    session = None;
                    login_screen.returned_with("Timed out / disconnected by server.");
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

                playing_state.simulate(&config, maze.as_ref());
                playing_state.render(&mut config, &username, player_id, maze.as_ref());

                draw_rectangle(
                    0.0,
                    0.0,
                    screen_width(),
                    screen_height(),
                    Color::from_rgba(0, 0, 0, 180)
                );

                settings_screen.draw(
                    &config,
                    &playing_state.weapons,
                    &preset_manager,
                    &playing_state.assets
                );
                if
                    settings_screen.update(
                        &mut config,
                        &mut playing_state.weapons,
                        &mut preset_manager
                    )
                {
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

                let sync = sync_world(link_of(&mut session), player_id, &username, &mut playing_state, &mut maze);
                if sync.removed_by_server {
                    session = None;
                    login_screen.returned_with("Timed out / disconnected by server.");
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

                playing_state.simulate(&config, maze.as_ref());
                playing_state.render(&mut config, &username, player_id, maze.as_ref());

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
                            clicked
                        );
                    }
                );
            }

            GameState::Exit => {
                if let Some(s) = link_of(&mut session) {
                    s.send(&client::net::protocols::ClientMsg::Disconnect.encode());
                }
                // process::exit skips destructors: stop a hosted server cleanly first
                // (its online listing, if any, expires by itself)
                drop(session.take());
                // A page cannot quit itself; go back to the menu instead.
                #[cfg(target_arch = "wasm32")]
                {
                    state = GameState::Login;
                }
                #[cfg(not(target_arch = "wasm32"))]
                std::process::exit(0);
            }
        }

        // A game we host: step it (the browser runs the server in this loop), and
        // pass on what the host should know, e.g. whether outsiders can reach it.
        if let Some(host) = session.as_mut().and_then(|s| s.host.as_mut()) {
            host.update();
            if let Some(notice) = host.take_notice() {
                playing_state.announcements.push((notice, 8.0));
            }
        }

        // Keepalive: if connected, periodically send a lightweight position update so the
        // server treats us as active even when idle/paused. Interval is less than server timeout.
        if session.is_some() {
            if
                last_keepalive
                    .map(|t| Instant::now().duration_since(t) > Duration::from_secs(4))
                    .unwrap_or(true)
            {
                playing_state.send_network(link_of(&mut session), &config);
                last_keepalive = Some(Instant::now());
            }
        }

        // Global: if we haven't heard from the server for a while, treat as disconnected.
        // Not while loading: each asset load is awaited inside this loop, and a large
        // model on a first visit to the browser build can take longer than the timeout.
        if session.is_some() && !matches!(state, GameState::Loading) {
            if let Some(last) = last_server_contact {
                if Instant::now().duration_since(last) > CLIENT_CONNECTION_LOST_TIMEOUT {
                    session = None;
                    login_screen.returned_with("Connection to server lost.");
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

fn resolve_asset_entries_for_quality(
    entries: &[(String, String)],
    model_quality: ModelQuality
) -> Vec<(String, String)> {
    // the `model_quality` parameter is currently unused; kept so we can
    // support different heuristics later (low/medium/high).
    let _ = model_quality;
    entries
        .iter()
        .map(|(key, path)| { (key.clone(), resolve_asset_path_for_quality(path, model_quality)) })
        .collect()
}

fn resolve_asset_path_for_quality(path: &str, _model_quality: ModelQuality) -> String {
    // avoid allocating a normalized copy unless we know it's a player GLB file
    if !path.ends_with(".glb") || !path.starts_with("assets/models/player/") {
        return path.to_string();
    }
    let normalized = if path.contains('\\') { path.replace('\\', "/") } else { path.to_string() };

    // The browser cannot probe which variants exist; the web build ships the
    // compressed set.
    #[cfg(target_arch = "wasm32")]
    if let Some(file_name) = normalized.rsplit('/').next() {
        return format!("assets/models/player/Compressed/{}", file_name);
    }

    if Path::new(&normalized).exists() {
        return normalized;
    }

    let Some(file_name) = Path::new(&normalized)
        .file_name()
        .and_then(|s| s.to_str()) else {
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

/// Viewmodel assets (first-person hands and weapons) the game needs that are
/// not loaded yet.
fn missing_viewmodel_assets(playing_state: &PlayingState) -> Vec<(String, String)> {
    let required = [
        ("fp_hands", "assets/models/hand/fps_animated_carbine.glb"),
        ("fp_hands_pistol", "assets/models/hand/fps_pistol_animated.glb"),

        ("fp_hands_smg", "assets/models/hand/fps_animated_smg.glb"),
        ("fp_hands_sniper", "assets/models/hand/sniper_animated.glb"),
        ("fp_hands_minigun", "assets/models/hand/minigun_animated.glb"),
        ("fp_hands_sawnoff", "assets/models/hand/sawnoff_animated.glb"),
        ("m1911_model", "assets/models/weapons/Pistols/m1911.glb"),
        ("deagle_model", "assets/models/weapons/Pistols/deagle.glb"),
        ("mp5_model", "assets/models/weapons/SMGs/mp5.glb"),
        ("spas_model", "assets/models/weapons/Shotguns/spas.glb"),
        ("m16_model", "assets/models/weapons/Assault Rifles/m16.glb"),
        ("bolt_action_model", "assets/models/weapons/Snipers/bolt_action.glb"),
        ("minigun_model", "assets/models/weapons/Other/minigun.glb"),
    ];

    required
        .into_iter()
        .filter(|(key, _)| {
            !playing_state.models.contains_key(*key) &&
                !playing_state.animated_models.contains_key(*key)
        })
        .map(|(key, path)| (key.to_string(), path.to_string()))
        .collect()
}

/// The live connection, for code that only sends and receives.
fn link_of(session: &mut Option<Session>) -> Option<&mut dyn Link> {
    match session {
        Some(s) => Some(s.link.as_mut()),
        None => None,
    }
}
