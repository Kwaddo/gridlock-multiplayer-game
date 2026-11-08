use macroquad::prelude::*;
use client::net::Session;
use transport::Link;
use platform::{Duration, Instant};

use client::core::{ GameConfig, GameState };
use client::screens::{ LoginScreen, PlayingState };
use client::types::maze::Maze;
use client::weapons::loadout::Loadout;
use client::weapons::WeaponInventory;

use super::super::asset_loader::AssetQueue;
use super::super::world_sync::sync_world;
use super::assets::wanted_assets;
use super::constants::CLIENT_CONNECTION_LOST_TIMEOUT;

/// One frame in a hidden browser tab: keep a game we host stepping, take in
/// what the server sent and send keepalives, without simulating or drawing.
#[allow(clippy::too_many_arguments)]
pub(super) async fn hidden_tab_frame(
    session: &mut Option<Session>,
    player_id: u32,
    username: &str,
    playing_state: &mut PlayingState,
    maze: &mut Option<Maze>,
    login_screen: &mut LoginScreen,
    state: &mut GameState,
    last_server_contact: &mut Option<Instant>,
    loadout: &Loadout,
    last_keepalive: &mut Option<Instant>,
    config: &GameConfig
) {
    let sync = sync_world(link_of(session), player_id, username, playing_state, maze);
    if sync.removed_by_server {
        *session = None;
        login_screen.returned_with("Timed out / disconnected by server.");
        *state = GameState::Login;
        *last_server_contact = None;
        return;
    }
    if sync.server_seen {
        *last_server_contact = Some(Instant::now());
    }
    // A match that ended while hidden still hands back the lobby loadout.
    if sync.returned_to_lobby {
        playing_state.weapons = WeaponInventory::from_loadout(loadout);
    }
    if let Some(host) = session.as_mut().and_then(|s| s.host.as_mut()) {
        host.update();
    }
    if last_keepalive.map_or(true, |t| t.elapsed() > Duration::from_secs(4)) {
        playing_state.send_network(link_of(session), config);
        *last_keepalive = Some(Instant::now());
    }
    next_frame().await;
}

/// The work after every visible frame's state handling: background asset
/// loading, stepping a hosted game, keepalives and the connection-lost check.
#[allow(clippy::too_many_arguments)]
pub(super) async fn finish_frame(
    session: &mut Option<Session>,
    asset_queue: &mut AssetQueue,
    state: &mut GameState,
    playing_state: &mut PlayingState,
    loadout: &Loadout,
    last_keepalive: &mut Option<Instant>,
    config: &GameConfig,
    last_server_contact: &mut Option<Instant>,
    login_screen: &mut LoginScreen
) {
    // In a game: keep loading the rest of the assets, one per frame at
    // most, whatever the player needs next first.
    if
        session.is_some() &&
        !asset_queue.is_done() &&
        !matches!(*state, GameState::Login | GameState::Loading | GameState::Exit)
    {
        asset_queue.prioritize(wanted_assets(playing_state, loadout));
        asset_queue.pump(playing_state).await;
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
            playing_state.send_network(link_of(session), config);
            *last_keepalive = Some(Instant::now());
        }
    }

    // Global: if we haven't heard from the server for a while, treat as disconnected.
    // Not while loading: each asset load is awaited inside this loop, and a large
    // model on a first visit to the browser build can take longer than the timeout.
    if session.is_some() && !matches!(*state, GameState::Loading) {
        if let Some(last) = *last_server_contact {
            if Instant::now().duration_since(last) > CLIENT_CONNECTION_LOST_TIMEOUT {
                *session = None;
                login_screen.returned_with("Connection to server lost.");
                *state = GameState::Login;
                set_cursor_grab(false);
                show_mouse(true);
                playing_state.clear_input();
                *last_server_contact = None;
            }
        }
    }
}

/// The live connection, for code that only sends and receives.
pub(super) fn link_of(session: &mut Option<Session>) -> Option<&mut dyn Link> {
    match session {
        Some(s) => Some(s.link.as_mut()),
        None => None,
    }
}
