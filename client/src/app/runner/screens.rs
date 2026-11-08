use macroquad::prelude::*;
use client::net::Session;
use platform::Instant;

use client::core::{ GameConfig, GameState };
use client::screens::dev_mode::draw_dev_overlay;
use client::screens::{ LobbyInventoryScreen, LoginScreen, PauseAction, PauseMenu, PlayingState };
use client::settings::SettingsScreen;
use client::weapons::crosshair_presets::CrosshairPresetManager;
use client::weapons::loadout::Loadout;
use client::weapons::WeaponInventory;

use super::super::asset_loader::{ download_progress, AssetQueue };
use super::super::dev_ui::draw_bot_tab;
use super::assets::spawn_assets;
use super::constants::LOADING_FRAME_BUDGET;

/// The loading screen after a join: load what spawning needs, then start
/// playing once it is ready.
#[allow(clippy::too_many_arguments)]
pub(super) async fn loading_frame(
    asset_queue: &mut AssetQueue,
    loadout: &Loadout,
    playing_state: &mut PlayingState,
    session: &mut Option<Session>,
    joined_at: &mut Option<Instant>,
    state: &mut GameState,
    last_server_contact: &mut Option<Instant>,
    login_screen: &mut LoginScreen
) {
    // The lobby loadout's weapons are what the player spawns with.
    asset_queue.prioritize(spawn_assets(loadout));
    // Nothing is being played yet, so load as much as fits in a
    // frame instead of one file per frame: the ~20 small sounds
    // and textures alone took a third of a second that way.
    let frame_start = Instant::now();
    while asset_queue.pump(playing_state).await
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
        playing_state.weapons = WeaponInventory::from_loadout(loadout);
        *state = GameState::Playing;
        // Loading paused this loop, so time spent there is not silence from the server.
        *last_server_contact = Some(Instant::now());
        set_cursor_grab(true);
        show_mouse(false);
        playing_state.last_mouse_pos = mouse_position();
    }

    let (loaded, total) = asset_queue.essential_progress();
    let current = asset_queue.current().cloned().unwrap_or_default();
    login_screen.draw_loading(total, loaded, &current.0, download_progress(&current.1));
}

/// The pause menu, and what its chosen action does.
pub(super) fn pause_menu_frame(
    pause_menu: &mut PauseMenu,
    config: &GameConfig,
    settings_screen: &mut SettingsScreen,
    playing_state: &mut PlayingState,
    player_id: u32,
    state: &mut GameState,
    lobby_inventory: &mut LobbyInventoryScreen
) {
    let in_match = playing_state.current_world == client::types::world::WorldId::Maze;
    let is_host = playing_state.is_host(player_id);
    if
        let Some(action) = pause_menu.draw(
            config,
            settings_screen,
            &mut playing_state.dev_mode,
            in_match,
            is_host
        )
    {
        // Ending or leaving the match keeps the connection: the
        // server moves us back to the lobby and play goes on there.
        *state = match action {
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
        if matches!(*state, GameState::LobbyInventory) {
            *lobby_inventory = LobbyInventoryScreen::new();
        }
        if matches!(*state, GameState::Playing) {
            set_cursor_grab(true);
            show_mouse(false);
            playing_state.last_mouse_pos = mouse_position();
        }
    }
}

/// The settings screen drawn over the game.
pub(super) fn settings_frame(
    config: &mut GameConfig,
    settings_screen: &mut SettingsScreen,
    playing_state: &mut PlayingState,
    preset_manager: &mut CrosshairPresetManager,
    state: &mut GameState
) {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::from_rgba(0, 0, 0, 180)
    );

    settings_screen.draw(
        config,
        &playing_state.weapons,
        preset_manager,
        &playing_state.assets
    );
    if
        settings_screen.update(
            config,
            &mut playing_state.weapons,
            preset_manager
        )
    {
        *state = GameState::Paused;
    }
}

/// The dev mode overlay, with its bot tab.
pub(super) fn dev_overlay_frame(playing_state: &mut PlayingState, config: &mut GameConfig) {
    draw_dev_overlay(
        &mut playing_state.dev_mode,
        &mut playing_state.weapons,
        &mut playing_state.movement_config,
        config,
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
