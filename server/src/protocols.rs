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
pub const MSG_DISCOVER: u8 = 11; // client -> server: LAN discovery broadcast
pub const MSG_WALL_BROKEN: u8 = 12; // client <-> server: breakable wall destroyed (x, y)
pub const MSG_DEATH: u8 = 13; // server -> client: you died (killer_id, weapon)
pub const MSG_RESPAWN_REQUEST: u8 = 14; // client -> server: i want to respawn
pub const MSG_RESPAWN: u8 = 15; // server -> client: respawn at position (x, y, z) + optional spawn-kind byte (0=random,1=teleporter-fallback)
pub const MSG_DAMAGE: u8 = 16; // server -> client: you took damage (amount, attacker_id)
pub const MSG_KILL: u8 = 17; // server -> client: you killed someone (victim_id, weapon)
pub const MSG_KILL_FEED: u8 = 18; // server -> all: broadcast kill (killer_id, victim_id)
pub const MSG_START_GAME: u8 = 19; // client -> server: request to start a match
pub const MSG_START_DENIED: u8 = 20; // server -> client: start request rejected (match running / full)
pub const MSG_MATCH_JOINED: u8 = 21; // server -> all: announce player joined a running match (id, name)

// State flags for player movement and actions, sent in the position message
pub const FLAG_SPRINTING: u8 = 1 << 0;
pub const FLAG_CROUCHING: u8 = 1 << 1;
pub const FLAG_ADS: u8 = 1 << 2;
pub const FLAG_AIRBORNE: u8 = 1 << 3;
pub const FLAG_PRONE: u8 = 1 << 4;
pub const FLAG_SLIDING: u8 = 1 << 5;

// Event flags for the state message
pub const EVT_JUMP_START: u8 = 1 << 0;
pub const EVT_LANDED: u8 = 1 << 1;
pub const EVT_SLIDE_START: u8 = 1 << 2;
pub const EVT_SLIDE_END: u8 = 1 << 3;
