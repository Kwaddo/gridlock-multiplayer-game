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
    download_progress,
    load_manifest,
    preload,
    preload_status,
    AssetQueue,
};
use super::dev_ui::draw_bot_tab;
use super::world_sync::sync_world;

/// Time the loading screen may spend loading assets each frame; it keeps the
/// screen drawing (and a game hosted in this tab answering) at about 60 fps.
const LOADING_FRAME_BUDGET: Duration = Duration::from_millis(12);
/// The same on the title screen and while a join is still connecting: less,
/// so the menus and the connection's own work (signalling, the join
/// handshake) keep their frames.
const CONNECTING_FRAME_BUDGET: Duration = Duration::from_millis(8);

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


                // The lobby loadout's weapons are what the player spawns with.
                asset_queue.prioritize(spawn_assets(&loadout));
                // Nothing is being played yet, so load as much as fits in a
                // frame instead of one file per frame: the ~20 small sounds
                // and textures alone took a third of a second that way.
                let frame_start = Instant::now();
                while asset_queue.pump(&mut playing_state).await
                    && frame_start.elapsed() < LOADING_FRAME_BUDGET
                    && !asset_queue.ready_to_play()
                {
                    // A game we host answers players between files too, not
                    // only once a frame: a frame of several big models would
                    // otherwise keep someone joining waiting.
                    if let Some(host) = session.as_mut().and_then(|s| s.host.as_mut()) {
                        host.update();
                    }
                }

                if asset_queue.ready_to_play() {
                    if let Some(at) = joined_at.take() {
                        macroquad::miniquad::info!("gridlock: in game {} ms after the join (loading screen)", at.elapsed().as_millis());
                    }
                    playing_state.weapons = WeaponInventory::from_loadout(&loadout);
                    state = GameState::Playing;
                    // Loading paused this loop, so time spent there is not silence from the server.
                    last_server_contact = Some(Instant::now());
                    set_cursor_grab(true);
                    show_mouse(false);
                    playing_state.last_mouse_pos = mouse_position();
                }

                let (loaded, total) = asset_queue.essential_progress();
                let current = asset_queue.current().cloned().unwrap_or_default();
                login_screen.draw_loading(total, loaded, &current.0, download_progress(&current.1));
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

        // In a game: keep loading the rest of the assets, one per frame at
        // most, whatever the player needs next first.
        if
            session.is_some() &&
            !asset_queue.is_done() &&
            !matches!(state, GameState::Login | GameState::Loading | GameState::Exit)
        {
            asset_queue.prioritize(wanted_assets(&playing_state, &loadout));
            asset_queue.pump(&mut playing_state).await;
            let (loaded, total) = asset_queue.progress();
            client::ui::hud::draw_background_loading(loaded, total);
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

/// The models a player needs to spawn with `loadout`: its weapons and their
/// hands, the knife's hands, and the idle animation every other player is
/// drawn with until their own animations arrive.
fn spawn_assets(loadout: &Loadout) -> Vec<&'static str> {
    let mut keys = vec!["fp_hands", "anim_rifle_idle"];
    for id in loadout.slots {
        keys.extend(id.stats().viewmodel_model);
        keys.push(PlayingState::preferred_hands_key_for_weapon(id));
    }
    keys
}

/// What to load next while playing: the weapons the player holds or picked in
/// the lobby, then the animations other players are showing right now.
fn wanted_assets(playing_state: &PlayingState, loadout: &Loadout) -> Vec<&'static str> {
    let held = playing_state.weapons.weapons.iter().map(|w| w.id);
    let mut keys = Vec::new();
    for id in held.chain(loadout.slots) {
        keys.extend(id.stats().viewmodel_model);
        keys.push(PlayingState::preferred_hands_key_for_weapon(id));
    }
    let skins = playing_state.other_players
        .iter()
        .map(|p| p.skin_type)
        .chain(playing_state.practice_bots.iter().map(|b| b.player.skin_type));
    keys.extend(skins.filter_map(|skin| skin.model_key()));
    keys
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
