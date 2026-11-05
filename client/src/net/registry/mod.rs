//! Client for the online host list (see `registry/` in the repo).
//!
//! Same API on both builds, all non-blocking: [`ListFetch`] for browsing and
//! [`Listing`] for a host's heartbeat. Native builds use background threads;
//! the browser build uses fetch() and is advanced by [`Listing::tick`].

use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::{json, Value};

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::{ListFetch, Listing};

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::{ListFetch, Listing};

/// Well inside the registry's 90 s listing TTL.
const HEARTBEAT_SECS: u64 = 30;
/// Retry sooner while the game is not listed yet.
const RETRY_SECS: u64 = 5;

/// Base URL of the host list, from `GRIDLOCK_REGISTRY` (at run time on native,
/// baked in at build time on both). `None` turns online play off.
pub fn registry_url() -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    let runtime = std::env::var("GRIDLOCK_REGISTRY").ok();
    #[cfg(target_arch = "wasm32")]
    let runtime: Option<String> = None;

    runtime
        .or_else(|| option_env!("GRIDLOCK_REGISTRY").map(str::to_string))
        .map(|url| url.trim_end_matches('/').to_string())
        .filter(|url| !url.is_empty())
}

/// The WebRTC signalling URL for a room on the registry's Worker.
pub fn room_url(base: &str, room: &str) -> String {
    let ws = base.replacen("https://", "wss://", 1).replacen("http://", "ws://", 1);
    format!("{ws}/room/{room}")
}

/// One public game. Native hosts have a UDP `addr`; browser hosts have a
/// WebRTC signalling `room` instead.
#[derive(Deserialize)]
pub struct Listed {
    pub name: String,
    pub addr: Option<String>,
    pub room: Option<String>,
    pub players: u8,
    pub max_players: u8,
}

/// How players reach the listed game.
pub enum Endpoint {
    /// UDP port on the host's public address. Polled until it returns a port,
    /// so the listing can wait for the router to say which one it opened.
    Udp(Box<dyn Fn() -> Option<u16> + Send>),
    /// WebRTC signalling room code.
    Room(String),
}

pub struct Announce {
    pub name: String,
    pub max_players: u8,
    pub endpoint: Endpoint,
    pub players: Arc<AtomicUsize>,
}

#[derive(Deserialize)]
struct Created {
    id: String,
    token: String,
    #[serde(default)]
    addr: Option<String>,
}

/// The POST body announcing a game, once its endpoint is known.
fn create_body(announce: &Announce, players: usize) -> Option<Value> {
    let endpoint = match &announce.endpoint {
        Endpoint::Room(room) => json!({ "room": room }),
        Endpoint::Udp(port) => json!({ "port": port()? }),
    };
    let mut body = json!({
        "name": announce.name,
        "players": players.min(255),
        "max_players": announce.max_players,
    });
    if let (Value::Object(body), Value::Object(endpoint)) = (&mut body, endpoint) {
        body.extend(endpoint);
    }
    Some(body)
}

fn heartbeat_body(players: usize) -> Value {
    json!({ "players": players.min(255) })
}
