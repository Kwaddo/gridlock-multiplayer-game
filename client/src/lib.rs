pub mod audio;
pub mod core;
pub mod net;
pub mod physics;
pub mod screens;
pub mod settings;
pub mod types;
pub mod ui;
pub mod weapons;
pub mod world;

// Re-export commonly used items
pub use core::{config, game_state};
pub use net::protocols;
pub use ui::{fonts, hud};
pub use world::lobby;
