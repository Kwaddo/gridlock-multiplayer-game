use macroquad::prelude::*;
use macroquad::window::gl_set_drawcall_buffer_capacity;
use client::net::Session;
use platform::Instant;

use client::core::{ GameConfig, GameState };
use client::screens::dev_mode::DevModeState;
use client::screens::{ LobbyInventoryScreen, LoginScreen, PauseMenu, PlayingState };
use client::settings::SettingsScreen;
use client::types::maze::Maze;
use client::ui::fonts;
use client::weapons::crosshair_presets::CrosshairPresetManager;
use client::weapons::loadout::Loadout;
use client::weapons::WeaponInventory;

use super::asset_loader::{
    load_manifest,
    preload,
    preload_status,
    AssetQueue,
};
use super::world_sync::sync_world;

mod assets;
mod constants;
mod frame;
mod screens;

use assets::{
    missing_viewmodel_assets,
    resolve_asset_entries_for_quality,
    set_cwd_to_assets_root,
    spawn_assets,
    ModelQuality,
};
use constants::CONNECTING_FRAME_BUDGET;
use frame::{ finish_frame, hidden_tab_frame, link_of };
use screens::{ dev_overlay_frame, loading_frame, pause_menu_frame, settings_frame };

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
    // keepalive: periodically send a lightweight position update while connected so the
    // server does not mark us inactive when we are idle/paused.
    let mut last_keepalive: Option<Instant> = None;

    // Joining waits only for what spawning needs; the rest of the game's
    // assets keep loading in the background while playing.
    let mut asset_queue = AssetQueue::default();
    // Whether `asset_queue` was started on the title screen: the game's
    // assets do not depend on which game it is, so loading starts before
    // the player even picks one, and the loading screen after the join has
    // little or nothing left to do.
    let mut queued_early = false;
    // When the join's Welcome came in, for the log of how long loading took.
    let mut joined_at: Option<Instant> = None;

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
            hidden_tab_frame(
                &mut session, player_id, &username, &mut playing_state, &mut maze,
                &mut login_screen, &mut state, &mut last_server_contact, &loadout,
                &mut last_keepalive, &config
            ).await;
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
                // Load in the background while the player picks a game, and
                // while it connects; within a frame budget, so the title
                // screen (and the connection's own work) keeps its frames.
                if !queued_early {
                    asset_queue = AssetQueue::start(&game_assets, spawn_assets(&loadout), &playing_state);
                    queued_early = true;
                }
                let frame_start = Instant::now();
                while asset_queue.pump(&mut playing_state).await
                    && frame_start.elapsed() < CONNECTING_FRAME_BUDGET
                    && !asset_queue.ready_to_play()
                {}
                if let Some(joined) = login_screen.update() {
                    playing_state.chat.clear();
                    pause_menu.reset();
                    username = joined.username.clone();
                    player_id = joined.player_id;
                    session = Some(joined);
                    // mark server contact as fresh on connect
                    last_server_contact = Some(Instant::now());

                    preset_manager = CrosshairPresetManager::new(&username);

                    if queued_early {
                        // The loadout may have changed since loading started.
                        asset_queue.require(spawn_assets(&loadout));
                    } else {
                        asset_queue = AssetQueue::start(
                            &game_assets,
                            spawn_assets(&loadout),
                            &playing_state
                        );
                    }
                    queued_early = false;
                    joined_at = Some(Instant::now());
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


                loading_frame(
                    &mut asset_queue, &loadout, &mut playing_state, &mut session, &mut joined_at,
                    &mut state, &mut last_server_contact, &mut login_screen
                ).await;
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

                pause_menu_frame(
                    &mut pause_menu, &config, &mut settings_screen, &mut playing_state, player_id,
                    &mut state, &mut lobby_inventory
                );
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

                settings_frame(
                    &mut config, &mut settings_screen, &mut playing_state, &mut preset_manager,
                    &mut state
                );
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

                dev_overlay_frame(&mut playing_state, &mut config);
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

        finish_frame(
            &mut session, &mut asset_queue, &mut state, &mut playing_state, &loadout,
            &mut last_keepalive, &config, &mut last_server_contact, &mut login_screen
        ).await;

        next_frame().await;
    }
}
