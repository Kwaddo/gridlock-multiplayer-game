//! Authoritative game server as a transport-agnostic library.
//!
//! Hosts (the native UDP binary, or a browser tab over WebRTC) feed packets in
//! with [`Server::handle_packet`], call [`Server::tick`] regularly, and send
//! whatever [`Server::drain_outbox`] returns.

#[macro_use]
mod log;
mod bot;
mod math;
mod maze;
mod net;
mod physics;
mod player;
mod protocols;
mod server;
mod weapons_stats;

#[cfg(not(target_arch = "wasm32"))]
pub mod udp_host;

#[doc(hidden)]
pub use platform;

pub use net::{PeerId, BOT_PEER_BASE};
pub use server::Server;
pub use server::game::MATCH_MAX_PLAYERS as MAX_PLAYERS;

/// Manifest bundled at compile time, for hosts without a filesystem.
pub const EMBEDDED_MANIFEST: &str = include_str!("../assets/manifest.json");
