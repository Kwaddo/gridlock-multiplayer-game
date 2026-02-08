// MESSAGE TYPES

// Message types - first byte of every packet
pub const MSG_JOIN: u8 = 1; // client -> server: username
pub const MSG_WELCOME: u8 = 2; // server -> client: player id, username
pub const MSG_POSITION: u8 = 3; // client -> server: x, y, yaw
pub const MSG_STATE: u8 = 4; // server -> client: all player positions
pub const MSG_SHOOT: u8 = 5; // client -> server: shot fired
pub const MSG_HIT: u8 = 6; // server -> client: you got hit
pub const MSG_DISCONNECT: u8 = 7; // client -> server: leaving
pub const MSG_MAZE: u8 = 8; // server -> client: maze layout
pub const MSG_WORLD_CHANGE: u8 = 9; // server -> client: bi-directional world transition
pub const MSG_ASSETS: u8 = 10; // server -> client: asset manifest per world

