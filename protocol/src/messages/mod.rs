//! Typed messages. `encode` produces a full packet (`[msg_type][payload]`);
//! `decode` takes a full packet and returns `None` for unknown or truncated ones.
//! `Option` fields are optional trailing bytes: they must stay at the end and are
//! only written when `Some`.

mod client_msg;
mod server_msg;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod wire_tests;
mod types;

pub use client_msg::ClientMsg;
pub use server_msg::ServerMsg;
pub use types::{
    BotShot, LobbySettings, MazeLayout, MazeLight, MazeProp, MazeRoom, PlayerInfo, PlayerListEntry,
    PlayerState, SndRound,
};
