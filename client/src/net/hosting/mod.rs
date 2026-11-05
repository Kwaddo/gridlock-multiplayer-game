//! Hosting a game from the client. Both builds offer the same [`LocalHost`]:
//!
//! - native: the server runs on a background thread behind UDP; the hosting
//!   player joins over loopback, and listing online also opens the router port.
//! - browser: the server runs inside the tab, driven by [`LocalHost::update`];
//!   players connect over WebRTC through a signalling room, and the hosting
//!   player uses an in-memory link.
//!
//! Dropping a `LocalHost` ends the game for everyone.

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::LocalHost;

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::LocalHost;
